use crate::model::queue::MusicQueueItem;
use crate::model::song::PublicSongDetail;
use crate::ui::navigation::{NavNode, SearchType};

use super::super::{App, AppMessage};

impl App {
    // — 播放控制 —

    pub(crate) fn toggle_play_pause(&mut self) {
        if self.player.bar.is_playing {
            self.player.engine.pause();
        } else if self.resume_position_ms.is_some() {
            // 恢复模式：音频尚未加载，需先获取
            self.resume_playback();
        } else if self.player.bar.has_song() {
            self.player.engine.resume();
        } else if let Some(song) = self.queue.current_song().cloned() {
            self.start_audio_fetch(song.id, &song.name, &song.artist);
        }
    }

    pub(crate) fn play_next(&mut self) {
        let mode = self.settings.player.default_play_mode.clone();
        if let Some(item) = self.queue.next_with_mode(&mode).cloned() {
            self.player.follow_playback = true;
            self.start_audio_fetch(item.id, &item.name, &item.artist);
        }
    }

    pub(crate) fn play_prev(&mut self) {
        let mode = self.settings.player.default_play_mode.clone();
        if let Some(item) = self.queue.prev_with_mode(&mode).cloned() {
            self.player.follow_playback = true;
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

    /// 替换队列为歌曲列表并播放指定索引
    pub(crate) fn play_from_list(&mut self, songs: &[PublicSongDetail], index: usize) {
        self.queue.clear();
        for song in songs {
            self.queue.add(Self::song_to_queue_item(song));
        }
        self.queue.current_index = Some(index);
        self.player.follow_playback = true;
        let song = &songs[index];
        self.start_audio_fetch(song.id, &song.title, &song.uploader_name);
    }

    /// 播放展开页当前显示的歌曲（如果不是正在播放的那首）
    pub(crate) fn play_expanded_song(&mut self) {
        // 复现 render_player_view 中确定展示歌曲的逻辑
        let node = self.nav.current().node.clone();
        let sel = self.nav.current().selected;

        let browsed_detail = if node == NavNode::Queue {
            self.queue.songs.get(sel).map(|item| {
                self.cache.queue_song_detail.get(&item.id).cloned()
                    .unwrap_or_else(|| item.to_song_detail())
            })
        } else if node == NavNode::SearchResults {
            match self.search.search_type {
                SearchType::Song => {
                    self.cache.songs.get(&node).and_then(|s| s.get(sel)).cloned()
                }
                _ => None,
            }
        } else if !node.has_static_children() && node != NavNode::Settings {
            self.cache.songs.get(&node).and_then(|s| s.get(sel)).cloned()
        } else {
            None
        };

        let detail = if self.player.follow_playback {
            self.player.current_detail.clone().or(browsed_detail)
        } else {
            browsed_detail.or_else(|| self.player.current_detail.clone())
        };

        let Some(detail) = detail else { return };

        // 如果已经在播放这首歌，不重复触发
        if self.player.current_detail.as_ref().map_or(false, |p| p.id == detail.id) {
            return;
        }

        // 把当前列表的所有歌曲替换进队列（与 nav_drill_in 行为一致）
        if let Some(songs) = self.cache.songs.get(&node).cloned() {
            if let Some(idx) = songs.iter().position(|s| s.id == detail.id) {
                self.play_from_list(&songs, idx);
                return;
            }
        } else if node == NavNode::Queue {
            // 已在队列中，只切换 current_index
            if let Some(idx) = self.queue.songs.iter().position(|q| q.id == detail.id) {
                self.queue.current_index = Some(idx);
            }
        } else {
            // 没有列表上下文，单独加入
            let item = Self::song_to_queue_item(&detail);
            if !self.queue.songs.iter().any(|q| q.id == item.id) {
                self.queue.add(item);
            }
            self.queue.current_index = self.queue.songs.iter().position(|q| q.id == detail.id);
        }
        self.player.follow_playback = true;
        self.start_audio_fetch(detail.id, &detail.title, &detail.uploader_name);
    }

    pub(crate) fn add_selected_to_queue(&mut self) {
        if let Some(song) = self.selected_song().cloned() {
            let item = Self::song_to_queue_item(&song);
            self.queue.add(item);
        }
    }

    pub(crate) fn remove_from_queue(&mut self) {
        if self.nav.current().node != NavNode::Queue {
            return;
        }
        let sel = self.nav.current().selected;
        if sel < self.queue.songs.len() {
            self.queue.remove(sel);
            // 修正选中索引
            let len = self.queue.songs.len();
            if len == 0 {
                self.nav.current_mut().selected = 0;
            } else if sel >= len {
                self.nav.current_mut().selected = len - 1;
            }
        }
    }

    /// 异步获取歌曲详情 → 下载音频 → 发送 AudioFetched
    pub(crate) fn start_audio_fetch(&mut self, song_id: i64, title: &str, artist: &str) {
        self.resume_position_ms = None; // 新歌播放时清除恢复位置
        self.player.bar.is_loading = true;
        self.player.bar.title = title.to_string();
        self.player.bar.artist = artist.to_string();

        // 记录播放历史，并使缓存失效以便下次进入时刷新
        self.cache.songs.remove(&NavNode::History);
        let history_client = self.client.clone();
        tokio::spawn(async move {
            if history_client.is_authenticated().await {
                let _ = history_client.touch_play_history(song_id).await;
            } else {
                let _ = history_client.touch_play_history_anonymous(song_id).await;
            }
        });

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
}
