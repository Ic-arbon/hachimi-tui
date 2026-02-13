# api — HTTP API 客户端

封装与 Hachimi 后端的全部 HTTP 通信。基于 `reqwest`，支持认证状态管理和自动 token 刷新。

## 文件索引

| 文件 | 职责 |
|------|------|
| `client.rs` | `HachimiClient` 结构体：HTTP 客户端初始化、请求封装（`get`/`post`/`get_with_query`）、Bearer token 管理（`Arc<RwLock>`）、自动刷新过期 token、音频流下载 |
| `endpoints.rs` | 所有 API 端点的请求参数结构体和 `impl HachimiClient` 方法，按领域分组：认证、歌曲、歌单、用户、播放历史 |

## API 覆盖

| 领域 | 端点示例 |
|------|---------|
| 认证 | `login`、`refresh_token`、`generate_captcha` |
| 歌曲 | `recent_songs`、`recommend_songs`、`hot_songs_weekly`、`search_songs`、`song_detail_by_id`、`recommend_tags` |
| 歌单 | `my_playlists`、`playlist_detail`、`create_playlist`、`add_song_to_playlist`、`search_playlists` |
| 用户 | `user_profile`、`search_users` |
| 历史 | `play_history`、`touch_play_history` |

## 认证流程

`HachimiClient` 内部通过 `Arc<RwLock<Option<AuthData>>>` 持有认证信息。
调用 `ensure_valid_auth()` 时，若 token 过期则自动使用 refresh_token 刷新。
所有需要认证的请求会自动附加 `Authorization: Bearer` 头。
