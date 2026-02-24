use mambocore::endpoints::{
    HistoryCursorQuery, PageByUserQuery, PlaylistSearchQuery, RecentQuery, SongSearchQuery,
    UserSearchQuery,
};

use crate::model::song::PublicSongDetail;
use crate::ui::navigation::{NavNode, SearchSort};

use super::super::{App, AppMessage, DataPayload};
use super::{HISTORY_PAGE_SIZE, SEARCH_PAGE_SIZE};

impl App {
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
