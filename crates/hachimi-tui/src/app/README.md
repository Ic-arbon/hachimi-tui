# app — 应用核心

应用主循环、状态管理、事件分发的中枢模块。`App` 结构体持有全部运行时状态，
通过 `mpsc` 通道接收终端事件、播放器回调、异步任务结果，在单线程中统一处理。

## 文件索引

| 文件 | 职责 |
|------|------|
| `mod.rs` | `App` 结构体与子状态（`PlayerState`、`DataCache`）定义；`new()`/`run()`/`main_loop()` 生命周期方法 |
| `event.rs` | 终端事件分发（`handle_event`）；Normal/Search/Login 三种输入模式的键盘处理；`handle_global_key` 提取 expanded/normal 共享键绑定（q/?/!/空格/n/N/±/⟨⟩/s）；`adjust_volume`/`seek_relative` 封装音量和快进操作；`handle_message` 处理所有 `AppMessage` |
| `actions.rs` | 业务动作：认证流程（captcha → login → logout）、数据加载、播放控制（play/pause/next/prev/seek）、Miller Columns 导航、图片异步加载；`play_from_list` 统一"替换队列并播放"逻辑；`on_selection_changed` 收敛列表选择变化后的 `follow_playback`/`scroll_tick` 处理 |
| `render.rs` | 帧渲染调度：header、miller columns、player bar、settings、player view、浮层（help/logs） |

## 状态分组

```
App
├── player: PlayerState      # 播放引擎 + 播放栏 + 音量/静音/展开
├── cache: DataCache          # 歌曲/标签缓存 + 加载状态 + 图片缓存 + Picker
├── queue: QueueState         # 播放队列（独立模块）
├── nav: NavStack             # Miller Columns 导航栈
├── search: SearchState       # 搜索输入状态
├── login: LoginState         # 登录表单状态
└── ...                       # running, settings, client, logs, msg channel 等
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
