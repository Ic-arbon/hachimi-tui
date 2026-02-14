use std::io::Cursor;
use std::time::Duration;

use anyhow::Result;
use rodio::{Decoder, OutputStream, Sink};
use tokio::sync::mpsc;

/// 播放引擎发给 UI 的事件
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Playing,
    Paused,
    Stopped,
    Progress { position_secs: u32, duration_secs: u32 },
    Error(String),
    TrackEnded,
    #[allow(dead_code)] // TODO: 加载状态指示
    Loading,
}

/// UI 发给播放引擎的命令
#[derive(Debug)]
pub enum PlayerCommand {
    Play(AudioSource, u32, Option<f32>), // (source, duration_secs, gain_db)
    Pause,
    Resume,
    #[allow(dead_code)] // TODO: 停止播放命令
    Stop,
    Seek(Duration),
    SetVolume(f32),
}

/// 音频来源
#[derive(Debug)]
pub enum AudioSource {
    /// 已缓存的完整音频数据
    Buffered(Vec<u8>),
}

pub struct PlayerEngine {
    cmd_tx: mpsc::UnboundedSender<PlayerCommand>,
    event_rx: Option<mpsc::UnboundedReceiver<PlayerEvent>>,
}

impl PlayerEngine {
    pub fn spawn() -> Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        std::thread::spawn(move || {
            player_thread(cmd_rx, event_tx);
        });

        Ok(Self { cmd_tx, event_rx: Some(event_rx) })
    }

    pub fn play(&self, source: AudioSource, duration_secs: u32, gain_db: Option<f32>) {
        let _ = self.cmd_tx.send(PlayerCommand::Play(source, duration_secs, gain_db));
    }

    pub fn pause(&self) {
        let _ = self.cmd_tx.send(PlayerCommand::Pause);
    }

    pub fn resume(&self) {
        let _ = self.cmd_tx.send(PlayerCommand::Resume);
    }

    #[allow(dead_code)] // TODO: 停止播放
    pub fn stop(&self) {
        let _ = self.cmd_tx.send(PlayerCommand::Stop);
    }

    pub fn seek(&self, pos: Duration) {
        let _ = self.cmd_tx.send(PlayerCommand::Seek(pos));
    }

    pub fn set_volume(&self, volume: f32) {
        let _ = self.cmd_tx.send(PlayerCommand::SetVolume(volume));
    }

    pub fn take_event_receiver(&mut self) -> mpsc::UnboundedReceiver<PlayerEvent> {
        self.event_rx.take().expect("event receiver already taken")
    }
}

fn player_thread(
    mut cmd_rx: mpsc::UnboundedReceiver<PlayerCommand>,
    event_tx: mpsc::UnboundedSender<PlayerEvent>,
) {
    let Ok((_stream, stream_handle)) = OutputStream::try_default() else {
        let _ = event_tx.send(PlayerEvent::Error("无法打开音频输出设备".to_string()));
        return;
    };

    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.pause();

    let mut has_source = false;
    let mut duration_secs: u32 = 0;
    let mut user_volume: f32 = 1.0;
    let mut gain_db: Option<f32> = None;

    let effective_volume = |uv: f32, g: Option<f32>| -> f32 {
        match g {
            Some(db) => (uv * 10_f32.powf(db / 20.0)).clamp(0.0, 2.0),
            None => uv,
        }
    };

    loop {
        // 非阻塞检查命令
        match cmd_rx.try_recv() {
            Ok(cmd) => match cmd {
                PlayerCommand::Play(source, dur, gain) => {
                    sink.stop();
                    duration_secs = dur;
                    gain_db = gain;
                    match source {
                        AudioSource::Buffered(data) => {
                            let cursor = Cursor::new(data);
                            match Decoder::new(cursor) {
                                Ok(decoder) => {
                                    sink.append(decoder);
                                    sink.set_volume(effective_volume(user_volume, gain_db));
                                    sink.play();
                                    has_source = true;
                                    let _ = event_tx.send(PlayerEvent::Playing);
                                }
                                Err(e) => {
                                    let _ = event_tx.send(PlayerEvent::Error(
                                        format!("解码失败: {e}"),
                                    ));
                                }
                            }
                        }
                    }
                }
                PlayerCommand::Pause => {
                    sink.pause();
                    let _ = event_tx.send(PlayerEvent::Paused);
                }
                PlayerCommand::Resume => {
                    sink.play();
                    let _ = event_tx.send(PlayerEvent::Playing);
                }
                PlayerCommand::Stop => {
                    sink.stop();
                    has_source = false;
                    gain_db = None;
                    let _ = event_tx.send(PlayerEvent::Stopped);
                }
                PlayerCommand::Seek(pos) => {
                    if let Err(e) = sink.try_seek(pos) {
                        let _ = event_tx.send(PlayerEvent::Error(
                            format!("Seek 失败: {e}"),
                        ));
                    }
                }
                PlayerCommand::SetVolume(vol) => {
                    user_volume = vol;
                    sink.set_volume(effective_volume(user_volume, gain_db));
                }
            },
            Err(mpsc::error::TryRecvError::Empty) => {}
            Err(mpsc::error::TryRecvError::Disconnected) => break,
        }

        // 上报播放进度
        if has_source && !sink.empty() && !sink.is_paused() {
            let pos = sink.get_pos().as_secs() as u32;
            let _ = event_tx.send(PlayerEvent::Progress {
                position_secs: pos,
                duration_secs,
            });
        }

        // 检测播放结束
        if has_source && sink.empty() {
            has_source = false;
            let _ = event_tx.send(PlayerEvent::TrackEnded);
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}
