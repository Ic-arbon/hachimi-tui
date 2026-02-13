# model — 数据模型

与后端 API 对应的数据结构定义，以及客户端本地状态模型。全部使用 serde 进行序列化/反序列化。

## 文件索引

| 文件 | 职责 |
|------|------|
| `mod.rs`（`model.rs`） | 通用 API 响应包装 `WebResp<T>`（ok/data 结构）、`CommonError` 错误类型、`play_history` 子模块（播放历史条目） |
| `auth.rs` | 认证相关：`LoginReq`/`LoginResp`、`TokenPair`、`RefreshTokenReq`、`GenerateCaptchaResp` |
| `song.rs` | 歌曲相关：`PublicSongDetail`（完整详情）、`SearchSongItem`（搜索结果，可转为 Detail）、`TagItem`/`TagRecommendItem`、各 API 响应体 |
| `playlist.rs` | 歌单相关：`PlaylistItem`（列表项）、`PlaylistMetadata`（搜索结果）、`PlaylistSongItem`（歌单内歌曲）、`PlaylistDetailResp` 等 |
| `queue.rs` | 播放队列：`MusicQueueItem`（队列条目）、`QueueState`（队列状态 + 增删查改 + 按模式切换上下首）；支持 JSON 持久化到 `queue.json` |
| `user.rs` | 用户相关：`PublicUserProfile`、`UserSearchResp` |

## 设计说明

- `WebResp<T>` 统一处理后端 `{ ok, data }` 格式，`into_result()` 将其转为 `Result<T, CommonError>`
- `SearchSongItem` 提供 `into_song_detail()` 方法，将搜索结果轻量结构转为完整的 `PublicSongDetail`
- `QueueState` 同时承担运行时队列管理和磁盘持久化职责
