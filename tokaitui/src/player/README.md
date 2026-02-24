# player — 音频播放引擎

基于 `rodio` 的音频播放，运行在独立线程中，通过命令通道与主线程通信。

## 文件索引

| 文件 | 职责 |
|------|------|
| `engine.rs` | `PlayerEngine`：在独立线程中运行 rodio sink；通过 `mpsc` 接收 `PlayerCommand`（Play/Pause/Resume/Stop/Seek/SetVolume）；通过 `watch` 通道广播 `PlayerEvent`（Playing/Paused/Stopped/Progress/TrackEnded/Error）；50ms 间隔上报播放进度 |
| `queue.rs` | `QueueState` 的播放模式扩展：`next_with_mode()`/`prev_with_mode()` 根据 `PlayMode`（Sequential/Shuffle/RepeatOne）决定下一首 |

## 架构

```
App (主线程/tokio)                   Player 线程
     │                                    │
     │── PlayerCommand::Play(data) ──────→│ rodio Sink 播放
     │── PlayerCommand::Pause ───────────→│
     │── PlayerCommand::Seek(pos) ───────→│
     │                                    │
     │←── PlayerEvent::Playing ──────────│
     │←── PlayerEvent::Progress{..} ─────│ (每 50ms)
     │←── PlayerEvent::TrackEnded ───────│
```

## 音频源

目前仅支持 `AudioSource::Buffered(Vec<u8>)`——先完整下载到内存再播放。
音频数据由 `app/actions/playback.rs` 中的 `start_audio_fetch` 异步下载后通过 `AppMessage::AudioFetched` 传递。
