use crate::ui::navigation::{NavNode, SearchType};

use super::super::{App, AppMessage};

impl App {
    // — 封面图片 —

    /// 返回当前导航选中项对应的封面 URL（用于触发封面加载）
    pub(crate) fn current_preview_cover_url(&self) -> Option<String> {
        let node = &self.nav.current().node;
        let sel = self.nav.current().selected;

        match node {
            NavNode::Queue => {
                let item = self.queue.songs.get(sel)?;
                if let Some(detail) = self.cache.queue_song_detail.get(&item.id) {
                    Some(detail.cover_url.clone())
                } else {
                    Some(item.cover_url.clone())
                }
            }
            NavNode::SearchResults => match self.search.search_type {
                SearchType::Song => {
                    let song = self.cache.songs.get(node)?.get(sel)?;
                    Some(song.cover_url.clone())
                }
                SearchType::User => {
                    let user = self.cache.search_users.get(sel)?;
                    user.avatar_url.clone()
                }
                SearchType::Playlist => {
                    let pl = self.cache.search_playlists.get(sel)?;
                    pl.cover_url.clone()
                }
            },
            NavNode::MyPlaylists => {
                let pl = self.cache.playlists.as_ref()?.get(sel)?;
                pl.cover_url.clone()
            }
            node if !node.has_static_children() => {
                let song = self.cache.songs.get(node)?.get(sel)?;
                Some(song.cover_url.clone())
            }
            _ => None,
        }
    }

    /// 记录待加载封面（防抖：实际加载在 PlayerTick 中延迟触发）
    /// 若封面已就绪或正在下载则跳过。
    pub(crate) fn schedule_cover_load(&mut self) {
        if !self.cover.kitty_supported {
            return;
        }
        // 展开页跟随播放时，优先取正在播放歌曲的封面
        let url = if self.player.expanded && self.player.follow_playback {
            self.player.current_detail.as_ref().map(|d| d.cover_url.clone())
                .or_else(|| self.current_preview_cover_url())
        } else {
            self.current_preview_cover_url()
        };
        if let Some(url) = url {
            if !url.is_empty()
                && !self.cache.covers.is_ready(&url)
                && !self.cache.covers.is_loading(&url)
            {
                self.cover.pending_cover_load = Some((url, std::time::Instant::now()));
            }
        }
    }

    /// 异步下载并上传封面到终端（Kitty 图形协议）
    pub(crate) fn maybe_load_cover(&mut self, url: String) {
        if !self.cover.kitty_supported {
            return;
        }
        if self.cache.covers.is_ready(&url) || self.cache.covers.is_loading(&url) {
            return;
        }

        // 超过 10 张时淘汰最旧的一张
        if self.cache.covers.len() >= 10 {
            if let Some((_, old_id)) = self.cache.covers.evict_one() {
                use std::io::Write;
                let seq = crate::ui::kitty::delete_image(old_id);
                let _ = std::io::stdout().write_all(&seq);
                let _ = std::io::stdout().flush();
            }
        }

        let id = self.cache.covers.alloc_id();
        self.cache.covers.mark_loading(url.clone());

        let tx = self.msg_tx.clone();
        let url_clone = url.clone();

        tokio::spawn(async move {
            let bytes = match reqwest::get(&url_clone).await {
                Ok(resp) => match resp.bytes().await {
                    Ok(b) => b.to_vec(),
                    Err(_) => return,
                },
                Err(_) => return,
            };

            let result = tokio::task::spawn_blocking(move || {
                let img = image::load_from_memory(&bytes).ok()?;
                let img = img.resize(800, 800, image::imageops::FilterType::Lanczos3);
                let rgb = img.to_rgb8();
                let (w, h) = rgb.dimensions();
                let raw_pixels = rgb.into_raw();
                // 只上传，不创建 placement（placement 在每帧 draw 后由主循环负责）
                let seq = crate::ui::kitty::upload_rgb(id, &raw_pixels, w, h);
                Some(seq)
            })
            .await;

            if let Ok(Some(upload_seq)) = result {
                let _ = tx.send(AppMessage::CoverReady { url: url_clone, id, upload_seq });
            }
        });
    }

    /// 下载当前选中歌曲的 B 站弹幕并保存为 XML
    pub(crate) fn fetch_danmaku(&mut self) {
        let song = if self.player.expanded {
            let node = self.nav.current().node.clone();
            let sel = self.nav.current().selected;
            let browsed = if !node.has_static_children() {
                self.cache.songs.get(&node).and_then(|s| s.get(sel)).cloned()
            } else {
                None
            };
            if self.player.follow_playback {
                self.player.current_detail.clone().or(browsed)
            } else {
                browsed.or_else(|| self.player.current_detail.clone())
            }
        } else {
            self.selected_song().cloned()
        };

        let Some(song) = song else {
            self.ui.logs.push(crate::ui::log_view::LogLevel::Warn, "无选中歌曲".to_string());
            return;
        };

        let bili_link = song.external_links.iter()
            .find(|l| l.platform.to_ascii_lowercase().contains("bilibili"))
            .map(|l| l.url.clone());

        let Some(url) = bili_link else {
            self.ui.logs.push(crate::ui::log_view::LogLevel::Warn,
                format!("「{}」无 Bilibili 外链", song.title));
            return;
        };

        let Some(bvid) = extract_bvid(&url) else {
            self.ui.logs.push(crate::ui::log_view::LogLevel::Warn,
                format!("无法从链接提取 BV 号：{url}"));
            return;
        };

        let title = song.title.clone();
        let display_id = song.display_id.clone();
        let tx = self.msg_tx.clone();

        tokio::spawn(async move {
            match do_fetch_danmaku(bvid, title.clone(), display_id).await {
                Ok(path) => {
                    let _ = tx.send(AppMessage::DanmakuFetched { title, path });
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::Error(format!("弹幕下载失败：{e}")));
                }
            }
        });
    }
}

fn extract_bvid(url: &str) -> Option<String> {
    let idx = url.find("/BV")?;
    let rest = &url[idx + 1..];
    let end = rest.find('/').unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

async fn do_fetch_danmaku(
    bvid: String,
    title: String,
    display_id: String,
) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()?;

    // 获取 cid
    let view_url = format!("https://api.bilibili.com/x/web-interface/view?bvid={bvid}");
    let resp: serde_json::Value = client.get(&view_url).send().await?.json().await?;
    let cid = resp["data"]["cid"].as_i64()
        .ok_or_else(|| anyhow::anyhow!("无法获取 cid，响应：{resp}"))?;

    // 下载弹幕 XML（B站返回裸 deflate）
    let dm_url = format!("https://comment.bilibili.com/{cid}.xml");
    let compressed = client.get(&dm_url).send().await?.bytes().await?;
    let xml_bytes: Vec<u8> = {
        use std::io::Read;
        let mut decoder = flate2::read::DeflateDecoder::new(compressed.as_ref());
        let mut out = Vec::new();
        decoder.read_to_end(&mut out)
            .map_err(|e| anyhow::anyhow!("deflate 解压失败：{e}"))?;
        out
    };

    // 写入文件
    let safe_title: String = title.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect();
    let filename = format!("{display_id}_{bvid}_{safe_title}.xml");
    let dir = crate::config::paths::danmaku_dir()?;
    let path = dir.join(&filename);
    std::fs::write(&path, &xml_bytes)?;

    Ok(path.to_string_lossy().into_owned())
}
