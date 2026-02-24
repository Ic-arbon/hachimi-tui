# 重构计划

## 概览

项目当前约 6,773 行 Rust 代码，架构基础良好但若干核心文件已膨胀。
本文档按优先级列出重构项，每项标注影响范围、具体步骤和验收标准。

---

## P0 — 架构拆分 ✅

### 1. actions.rs 拆分 ✅

已完成：`actions.rs`(1011行) 拆为 5 个子模块。

```
app/actions/
├── mod.rs          // 常量 SEARCH_PAGE_SIZE / HISTORY_PAGE_SIZE + mod 声明
├── auth.rs         // start_captcha, submit_login, logout, resume_playback
├── data.rs         // execute_search, load_node_data, maybe_load_preview_data, maybe_fetch_song_detail, maybe_fetch_queue_detail
├── playback.rs     // toggle_play_pause, play_next/prev, play_from_list, play_expanded_song, start_audio_fetch, 队列操作
├── navigation.rs   // nav_down/up/drill_in/out/top/bottom, after_nav_move, current_list_len
└── cover.rs        // schedule_cover_load, maybe_load_cover, fetch_danmaku, extract_bvid, do_fetch_danmaku
```

---

### 2. miller.rs 拆分 ✅

已完成：`miller.rs`(826行) 拆为 3 个文件。

```
ui/
├── miller.rs         // ColumnData + render() + render_column()
├── preview.rs        // render_preview_column + 各种详情预览 + apply_cover
└── format.rs         // song_list_line, marquee_text, truncate_with_dots
```

---

### 3. App 结构体字段聚合 ✅

已完成：新增 `UiState` 和 `CoverState` 结构体，10 个平铺字段收归 `ui.*` 和 `cover.*`。

```rust
pub struct UiState {
    pub input_mode: InputMode,
    pub show_help: bool,
    pub help_scroll: u16,
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

---

## P1 — 正确性修复

### 4. CoverCache 资源泄漏 ✅

已在之前的提交中修复：`evict_one()` 返回被驱逐的 id，调用方发送 `delete_image` 清理。

---

### 5. QueueState 边界安全 ✅

已确认安全：`current_index` 为 `Option<usize>`，所有访问使用 `.get()`。
移除了 `insert_next` 上的 `#[allow(dead_code)]` 标记。

---

### 6. 翻译 key 编译期校验

**问题**：`t!("key")` 拼错时运行时静默显示 `???`。

**修复方案 A**（推荐，改动小）：
在 debug build 中，`tr()` 函数对未找到的 key 加 `debug_assert!` 或 `eprintln!` 告警。

**修复方案 B**（彻底）：
将所有 key 改为枚举 `I18nKey`，`t!` 宏接受枚举变体，编译期排除拼写错误。

---

### 7. JWT 解析加固 ✅

已完成：`extract_uid_from_token` 返回 `Result<i64, String>`，每个失败点提供错误描述。
调用方使用 `match` + `eprintln!` 输出错误信息。

---

## P2 — 性能优化

### 8. 图片处理移入 spawn_blocking ✅

已在之前的提交中完成。

---

### 9. 搜索防抖 ✅

已确认：当前为 Enter 触发搜索，无需防抖。

---

### 10. 减少大对象 Clone

**问题**：`AppMessage` 变体携带完整 `Vec<PublicSongDetail>` 跨 channel clone。

**修复**：
- `DataLoaded` 载荷改为 `Box<DataPayload>` 或 `Arc<...>`
- 评估 `PublicSongDetail` 是否需要全部字段，考虑拆为 summary/full 两级

---

## P3 — 代码质量

### 11. 魔法数字集中管理 ✅

已完成：`constants.rs` 新增 `HEADER_HEIGHT`、`PLAYER_BAR_HEIGHT`、`SEARCH_BAR_HEIGHT`、`MILLER_*_PCT` 常量，替换 `render.rs` 和 `miller.rs` 中的硬编码数字。

---

### 12. 事件处理拆分 ✅

已完成：提取 `handle_overlay_key()` 方法处理帮助/日志浮层，简化 `handle_event` 为：
Ctrl+C → quit → handle_overlay_key → match input_mode。

---

### 13. NavNode 数据驱动化

**问题**：`display_name()`、`children()` 各有 20+ match 分支。

**修复**：为静态节点建立描述表，动态节点仍用 match，但分支数大幅减少。

---

### 14. 清理 dead_code 和 TODO

**修复**：
- 移除所有 `#[allow(dead_code)]`，未实现的功能用 `todo!()` 或直接删除
- 汇总未完成功能到本文档的附录

---

## 执行顺序建议

```
第一轮（基础拆分，不改行为）：          ✅ 全部完成
  #1 actions.rs 拆分 → #2 miller.rs 拆分 → #3 App 字段聚合

第二轮（正确性）：                      ✅ 全部完成
  #4 CoverCache 泄漏 → #5 Queue 边界 → #7 JWT 日志

第三轮（性能 + 质量）：                 ✅ 部分完成
  #8 spawn_blocking → #9 搜索防抖 → #11 常量 → #12 事件拆分

按需：
  #6 i18n 枚举化 → #10 减少 Clone → #13 NavNode 数据化 → #14 清理
```

每轮完成后 `cargo build` + 手动测试基本流程（登录、搜索、播放、切歌、封面加载）确保无回归。
