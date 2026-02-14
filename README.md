# Hachimi TUI

[English](#english) | [中文](#中文)

```
░█░█░█▀█░█▀▀░█░█░▀█▀░█▄█░▀█▀
░█▀█░█▀█░█░░░█▀█░░█░░█░█░░█░
░▀░▀░▀░▀░▀▀▀░▀░▀░▀▀▀░▀░▀░▀▀▀
```

---

## English

Terminal music player for [Hachimi](https://hachimi.world).

### Features

- Miller Columns navigation (latest / recommended / weekly hot / tags / playlists / history / search)
- Kitty image protocol cover art rendering
- Time-synced LRC lyrics
- Playback modes: sequential, shuffle, repeat-one
- ReplayGain support
- Queue management with persistence across sessions
- Vim-style keybindings
- i18n: English, Simplified Chinese (auto-detect from locale)
- Kids mode (hide explicit content)

### Keybindings

| Key | Action |
|-----|--------|
| `j/k` | Navigate up/down |
| `h/l` | Drill out/in |
| `Enter` | Play / enter |
| `Space` | Play/pause |
| `n/N` | Next/previous track |
| `+/-` | Volume up/down |
| `</>` | Seek backward/forward 5s |
| `s` | Cycle play mode |
| `i` | Toggle expanded player view |
| `/` | Search |
| `Tab` | Switch search type (song/user/playlist) |
| `a/d` | Add to / remove from queue |
| `o` | Open external link |
| `g/G` | Jump to top/bottom |
| `L` | Logout |
| `?` | Help |
| `!` | Logs |
| `q` | Quit |

### Project Structure

```
hachimi-core/     API client & data models (reusable library)
hachimi-tui/      Terminal UI application
├── src/app/      App state, event loop, actions, rendering dispatch
├── src/config/   Settings, auth persistence, paths
├── src/model/    Queue model (TUI-specific)
├── src/player/   Audio engine (rodio), queue playback modes
└── src/ui/       Ratatui widgets, Miller Columns, player view, i18n
```

### Tech Stack

- **UI**: ratatui + crossterm + ratatui-image
- **Audio**: rodio (MP3, AAC, FLAC, Vorbis, WAV)
- **HTTP**: reqwest
- **Async**: tokio
- **Edition**: Rust 2024

### Build

```sh
cargo build --release
```

Or with Nix:

```sh
nix develop
cargo build --release
```

### Configuration

Files are stored in `~/.config/hachimi-tui/`:

| File | Description |
|------|-------------|
| `config.toml` | Player, cache, display settings |
| `auth.json` | Credentials (mode 600) |
| `queue.json` | Playback queue state |

---

## 中文

[Hachimi](https://hachimi.world) 的终端音乐播放器。

### 功能

- Miller Columns 三栏导航（最新 / 推荐 / 周热门 / 标签 / 歌单 / 历史 / 搜索）
- Kitty 图像协议封面渲染
- LRC 时间同步歌词
- 播放模式：顺序播放、随机播放、单曲循环
- ReplayGain 响度均衡
- 播放队列跨会话持久化
- Vim 风格快捷键
- 国际化：中文、英文（自动检测系统 locale）
- 儿童模式（隐藏 explicit 内容）

### 快捷键

| 按键 | 功能 |
|------|------|
| `j/k` | 上下导航 |
| `h/l` | 返回/进入 |
| `Enter` | 播放/进入 |
| `Space` | 播放/暂停 |
| `n/N` | 下一首/上一首 |
| `+/-` | 音量加/减 |
| `</>` | 快退/快进 5 秒 |
| `s` | 切换播放模式 |
| `i` | 展开/收起播放器 |
| `/` | 搜索 |
| `Tab` | 切换搜索类型（歌曲/用户/歌单） |
| `a/d` | 添加到队列/从队列移除 |
| `o` | 打开外部链接 |
| `g/G` | 跳到顶部/底部 |
| `L` | 登出 |
| `?` | 帮助 |
| `!` | 日志 |
| `q` | 退出 |

### 项目结构

```
hachimi-core/     API 客户端与数据模型（可复用库）
hachimi-tui/      终端 UI 应用
├── src/app/      应用状态、事件循环、业务逻辑、渲染分发
├── src/config/   配置、认证持久化、路径管理
├── src/model/    队列模型（TUI 专用）
├── src/player/   音频引擎（rodio）、队列播放模式
└── src/ui/       Ratatui 组件、Miller Columns、播放器视图、国际化
```

### 技术栈

- **UI**: ratatui + crossterm + ratatui-image
- **音频**: rodio（MP3、AAC、FLAC、Vorbis、WAV）
- **HTTP**: reqwest
- **异步**: tokio
- **Edition**: Rust 2024

### 构建

```sh
cargo build --release
```

或通过 Nix：

```sh
nix develop
cargo build --release
```

### 配置文件

存储在 `~/.config/hachimi-tui/`：

| 文件 | 说明 |
|------|------|
| `config.toml` | 播放器、缓存、显示设置 |
| `auth.json` | 认证凭据（权限 600） |
| `queue.json` | 播放队列状态 |
