use std::cell::RefCell;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::Widget,
};

// 每帧 draw 期间收集到的待放置封面，draw 结束后由主循环消费并写入 stdout
thread_local! {
    pub static PENDING_PLACEMENTS: RefCell<Vec<(u32, Rect)>> = RefCell::new(Vec::new());
}

pub struct CoverWidget {
    pub image_id: u32,
}

impl Widget for CoverWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 记录放置位置，留给 draw 结束后由主循环写入终端
        PENDING_PLACEMENTS.with(|p| p.borrow_mut().push((self.image_id, area)));
        // 将该区域设为空格，让 ratatui buffer 拥有这些格子（以便内容切换时正确清除）
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.reset();
                }
            }
        }
    }
}
