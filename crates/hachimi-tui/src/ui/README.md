# ui — 终端界面

基于 `ratatui` 的 TUI 渲染层。所有组件为纯函数式（接收状态引用，输出到 Frame），
不持有自身状态，状态由 `App` 统一管理。

## 文件索引

| 文件 | 职责 |
|------|------|
| `i18n.rs` | 国际化：`t!()` 宏 + `Lang` 枚举（En/Zh）；`tr()` 函数查表返回 `&'static str`；全局原子变量存储当前语言 |
| `theme.rs` | `Theme` 工具结构体：提供 `highlight()`、`secondary()`、`active()`、`error()` 等预设 `Style`（Cyan/DarkGray 为主色调） |
| `miller.rs` | Miller Columns 三栏布局：父级列表 + 当前列表 + 预览列（歌曲详情/封面图）；支持歌曲列表、标签列表、静态导航节点的渲染 |
| `navigation.rs` | 导航数据模型：`NavNode` 枚举（Root/Home/Library/Settings/Tag 等节点树）、`NavStack` 导航栈、`SearchState`/`SearchType`/`SearchSort` 搜索状态 |
| `player_bar.rs` | 底部播放状态栏：播放/暂停图标、歌曲名-歌手、时间进度、Braille 字符进度条 |
| `player_view.rs` | 展开播放器视图：左侧封面图（`ratatui-image`）+ 右侧歌曲信息 |
| `login.rs` | 登录界面：ASCII art Logo（渐变色）+ 邮箱/密码表单 + captcha 流程提示；`LoginState` 管理表单状态和登录步骤 |
| `settings_view.rs` | 设置页面：可切换的设置项列表（语言、播放模式）；`cycle_setting()` 循环切换设置值 |
| `help.rs` | 快捷键帮助浮层：居中弹出，按分组列出所有键绑定 |
| `log_view.rs` | 日志浮层：`LogStore` 环形缓冲（200 条）+ 文件持久化（`hachimi.log`）；支持滚动浏览 |

## 渲染流程

```
App::render(frame)
  ├── render_header()          → 顶栏：标题 + 用户名 + 播放模式/音量/时间色块
  ├── match input_mode
  │   ├── Login  → login::render()
  │   ├── player_expanded → player_view::render()
  │   ├── Settings → render_settings()
  │   └── _      → miller::render()    → Miller Columns 三栏
  ├── render_player_bar()      → 底栏：播放状态
  ├── [show_logs] log_view::render()   → 日志浮层
  └── [show_help] help::render()       → 帮助浮层
```
