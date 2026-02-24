use crate::ui::navigation::{NavNode, SearchType};

use super::super::App;

impl App {
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
        self.ui.scroll_tick = 0;
        self.after_nav_move();
    }

    fn push_and_load(&mut self, node: NavNode) {
        self.load_node_data(&node);
        self.nav.push(node);
        self.ui.scroll_tick = 0;
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
        self.ui.scroll_tick = 0;
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
}
