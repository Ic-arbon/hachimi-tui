use mambocore::endpoints::{
    HistoryCursorQuery, PageByUserQuery, PlaylistSearchQuery, RecentQuery, SongSearchQuery,
    UserSearchQuery,
};
use crate::model::auth::LoginReq;
use crate::model::queue::MusicQueueItem;
use crate::model::song::PublicSongDetail;
use crate::ui::login::{LoginState, LoginStep};
use crate::ui::navigation::{NavNode, SearchSort, SearchType};

use super::{App, AppMessage, DataPayload, InputMode};

const SEARCH_PAGE_SIZE: i32 = 30;
const HISTORY_PAGE_SIZE: i32 = 50;

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
                device_info: "tokaitui".to_string(),
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
        self.cache.tags = None;
        self.cache.playlists = None;
        self.cache.queue_song_detail.clear();
        self.login = LoginState::new();
        self.input_mode = InputMode::Login;
    }

    /// 恢复上次退出时的播放
    pub(crate) fn resume_playback(&mut self) {
        if let Some(song) = self.queue.current_song().cloned() {
            let pos = self.resume_position_ms;
            self.start_audio_fetch(song.id, &song.name, &song.artist);
            self.resume_position_ms = pos;
        }
    }

    // — 搜索 —

    pub(crate) fn execute_search(&mut self) {
        let query = self.search.query.trim().to_string();
        let sort = self.search.sort;
        let tx = self.msg_tx.clone();
        let client = self.client.clone();

        // 清空旧结果
        self.cache.songs.remove(&NavNode::SearchResults);
        self.cache.search_users.clear();
        self.cache.search_playlists.clear();
        self.cache.loading.insert(NavNode::SearchResults);

        let sort_by = match sort {
            SearchSort::Relevance => None,
            SearchSort::Newest => Some("release_time_desc".to_string()),
            SearchSort::Oldest => Some("release_time_asc".to_string()),
        };

        // 同时搜索三种类型
        tokio::spawn(async move {
            let song_q = SongSearchQuery {
                q: query.clone(),
                limit: Some(SEARCH_PAGE_SIZE),
                offset: None,
                filter: None,
                sort_by,
            };
            let user_q = UserSearchQuery {
                q: query.clone(),
                page: 0,
                size: SEARCH_PAGE_SIZE,
            };
            let playlist_q = PlaylistSearchQuery {
                q: query,
                limit: Some(SEARCH_PAGE_SIZE as i64),
                offset: None,
                sort_by: None,
                user_id: None,
            };
            let (songs_res, users_res, playlists_res) = tokio::join!(
                client.search_songs(&song_q),
                client.search_users(&user_q),
                client.search_playlists(&playlist_q),
            );

            match songs_res {
                Ok(resp) => {
                    let songs: Vec<PublicSongDetail> =
                        resp.hits.into_iter().map(|s| s.into_song_detail()).collect();
                    let _ = tx.send(AppMessage::DataLoaded(DataPayload::Songs(
                        NavNode::SearchResults,
                        songs,
                    )));
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::DataLoaded(DataPayload::Songs(
                        NavNode::SearchResults,
                        vec![],
                    )));
                    let _ = tx.send(AppMessage::Error(e.to_string()));
                }
            }
            match users_res {
                Ok(resp) => {
                    let _ = tx.send(AppMessage::DataLoaded(DataPayload::SearchUsers(resp.hits)));
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::DataLoaded(DataPayload::SearchUsers(vec![])));
                    let _ = tx.send(AppMessage::Error(e.to_string()));
                }
            }
            match playlists_res {
                Ok(resp) => {
                    let _ = tx.send(AppMessage::DataLoaded(DataPayload::SearchPlaylists(resp.hits)));
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::DataLoaded(DataPayload::SearchPlaylists(vec![])));
                    let _ = tx.send(AppMessage::Error(e.to_string()));
                }
            }
        });
    }

    // — 数据加载 —

    pub(crate) fn load_node_data(&mut self, node: &NavNode) {
        // Categories 用 tag_cache，MyPlaylists 用 playlist_cache
        if *node == NavNode::Categories {
            if self.cache.loading.contains(node) || self.cache.tags.is_some() {
                return;
            }
        } else if *node == NavNode::MyPlaylists {
            if self.cache.loading.contains(node) || self.cache.playlists.is_some() {
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

            // MyPlaylists 走单独的歌单列表加载流程
            if node_owned == NavNode::MyPlaylists {
                match client.my_playlists().await {
                    Ok(resp) => {
                        let _ = tx.send(AppMessage::DataLoaded(DataPayload::Playlists(resp.playlists)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::DataLoaded(DataPayload::Playlists(vec![])));
                        let _ = tx.send(AppMessage::Error(e.to_string()));
                    }
                }
                return;
            }

            let result = match &node_owned {
                NavNode::LatestReleases => client
                    .recent_songs(&RecentQuery {
                        cursor: None,
                        limit: SEARCH_PAGE_SIZE,
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
                            limit: Some(SEARCH_PAGE_SIZE),
                            offset: None,
                            filter: Some(format!("tags = \"{}\"", name)),
                            sort_by: Some("release_time_desc".to_string()),
                        })
                        .await
                        .map(|r| r.hits.into_iter().map(|s| s.into_song_detail()).collect())
                }
                NavNode::History => {
                    client
                        .play_history(&HistoryCursorQuery { cursor: None, size: HISTORY_PAGE_SIZE })
                        .await
                        .map(|r| r.list.into_iter().map(|h| h.song_info).collect())
                }
                NavNode::PlaylistDetail { id } => {
                    client
                        .playlist_detail_private(*id)
                        .await
                        .map(|r| r.songs.into_iter().map(|s| s.into_song_detail()).collect())
                }
                NavNode::UserDetail { id } => {
                    client
                        .songs_by_user(&PageByUserQuery {
                            user_id: *id,
                            page: None,
                            size: Some(HISTORY_PAGE_SIZE as i64),
                        })
                        .await
                        .map(|r| r.songs)
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
            if let Some(tag_name) = self.cache.tags.as_ref().and_then(|t| t.get(sel)).cloned() {
                let tag_node = NavNode::Tag { name: tag_name };
                self.load_node_data(&tag_node);
            }
        } else if node == NavNode::MyPlaylists {
            // 加载选中歌单的歌曲预览
            if let Some(pl) = self.cache.playlists.as_ref().and_then(|p| p.get(sel)) {
                let pl_node = NavNode::PlaylistDetail { id: pl.id };
                self.load_node_data(&pl_node);
            }
        }
    }

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
    fn play_from_list(&mut self, songs: &[PublicSongDetail], index: usize) {
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

    // — Miller Columns 导航 —

    pub(crate) fn after_nav_move(&mut self) {
        self.maybe_load_preview_data();
        self.maybe_fetch_song_detail();
        self.maybe_fetch_queue_detail();
        self.schedule_cover_load();
    }

    /// 用户手动改变选中项后的共享后处理
    fn on_selection_changed(&mut self) {
        if self.player.expanded {
            self.player.follow_playback = false;
        }
        self.scroll_tick = 0;
        self.after_nav_move();
    }

    fn push_and_load(&mut self, node: NavNode) {
        self.load_node_data(&node);
        self.nav.push(node);
        self.scroll_tick = 0;
        self.after_nav_move();
    }

    pub(crate) fn current_list_len(&self) -> usize {
        let node = &self.nav.current().node;
        if *node == NavNode::Settings {
            crate::ui::settings_view::ITEMS_COUNT
        } else if node.has_static_children() {
            node.children().len()
        } else if *node == NavNode::Categories {
            self.cache.tags.as_ref().map_or(0, |t| t.len())
        } else if *node == NavNode::MyPlaylists {
            self.cache.playlists.as_ref().map_or(0, |p| p.len())
        } else if *node == NavNode::Queue {
            self.queue.songs.len()
        } else if *node == NavNode::SearchResults {
            match self.search.search_type {
                SearchType::Song => {
                    self.cache.songs.get(&NavNode::SearchResults).map_or(0, |s| s.len())
                }
                SearchType::User => self.cache.search_users.len(),
                SearchType::Playlist => self.cache.search_playlists.len(),
            }
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
            }
        }
        self.on_selection_changed();
    }

    pub(crate) fn nav_up(&mut self) {
        let sel = self.nav.current().selected;
        if sel > 0 {
            self.nav.current_mut().selected = sel - 1;
        }
        self.on_selection_changed();
    }

    pub(crate) fn nav_drill_in(&mut self) {
        let node = self.nav.current().node.clone();
        let sel = self.nav.current().selected;
        if node == NavNode::Settings {
            crate::ui::settings_view::cycle_setting(&mut self.settings, sel);
            if sel == 3 {
                // cover_scale 变化
            }
            let _ = self.settings.save();
            return;
        }
        if node.has_static_children() {
            let children = node.children();
            if sel < children.len() {
                let child = children[sel].clone();
                self.push_and_load(child);
            }
        } else if node == NavNode::Categories {
            // 进入选中的标签
            if let Some(tag_name) = self.cache.tags.as_ref().and_then(|t| t.get(sel)).cloned() {
                self.push_and_load(NavNode::Tag { name: tag_name });
            }
        } else if node == NavNode::MyPlaylists {
            // 进入选中的歌单
            if let Some(pl) = self.cache.playlists.as_ref().and_then(|p| p.get(sel)) {
                let pl_node = NavNode::PlaylistDetail { id: pl.id };
                self.push_and_load(pl_node);
            }
        } else if node == NavNode::Queue {
            // 队列中按 Enter 播放选中歌曲
            if sel < self.queue.songs.len() {
                self.queue.current_index = Some(sel);
                let item = self.queue.songs[sel].clone();
                self.start_audio_fetch(item.id, &item.name, &item.artist);
            }
        } else if node == NavNode::SearchResults {
            match self.search.search_type {
                SearchType::Song => {
                    if let Some(songs) = self.cache.songs.get(&NavNode::SearchResults).cloned() {
                        if sel < songs.len() {
                            self.play_from_list(&songs, sel);
                        }
                    }
                }
                SearchType::Playlist => {
                    if let Some(pl) = self.cache.search_playlists.get(sel) {
                        let pl_node = NavNode::PlaylistDetail { id: pl.id };
                        self.push_and_load(pl_node);
                    }
                }
                SearchType::User => {
                    if let Some(user) = self.cache.search_users.get(sel) {
                        self.push_and_load(NavNode::UserDetail { id: user.uid });
                    }
                }
            }
        } else {
            // 当前节点是歌曲列表，按 Enter 播放选中歌曲
            if let Some(songs) = self.cache.songs.get(&node).cloned() {
                if sel < songs.len() {
                    self.play_from_list(&songs, sel);
                }
            }
        }
    }

    pub(crate) fn nav_drill_out(&mut self) {
        self.nav.pop();
        self.scroll_tick = 0;
        self.after_nav_move();
    }

    pub(crate) fn nav_top(&mut self) {
        self.nav.current_mut().selected = 0;
        self.on_selection_changed();
    }

    pub(crate) fn nav_bottom(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            self.nav.current_mut().selected = len - 1;
        }
        self.on_selection_changed();
    }

    /// 预览选中歌曲时，若为搜索结果（partial），异步补全完整详情
    pub(crate) fn maybe_fetch_song_detail(&mut self) {
        let node = self.nav.current().node.clone();
        let sel = self.nav.current().selected;

        // 仅对歌曲列表节点生效
        if node.has_static_children() || node == NavNode::Categories || node == NavNode::MyPlaylists || node == NavNode::Queue || node == NavNode::Settings {
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
        if !self.kitty_supported {
            return;
        }
        if let Some(url) = self.current_preview_cover_url() {
            if !url.is_empty()
                && !self.cache.covers.is_ready(&url)
                && !self.cache.covers.is_loading(&url)
            {
                self.pending_cover_load = Some((url, std::time::Instant::now()));
            }
        }
    }

    /// 异步下载并上传封面到终端（Kitty 图形协议）
    pub(crate) fn maybe_load_cover(&mut self, url: String) {
        if !self.kitty_supported {
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
                let img = img.resize(200, 200, image::imageops::FilterType::Lanczos3);
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

    /// 队列预览时异步获取选中项的完整歌曲详情
    pub(crate) fn maybe_fetch_queue_detail(&mut self) {
        if self.nav.current().node != NavNode::Queue {
            return;
        }
        let sel = self.nav.current().selected;
        if let Some(item) = self.queue.songs.get(sel) {
            let song_id = item.id;
            if self.cache.queue_song_detail.contains_key(&song_id)
                || self.cache.detail_loading.contains(&song_id)
            {
                return;
            }
            self.cache.detail_loading.insert(song_id);
            let tx = self.msg_tx.clone();
            let client = self.client.clone();

            tokio::spawn(async move {
                if let Ok(detail) = client.song_detail_by_id(song_id).await {
                    let _ = tx.send(AppMessage::SongDetailFetched {
                        node: NavNode::Queue,
                        index: 0,
                        detail,
                    });
                }
            });
        }
    }

}
