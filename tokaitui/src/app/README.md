# app — 应用核心

应用主循环、状态管理、事件分发的中枢模块。`App` 结构体持有全部运行时状态，
通过 `mpsc` 通道接收终端事件、播放器回调、异步任务结果，在单线程中统一处理。

## 文件索引

| 文件 | 职责 |
|------|------|
| `mod.rs` | `App`、`UiState`、`CoverState`、`PlayerState`、`DataCache` 等结构体定义；`new()`/`run()`/`main_loop()` 生命周期方法 |
| `event.rs` | 终端事件分发（`handle_event`）；`handle_overlay_key` 处理帮助/日志浮层；Normal/Search/Login 三种输入模式的键盘处理；`handle_global_key` 提取 expanded/normal 共享键绑定（q/?/!/空格/n/N/±/⟨⟩/s）；`handle_message` 处理所有 `AppMessage` |
| `render.rs` | 帧渲染调度：header、miller columns、player bar、settings、player view、浮层（help/logs）、封面 placement |
| `actions/mod.rs` | 常量定义（`SEARCH_PAGE_SIZE`、`HISTORY_PAGE_SIZE`）+ 子模块声明 |
| `actions/auth.rs` | 认证流程：`start_captcha`、`submit_login`、`logout`、`resume_playback` |
| `actions/data.rs` | 数据加载：`execute_search`、`load_node_data`、`maybe_load_preview_data`、`maybe_fetch_song_detail`、`maybe_fetch_queue_detail` |
| `actions/playback.rs` | 播放控制：`toggle_play_pause`、`play_next`/`play_prev`、`play_from_list`、`play_expanded_song`、`start_audio_fetch`、队列操作 |
| `actions/navigation.rs` | Miller Columns 导航：`nav_down`/`up`/`drill_in`/`drill_out`/`top`/`bottom`、`after_nav_move`、`current_list_len` |
| `actions/cover.rs` | 封面图片：`schedule_cover_load`、`maybe_load_cover`、`current_preview_cover_url`；弹幕下载：`fetch_danmaku` |

## 状态分组

```
App
├── player: PlayerState      # 播放引擎 + 播放栏 + 音量/静音/展开
├── cache: DataCache          # 歌曲/标签缓存 + 加载状态 + 封面缓存(CoverCache)
├── queue: QueueState         # 播放队列（独立模块）
├── nav: NavStack             # Miller Columns 导航栈
├── search: SearchState       # 搜索输入状态
├── ui: UiState               # input_mode, show_help, help_scroll, show_logs, logs, scroll_tick
├── cover: CoverState         # kitty_supported, pending_cover_load, active_cover_ids, needs_cover_reupload
├── login: LoginState         # 登录表单状态
└── ...                       # running, settings, client, username, msg channel 等
```

## 消息驱动架构

```
┌─────────────┐   TermEvent    ┌───────────────┐
│ 事件读取线程 │ ──────────────→ │               │
└─────────────┘                │               │
┌─────────────┐   PlayerTick   │               │
│ tick 定时器  │ ──────────────→ │   main_loop   │
└─────────────┘                │  msg_rx.recv  │
┌─────────────┐  PlayerState   │               │
│ 播放引擎     │ ──────────────→ │               │
└─────────────┘                └───────┬───────┘
┌─────────────┐  DataLoaded/          │
│ tokio 异步   │  AudioFetched/       ↓
│   任务       │  LoginResult  → handle_message
└─────────────┘
```
