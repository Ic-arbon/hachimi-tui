# ui — 终端界面

基于 `ratatui` 的 TUI 渲染层。所有组件为纯函数式（接收状态引用，输出到 Frame），
不持有自身状态，状态由 `App` 统一管理。

## 文件索引

| 文件 | 职责 |
|------|------|
| `constants.rs` | UI 常量：面板尺寸（`HELP_PANEL_WIDTH`、`LOG_PANEL_*`、`LOGIN_FORM_WIDTH`）；布局比例（`HEADER_HEIGHT`、`PLAYER_BAR_HEIGHT`、`SEARCH_BAR_HEIGHT`、`MILLER_*_PCT`） |
| `i18n.rs` | 国际化：`t!()` 宏 + `Lang` 枚举（En/Zh）；`tr()` 函数查表返回 `&'static str`；全局原子变量存储当前语言 |
| `lyrics.rs` | LRC 歌词解析：`parse()` 支持 `[mm:ss.xx]` 时间标签（含多标签行）；`ParsedLyrics` 枚举（Synced/Plain/Empty）；`current_index()` 二分查找当前行 |
| `theme.rs` | `Theme` 工具结构体：`highlight()`、`secondary()`、`active()`、`error()` 等预设 `Style`（Cyan/DarkGray 为主色调）；`list_item_style(selected, active)` 统一列表项选中/激活样式 |
| `util.rs` | 渲染工具函数：`padded_rect` 水平内边距裁剪、`render_placeholder` 加载/空列表提示、`square_cells` 视觉近正方形尺寸计算、`gcd` |
| `miller.rs` | Miller Columns 三栏布局：`ColumnData` 共享数据结构、`render()` 布局分割、`render_column()` 单列渲染 |
| `preview.rs` | 预览列渲染：`render_preview_column()` 分派歌曲详情/队列项/用户/歌单/标签预览；`apply_cover()` 封面渲染辅助 |
| `format.rs` | 文本格式化：`song_list_line()` 标题+歌手行、`marquee_text()` 滚动文字、`truncate_with_dots()` 截断 |
| `navigation.rs` | 导航数据模型：`NavNode` 枚举（Root/Home/Library/Settings/Tag 等节点树）、`NavStack` 导航栈、`SearchState`/`SearchType`/`SearchSort` 搜索状态 |
| `player_bar.rs` | 底部播放状态栏：播放/暂停图标、歌曲名-歌手、时间进度、Braille 字符进度条 |
| `player_view.rs` | 展开播放器视图：左侧封面图（Kitty 图形协议）+ 右侧歌曲信息（浏览模式展示元数据、播放模式展示时间同步歌词） |
| `cover_widget.rs` | `CoverWidget`：Kitty Unicode Placeholder 封面渲染 Widget |
| `kitty.rs` | Kitty 图形协议：APC 序列生成（upload_rgb、create_placement、delete_image 等）、终端支持检测 |
| `login.rs` | 登录界面：ASCII art Logo（渐变色）+ 邮箱/密码表单 + captcha 流程提示；`LoginState` 管理表单状态和登录步骤 |
| `settings_view.rs` | 设置页面：可切换的设置项列表（语言、播放模式）；`cycle_setting()` 循环切换设置值 |
| `help.rs` | 快捷键帮助浮层：居中弹出，按分组列出所有键绑定 |
| `log_view.rs` | 日志浮层：`LogStore` 环形缓冲（200 条）+ 文件持久化（`hachimi.log`）；支持滚动浏览 |

## 渲染流程

```
App::render(frame)
  ├── render_header()          → 顶栏：标题 + 用户名 + 播放模式/音量/时间色块
  ├── match ui.input_mode
  │   ├── Login  → login::render()
  │   ├── player_expanded → player_view::render()
  │   ├── Settings → render_settings()
  │   ├── Search → search_bar + miller::render()
  │   └── _      → miller::render()    → Miller Columns 三栏
  ├── render_player_bar()      → 底栏：播放状态
  ├── [ui.show_logs] log_view::render()   → 日志浮层
  └── [ui.show_help] help::render()       → 帮助浮层
```
