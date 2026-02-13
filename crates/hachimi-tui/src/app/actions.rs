use crate::api::endpoints::{RecentQuery, SongSearchQuery};
use crate::model::auth::LoginReq;
use crate::model::queue::MusicQueueItem;
use crate::model::song::PublicSongDetail;
use crate::ui::login::{LoginState, LoginStep};
use crate::ui::navigation::NavNode;

use super::{App, AppMessage, DataPayload, InputMode};

impl App {
    // — 认证 —

    /// 第一步：校验输入 → 异步生成 captcha
    pub(crate) fn start_captcha(&mut self) {
        let email = self.login.email.trim().to_string();
        let password = self.login.password.clone();

        if email.is_empty() || password.is_empty() {
            self.login.error = Some(t!("app.email_password_required").to_string());
            return;
        }

        self.login.step = LoginStep::GeneratingCaptcha;
        self.login.error = None;

        let tx = self.msg_tx.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let result = client.generate_captcha().await;
            let _ = tx.send(AppMessage::CaptchaGenerated(
                result
                    .map(|resp| (resp.captcha_key, resp.url))
                    .map_err(|e| e.to_string()),
            ));
        });
    }

    /// 第二步：captcha 已完成，提交登录
    pub(crate) fn submit_login(&mut self) {
        let Some(captcha_key) = self.login.captcha_key.clone() else {
            self.login.error = Some(t!("app.no_captcha_key").to_string());
            self.login.step = LoginStep::Input;
            return;
        };

        let email = self.login.email.trim().to_string();
        let password = self.login.password.clone();

        self.login.step = LoginStep::Submitting;
        self.login.error = None;

        let tx = self.msg_tx.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let req = LoginReq {
                email,
                password,
                code: None,
                device_info: "hachimi-tui".to_string(),
                captcha_key,
            };

            let result = client.login(&req).await;
            let _ = tx.send(AppMessage::LoginResult(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    pub(crate) fn logout(&mut self) {
        let _ = crate::config::auth_store::clear();

        let client = self.client.clone();
        tokio::spawn(async move {
            client.clear_auth().await;
        });

        self.username = None;
        self.cache.songs.clear();
        self.cache.loading.clear();
        self.login = LoginState::new();
        self.input_mode = InputMode::Login;
    }

    // — 数据加载 —

    pub(crate) fn load_node_data(&mut self, node: &NavNode) {
        // Categories 用 tag_cache 而非 song_cache
        if *node == NavNode::Categories {
            if self.cache.loading.contains(node) || !self.cache.tags.is_empty() {
                return;
            }
        } else if self.cache.loading.contains(node) || self.cache.songs.contains_key(node) {
            return;
        }
        self.cache.loading.insert(node.clone());
        let node_owned = node.clone();
        let tx = self.msg_tx.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            // Categories 走单独的 tag 加载流程
            if node_owned == NavNode::Categories {
                let result = if client.is_authenticated().await {
                    client.recommend_tags().await
                } else {
                    client.recommend_tags_anonymous().await
                };
                match result {
                    Ok(resp) => {
                        let names: Vec<String> = resp.result.into_iter().map(|t| t.name).collect();
                        let _ = tx.send(AppMessage::DataLoaded(DataPayload::Tags(names)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::DataLoaded(DataPayload::Tags(vec![])));
                        let _ = tx.send(AppMessage::Error(e.to_string()));
                    }
                }
                return;
            }

            let result = match &node_owned {
                NavNode::LatestReleases => client
                    .recent_songs(&RecentQuery {
                        cursor: None,
                        limit: 30,
                        after: None,
                    })
                    .await
                    .map(|r| r.songs),
                NavNode::DailyRecommend => {
                    let resp = if client.is_authenticated().await {
                        client.recommend_songs().await
                    } else {
                        client.recommend_songs_anonymous().await
                    };
                    resp.map(|r| r.songs)
                }
                NavNode::WeeklyHot => {
                    client.hot_songs_weekly().await.map(|r| r.songs)
                }
                NavNode::Tag { name } => {
                    client
                        .search_songs(&SongSearchQuery {
                            q: String::new(),
                            limit: Some(30),
                            offset: None,
                            filter: Some(format!("tags = \"{}\"", name)),
                            sort_by: Some("release_time_desc".to_string()),
                        })
                        .await
                        .map(|r| r.hits.into_iter().map(|s| s.into_song_detail()).collect())
                }
                _ => return,
            };

            match result {
                Ok(songs) => {
                    let _ = tx.send(AppMessage::DataLoaded(DataPayload::Songs(
                        node_owned, songs,
                    )));
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::DataLoaded(DataPayload::Songs(
                        node_owned,
                        vec![],
                    )));
                    let _ = tx.send(AppMessage::Error(e.to_string()));
                }
            }
        });
    }

    pub(crate) fn maybe_load_preview_data(&mut self) {
        let node = self.nav.current().node.clone();
        let sel = self.nav.current().selected;
        if node.has_static_children() {
            let children = node.children();
            if let Some(child) = children.get(sel) {
                if child.needs_dynamic_data() {
                    let child = child.clone();
                    self.load_node_data(&child);
                }
            }
        } else if node == NavNode::Categories {
            // 加载选中标签的歌曲预览
            if let Some(tag_name) = self.cache.tags.get(sel).cloned() {
                let tag_node = NavNode::Tag { name: tag_name };
                self.load_node_data(&tag_node);
            }
        }
    }

    // — 播放控制 —

    pub(crate) fn toggle_play_pause(&mut self) {
        if self.player.bar.is_playing {
            self.player.engine.pause();
        } else if self.player.bar.has_song() {
            self.player.engine.resume();
        } else if let Some(song) = self.queue.current_song().cloned() {
            self.start_audio_fetch(song.id, &song.name, &song.artist);
        }
    }

    pub(crate) fn play_next(&mut self) {
        let mode = self.settings.player.default_play_mode.clone();
        if let Some(item) = self.queue.next_with_mode(&mode).cloned() {
            self.start_audio_fetch(item.id, &item.name, &item.artist);
        }
    }

    pub(crate) fn play_prev(&mut self) {
        let mode = self.settings.player.default_play_mode.clone();
        if let Some(item) = self.queue.prev_with_mode(&mode).cloned() {
            self.start_audio_fetch(item.id, &item.name, &item.artist);
        }
    }

    /// 获取当前 Miller Columns 选中的歌曲
    pub(crate) fn selected_song(&self) -> Option<&PublicSongDetail> {
        let node = &self.nav.current().node;
        let sel = self.nav.current().selected;
        if !node.has_static_children() {
            self.cache.songs.get(node).and_then(|songs| songs.get(sel))
        } else {
            None
        }
    }

    pub(crate) fn song_to_queue_item(song: &PublicSongDetail) -> MusicQueueItem {
        MusicQueueItem {
            id: song.id,
            display_id: song.display_id.clone(),
            name: song.title.clone(),
            artist: song.uploader_name.clone(),
            duration_secs: song.duration_seconds,
            cover_url: song.cover_url.clone(),
            explicit: song.explicit,
            audio_url: song.audio_url.clone(),
            gain: song.gain,
        }
    }

    pub(crate) fn add_selected_to_queue(&mut self) {
        if let Some(song) = self.selected_song().cloned() {
            let item = Self::song_to_queue_item(&song);
            self.queue.add(item);
        }
    }

    /// 异步获取歌曲详情 → 下载音频 → 发送 AudioFetched
    pub(crate) fn start_audio_fetch(&mut self, song_id: i64, title: &str, artist: &str) {
        self.player.bar.is_loading = true;
        self.player.bar.title = title.to_string();
        self.player.bar.artist = artist.to_string();

        let tx = self.msg_tx.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            // 第一步：获取歌曲详情拿到 audio_url
            let detail = match client.song_detail_by_id(song_id).await {
                Ok(d) => d,
                Err(e) => {
                    let _ = tx.send(AppMessage::AudioFetchError(
                        format!("获取歌曲详情失败: {e}"),
                    ));
                    return;
                }
            };

            if detail.audio_url.is_empty() {
                let _ = tx.send(AppMessage::AudioFetchError(
                    "歌曲无音频地址".to_string(),
                ));
                return;
            }

            // 第二步：下载音频数据
            let audio_url = &detail.audio_url;
            match client.get_audio_stream(audio_url).await {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        let body = resp.text().await.unwrap_or_default();
                        let _ = tx.send(AppMessage::AudioFetchError(
                            format!("音频请求返回 {status}: {body}"),
                        ));
                        return;
                    }

                    match resp.bytes().await {
                        Ok(bytes) => {
                            if bytes.is_empty() {
                                let _ = tx.send(AppMessage::AudioFetchError(
                                    "音频数据为空".to_string(),
                                ));
                                return;
                            }
                            let _ = tx.send(AppMessage::AudioFetched {
                                detail,
                                data: bytes.to_vec(),
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(AppMessage::AudioFetchError(
                                format!("下载音频失败: {e}"),
                            ));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::AudioFetchError(
                        format!("请求音频失败: {e}"),
                    ));
                }
            }
        });
    }

    // — Miller Columns 导航 —

    pub(crate) fn current_list_len(&self) -> usize {
        let node = &self.nav.current().node;
        if *node == NavNode::Settings {
            crate::ui::settings_view::ITEMS_COUNT
        } else if node.has_static_children() {
            node.children().len()
        } else if *node == NavNode::Categories {
            self.cache.tags.len()
        } else if let Some(songs) = self.cache.songs.get(node) {
            songs.len()
        } else {
            0
        }
    }

    pub(crate) fn nav_down(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            let sel = self.nav.current().selected;
            if sel + 1 < len {
                self.nav.current_mut().selected = sel + 1;
                self.scroll_tick = 0;
            }
        }
        self.maybe_load_preview_data();
        self.maybe_fetch_song_detail();
        self.maybe_load_cover_image();
    }

    pub(crate) fn nav_up(&mut self) {
        let sel = self.nav.current().selected;
        if sel > 0 {
            self.nav.current_mut().selected = sel - 1;
            self.scroll_tick = 0;
        }
        self.maybe_load_preview_data();
        self.maybe_fetch_song_detail();
        self.maybe_load_cover_image();
    }

    pub(crate) fn nav_drill_in(&mut self) {
        let node = self.nav.current().node.clone();
        let sel = self.nav.current().selected;
        if node == NavNode::Settings {
            crate::ui::settings_view::cycle_setting(&mut self.settings, sel);
            // 播放模式已直接修改 settings，无需同步
            let _ = self.settings.save();
            return;
        }
        if node.has_static_children() {
            let children = node.children();
            if sel < children.len() {
                let child = children[sel].clone();
                if child.needs_dynamic_data() {
                    self.load_node_data(&child);
                }
                self.nav.push(child);
                self.scroll_tick = 0;
                self.maybe_load_preview_data();
                self.maybe_load_cover_image();
            }
        } else if node == NavNode::Categories {
            // 进入选中的标签
            if let Some(tag_name) = self.cache.tags.get(sel).cloned() {
                let tag_node = NavNode::Tag { name: tag_name };
                self.load_node_data(&tag_node);
                self.nav.push(tag_node);
                self.scroll_tick = 0;
                self.maybe_load_preview_data();
                self.maybe_load_cover_image();
            }
        } else {
            // 当前节点是歌曲列表，按 Enter 播放选中歌曲
            if let Some(songs) = self.cache.songs.get(&node).cloned() {
                if sel < songs.len() {
                    // 替换队列为当前列表所有歌曲
                    self.queue.clear();
                    for song in &songs {
                        self.queue.add(Self::song_to_queue_item(song));
                    }
                    self.queue.current_index = Some(sel);

                    // 播放选中歌曲
                    let song = &songs[sel];
                    self.start_audio_fetch(
                        song.id, &song.title, &song.uploader_name,
                    );
                }
            }
        }
    }

    pub(crate) fn nav_drill_out(&mut self) {
        self.nav.pop();
        self.scroll_tick = 0;
        self.maybe_load_preview_data();
        self.maybe_fetch_song_detail();
        self.maybe_load_cover_image();
    }

    pub(crate) fn nav_top(&mut self) {
        self.nav.current_mut().selected = 0;
        self.scroll_tick = 0;
        self.maybe_load_preview_data();
        self.maybe_fetch_song_detail();
        self.maybe_load_cover_image();
    }

    pub(crate) fn nav_bottom(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            self.nav.current_mut().selected = len - 1;
            self.scroll_tick = 0;
        }
        self.maybe_load_preview_data();
        self.maybe_fetch_song_detail();
        self.maybe_load_cover_image();
    }

    // — 图片加载 —

    pub(crate) fn start_image_fetch(&mut self, url: &str) {
        if url.is_empty() || self.cache.images.contains_key(url) || self.cache.images_loading.contains(url) {
            return;
        }
        let Some(ref mut picker) = self.cache.picker else { return };
        // Picker 是 Copy，直接复制会导致 kitty_counter 不递增，
        // 所有后台创建的协议拿到相同 ID → 终端无法区分不同图片。
        // 先用 1×1 空图推进计数器，再复制给后台线程。
        let _ = picker.new_resize_protocol(image::DynamicImage::new_rgb8(1, 1));
        let picker = *picker;
        self.cache.images_loading.insert(url.to_string());
        let tx = self.msg_tx.clone();
        let client = self.client.clone();
        let url = url.to_string();
        let hint_rect = self.cache.last_image_rect;

        tokio::spawn(async move {
            let resp = match client.get_audio_stream(&url).await {
                Ok(r) if r.status().is_success() => r,
                _ => return,
            };
            let bytes = match resp.bytes().await {
                Ok(b) => b,
                _ => return,
            };
            // 解码 + 裁剪 + resize + protocol 编码全部在 blocking 线程
            let data = bytes.to_vec();
            let result = tokio::task::spawn_blocking(move || {
                let img = image::load_from_memory(&data).ok()?;
                let min_side = img.width().min(img.height());
                let x = (img.width() - min_side) / 2;
                let y = (img.height() - min_side) / 2;
                let img = img.crop_imm(x, y, min_side, min_side);

                let (fw, fh) = picker.font_size();
                let g = crate::ui::util::gcd(fw, fh);
                let lcm = (fw / g) as u32 * fh as u32;
                let n = (800u32 / lcm).max(1);
                let target = n * lcm;
                let img = img.resize_exact(target, target, image::imageops::FilterType::Triangle);
                let mut picker = picker;
                let mut protocol = picker.new_resize_protocol(img);

                // 用上一次渲染的 widget 区域预编码终端协议数据，
                // 使 draw 时 needs_resize 返回 None，避免主线程阻塞
                if hint_rect.width > 0 && hint_rect.height > 0 {
                    let resize = ratatui_image::Resize::Fit(None);
                    if let Some(rect) = protocol.needs_resize(&resize, hint_rect) {
                        protocol.resize_encode(&resize, None, rect);
                    }
                }

                Some(protocol)
            })
            .await;
            if let Ok(Some(protocol)) = result {
                let _ = tx.send(AppMessage::ImageFetched { url, protocol });
            }
        });
    }

    /// 预览选中歌曲时，若为搜索结果（partial），异步补全完整详情
    pub(crate) fn maybe_fetch_song_detail(&mut self) {
        let node = self.nav.current().node.clone();
        let sel = self.nav.current().selected;

        // 仅对歌曲列表节点生效
        if node.has_static_children() || node == NavNode::Categories || node == NavNode::Settings {
            return;
        }

        if let Some(song) = self.cache.songs.get(&node).and_then(|songs| songs.get(sel)) {
            if !song.partial || self.cache.detail_loading.contains(&song.id) {
                return;
            }
            let song_id = song.id;
            self.cache.detail_loading.insert(song_id);
            let tx = self.msg_tx.clone();
            let client = self.client.clone();
            let node = node.clone();

            tokio::spawn(async move {
                if let Ok(detail) = client.song_detail_by_id(song_id).await {
                    let _ = tx.send(AppMessage::SongDetailFetched { node, index: sel, detail });
                }
            });
        }
    }

    /// 防抖加载预览封面：取消旧定时器，停下 150ms 后才真正发起请求
    pub(crate) fn maybe_load_cover_image(&mut self) {
        // 取消上一次未触发的防抖
        if let Some(h) = self.cover_debounce.take() {
            h.abort();
        }
        if let Some(song) = self.selected_song().cloned() {
            let url = &song.cover_url;
            if !url.is_empty()
                && !self.cache.images.contains_key(url)
                && !self.cache.images_loading.contains(url)
            {
                let tx = self.msg_tx.clone();
                let url = url.clone();
                self.cover_debounce = Some(tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                    let _ = tx.send(AppMessage::DebouncedCoverLoad(url));
                }));
            }
        }
    }
}
