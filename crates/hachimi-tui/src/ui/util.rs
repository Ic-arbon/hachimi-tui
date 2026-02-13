/// 最大公约数
pub fn gcd(a: u16, b: u16) -> u16 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// 计算像素精确对齐的视觉正方形 cell 尺寸。
///
/// 返回 `(width, height)` 满足 `width * fw == height * fh`（像素完全正方形），
/// 且 `width <= max_w`, `height <= max_h`。
pub fn square_cells(max_w: u16, max_h: u16, fw: u16, fh: u16) -> (u16, u16) {
    let g = gcd(fw, fh);
    let step_w = fh / g; // width 步进
    let step_h = fw / g; // height 步进

    // 从 height 推导
    let h = (max_h / step_h) * step_h;
    if h > 0 {
        let w = h / step_h * step_w;
        if w <= max_w {
            return (w, h);
        }
    }

    // 从 width 推导
    let w = (max_w / step_w) * step_w;
    if w > 0 {
        let h = w / step_w * step_h;
        if h <= max_h {
            return (w, h);
        }
    }

    // 兜底：最小步进
    (step_w.min(max_w).max(1), step_h.min(max_h).max(1))
}
