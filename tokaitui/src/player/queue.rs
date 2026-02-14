use anyhow::Result;

use crate::config::settings::PlayMode;
use crate::model::queue::{MusicQueueItem, QueueState};
use crate::config::paths;

impl QueueState {
    pub fn load_persisted() -> Result<Self> {
        let path = paths::queue_file()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::new())
        }
    }

    pub fn persist(&self) -> Result<()> {
        let path = paths::queue_file()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn next_with_mode(&mut self, mode: &PlayMode) -> Option<&MusicQueueItem> {
        match mode {
            PlayMode::Sequential => self.next(),
            PlayMode::RepeatOne => {
                self.position_ms = 0;
                self.current_song()
            }
            PlayMode::Shuffle => {
                if self.songs.is_empty() {
                    return None;
                }
                use rand::Rng;
                let mut rng = rand::rng();
                let new_idx = rng.random_range(0..self.songs.len());
                self.current_index = Some(new_idx);
                self.position_ms = 0;
                self.songs.get(new_idx)
            }
        }
    }

    pub fn prev_with_mode(&mut self, mode: &PlayMode) -> Option<&MusicQueueItem> {
        match mode {
            PlayMode::RepeatOne => {
                self.position_ms = 0;
                self.current_song()
            }
            _ => self.prev(),
        }
    }
}
