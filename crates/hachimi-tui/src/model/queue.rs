use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicQueueItem {
    pub id: i64,
    pub display_id: String,
    pub name: String,
    pub artist: String,
    pub duration_secs: i32,
    pub cover_url: String,
    pub explicit: Option<bool>,
    pub audio_url: String,
    pub gain: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueState {
    pub current_index: Option<usize>,
    pub position_ms: u64,
    pub songs: Vec<MusicQueueItem>,
}

impl QueueState {
    pub fn new() -> Self {
        Self {
            current_index: None,
            position_ms: 0,
            songs: Vec::new(),
        }
    }

    pub fn current_song(&self) -> Option<&MusicQueueItem> {
        self.current_index.and_then(|i| self.songs.get(i))
    }

    pub fn next(&mut self) -> Option<&MusicQueueItem> {
        if let Some(idx) = self.current_index {
            if idx + 1 < self.songs.len() {
                self.current_index = Some(idx + 1);
                self.position_ms = 0;
                return self.songs.get(idx + 1);
            }
        }
        None
    }

    pub fn prev(&mut self) -> Option<&MusicQueueItem> {
        if let Some(idx) = self.current_index {
            if idx > 0 {
                self.current_index = Some(idx - 1);
                self.position_ms = 0;
                return self.songs.get(idx - 1);
            }
        }
        None
    }

    pub fn add(&mut self, item: MusicQueueItem) {
        self.songs.push(item);
        if self.current_index.is_none() && !self.songs.is_empty() {
            self.current_index = Some(0);
        }
    }

    pub fn insert_next(&mut self, item: MusicQueueItem) {
        let pos = self.current_index.map_or(0, |i| i + 1);
        self.songs.insert(pos, item);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.songs.len() {
            self.songs.remove(index);
            if let Some(curr) = self.current_index {
                if index < curr {
                    self.current_index = Some(curr - 1);
                } else if index == curr {
                    if self.songs.is_empty() {
                        self.current_index = None;
                    } else if curr >= self.songs.len() {
                        self.current_index = Some(self.songs.len() - 1);
                    }
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.songs.clear();
        self.current_index = None;
        self.position_ms = 0;
    }
}
