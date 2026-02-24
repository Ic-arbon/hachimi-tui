# 重构计划

## 概览

项目当前约 6,773 行 Rust 代码，架构基础良好但若干核心文件已膨胀。
本文档按优先级列出重构项，每项标注影响范围、具体步骤和验收标准。

---

## P0 — 架构拆分

### 1. actions.rs 拆分（902 行 → 5 个子模块）

**问题**：单文件承载认证、搜索、数据加载、播放、队列、导航、封面、设置全部业务逻辑。

**目标结构**：
```
app/actions/
├── mod.rs          // pub use 统一导出
├── auth.rs         // start_captcha, submit_login, logout, resume_playback
├── playback.rs     // toggle_play_pause, play_next, play_prev, seek_relative, play_expanded_song
├── navigation.rs   // nav_down/up/top/bottom, nav_drill_in/out, add_selected_to_queue, remove_from_queue
├── data.rs         // load_node_data, execute_search
└── cover.rs        // schedule_cover_load, maybe_load_cover, apply_cover, render_cover_placements
```

**步骤**：
1. 创建 `app/actions/` 目录和 `mod.rs`
2. 按函数归属移动到对应子文件，保持 `impl App` 签名不变
3. `mod.rs` 中 `pub use` 保证外部调用零改动
4. 编译通过即可，无需改接口

**验收**：`cargo build` 通过，行为不变。

---

### 2. miller.rs 拆分（826 行 → 3 个文件）

**问题**：混合了列渲染、预览渲染、歌曲格式化、封面调度。

**目标结构**：
```
ui/
├── miller.rs         // 主布局 + render_column（精简后 ~400 行）
├── preview.rs        // render_preview_column + 预览相关逻辑
└── format.rs         // song_list_line, playlist_line 等格式化函数
```

**步骤**：
1. 提取 `song_list_line()`、播放列表/用户格式化等纯函数到 `ui/format.rs`
2. 提取 `render_preview_column()` 及其辅助函数到 `ui/preview.rs`
3. `miller.rs` 只保留主布局分割和 `render_column()` 调度

**验收**：`cargo build` 通过，UI 渲染无变化。

---

### 3. App 结构体字段聚合（~20 字段 → 子状态）

**问题**：App 是 God Object，所有状态平铺。

**目标**：
```rust
pub struct App {
    pub running: bool,
    pub settings: Settings,
    pub client: MamboClient,
    pub player: PlayerState,
    pub queue: QueueState,
    pub cache: DataCache,
    pub nav: NavStack,
    pub search: SearchState,
    pub login: LoginState,
    pub ui: UiState,           // NEW: show_help, show_logs, logs, scroll_tick, input_mode
    pub cover: CoverState,     // NEW: kitty_supported, pending_cover_load, active_cover_ids, needs_cover_reupload
    pub msg_tx: ...,
    pub msg_rx: ...,
    pub username: Option<String>,
}

pub struct UiState {
    pub input_mode: InputMode,
    pub show_help: bool,
    pub show_logs: bool,
    pub logs: LogStore,
    pub scroll_tick: u16,
}

pub struct CoverState {
    pub kitty_supported: bool,
    pub pending_cover_load: Option<(String, Instant)>,
    pub active_cover_ids: Vec<u32>,
    pub needs_cover_reupload: bool,
}
```

**步骤**：
1. 新建 `UiState`、`CoverState` 结构体（放在 `app/mod.rs` 内）
2. 将对应字段迁入，全局替换 `self.show_help` → `self.ui.show_help` 等
3. 逐文件修改引用（主要影响 event.rs、render.rs、actions/cover.rs）

**验收**：`cargo build` 通过，行为不变。

---

## P1 — 正确性修复

### 4. CoverCache 资源泄漏

**问题**：`evict_one()` 只从 HashMap 删除，未向终端发送 `delete_image` 清理图像数据。

**修复**：
```rust
// evict_one() 改为返回被驱逐的 id
fn evict_one(&mut self) -> Option<u32> {
    // ... 找到最旧条目
    let evicted_id = entry.id;
    self.entries.remove(&url);
    self.ids.remove(&url);
    Some(evicted_id)
}
```
调用方拿到 id 后写入 `kitty::delete_image(id)` 到 stdout。

**验收**：长时间浏览不同歌曲后，终端内存占用保持稳定。

---

### 5. QueueState 边界安全

**问题**：`next()`、`prev()`、`current_song()` 假设 index 有效，空队列或越界会 panic。

**修复**：
- 所有索引操作前加 `if self.songs.is_empty() { return None; }`
- 用 `.get(index)` 替代 `self.songs[index]`
- `current_index` 改为 `Option<usize>`，或每次访问前 clamp

**验收**：空队列时操作不 panic，快速增删歌曲不崩溃。

---

### 6. 翻译 key 编译期校验

**问题**：`t!("key")` 拼错时运行时静默显示 `???`。

**修复方案 A**（推荐，改动小）：
在 debug build 中，`tr()` 函数对未找到的 key 加 `debug_assert!` 或 `eprintln!` 告警。

**修复方案 B**（彻底）：
将所有 key 改为枚举 `I18nKey`，`t!` 宏接受枚举变体，编译期排除拼写错误。

**验收**：debug build 中拼错 key 立即报错。

---

### 7. JWT 解析加固

**问题**：`extract_uid` 内部 `?` 静默失败，无日志。

**修复**：保持 `Option<String>` 返回，但在每个 `?` 失败点加 `log::warn!`，方便排查 token 格式变更。

**验收**：token 异常时日志中有明确提示。

---

## P2 — 性能优化

### 8. 图片处理移入 spawn_blocking

**问题**：`image::load_from_memory` + `resize` 是 CPU 密集操作，在 `tokio::spawn` 中会阻塞 worker 线程。

**修复**：
```rust
let (rgb, w, h) = tokio::task::spawn_blocking(move || {
    let img = image::load_from_memory(&bytes)?;
    let resized = img.resize(200, 200, image::imageops::FilterType::Triangle);
    let rgb = resized.to_rgb8();
    Ok::<_, anyhow::Error>((rgb.to_vec(), resized.width(), resized.height()))
}).await??;
```

**验收**：快速切换歌曲时 UI 不卡顿。

---

### 9. 搜索防抖

**问题**：每次按键都可能触发 3 个并行 API 请求。

**修复**：复用封面的防抖模式，新增 `pending_search: Option<(String, Instant)>`，300ms 内无新输入才发请求。或改为仅 Enter 触发搜索。

**验收**：快速输入时网络请求数量显著减少。

---

### 10. 减少大对象 Clone

**问题**：`AppMessage` 变体携带完整 `Vec<PublicSongDetail>` 跨 channel clone。

**修复**：
- `DataLoaded` 载荷改为 `Box<DataPayload>` 或 `Arc<...>`
- `AudioFetched` 中的 `data: Bytes` 已是引用计数，无需改
- 评估 `PublicSongDetail` 是否需要全部字段，考虑拆为 summary/full 两级

**验收**：profile 显示 clone 开销降低。

---

## P3 — 代码质量

### 11. 魔法数字集中管理

**问题**：render.rs、miller.rs 中散布 `1`、`2`、`50`、`15`、`45` 等布局数字。

**修复**：移入 `ui/constants.rs`：
```rust
pub const HEADER_HEIGHT: u16 = 1;
pub const PLAYER_BAR_HEIGHT: u16 = 2;
pub const MILLER_PARENT_PCT: u16 = 15;
pub const MILLER_CURRENT_PCT: u16 = 45;
pub const MILLER_PREVIEW_PCT: u16 = 40;
```

---

### 12. 事件处理拆分

**问题**：`event.rs` 有 3-4 层嵌套 match，可读性差。

**修复**：拆为独立函数：
```rust
fn handle_normal_key(&mut self, key: KeyEvent) -> bool { ... }
fn handle_search_key(&mut self, key: KeyEvent) -> bool { ... }
fn handle_login_key(&mut self, key: KeyEvent) -> bool { ... }
fn handle_overlay_key(&mut self, key: KeyEvent) -> bool { ... }
```
顶层 match 只做 mode 分发。

---

### 13. NavNode 数据驱动化

**问题**：`display_name()`、`children()` 各有 20+ match 分支。

**修复**：为静态节点建立描述表：
```rust
struct NavNodeMeta {
    display_key: &'static str,  // i18n key
    children: &'static [NavNode],
}
```
动态节点仍用 match，但分支数大幅减少。

---

### 14. 清理 dead_code 和 TODO

**修复**：
- 移除所有 `#[allow(dead_code)]`，未实现的功能用 `todo!()` 或直接删除
- 汇总未完成功能到本文档的附录

---

## 执行顺序建议

```
第一轮（基础拆分，不改行为）：
  #1 actions.rs 拆分 → #2 miller.rs 拆分 → #3 App 字段聚合

第二轮（正确性）：
  #4 CoverCache 泄漏 → #5 Queue 边界 → #7 JWT 日志

第三轮（性能 + 质量）：
  #8 spawn_blocking → #9 搜索防抖 → #11 常量 → #12 事件拆分

按需：
  #6 i18n 枚举化 → #10 减少 Clone → #13 NavNode 数据化 → #14 清理
```

每轮完成后 `cargo build` + 手动测试基本流程（登录、搜索、播放、切歌、封面加载）确保无回归。
