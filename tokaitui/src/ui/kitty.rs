/// 检测终端是否支持 Kitty 图形协议
pub fn is_supported() -> bool {
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return true;
    }
    if let Ok(val) = std::env::var("TERM_PROGRAM") {
        if val == "ghostty" || val == "kitty" {
            return true;
        }
    }
    false
}

/// 生成上传 raw RGB 图片的 APC 序列（分块 base64，f=24，a=T 纯上传，q=2 静默）
pub fn upload_rgb(id: u32, rgb: &[u8], w: u32, h: u32) -> Vec<u8> {
    use base64::Engine;
    const CHUNK_SIZE: usize = 4096;

    let encoded = base64::engine::general_purpose::STANDARD.encode(rgb);
    let chunks: Vec<&[u8]> = encoded.as_bytes().chunks(CHUNK_SIZE).collect();
    let total = chunks.len();
    let mut out = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == total - 1;
        let m = if is_last { 0 } else { 1 };
        let chunk_str = std::str::from_utf8(chunk).unwrap_or("");
        if i == 0 {
            let header = format!(
                "\x1b_Ga=t,f=24,s={w},v={h},i={id},q=2,m={m};{chunk_str}\x1b\\"
            );
            out.extend_from_slice(header.as_bytes());
        } else {
            let cont = format!("\x1b_Gm={m};{chunk_str}\x1b\\");
            out.extend_from_slice(cont.as_bytes());
        }
    }
    out
}

/// 生成在当前光标位置放置图片的序列（需调用方先移动光标到目标位置）
/// c = 列数, r = 行数（字符单元格数）
pub fn place_at_cursor(id: u32, cols: u16, rows: u16) -> Vec<u8> {
    format!("\x1b_Ga=p,i={id},c={cols},r={rows},q=2;\x1b\\").into_bytes()
}

/// 删除图片的所有 placement（d=i 小写：保留 image data，可再次 place）
/// 用于帧间清理，避免 image data 被意外释放
pub fn delete_placement(id: u32) -> Vec<u8> {
    format!("\x1b_Ga=d,d=i,i={id},q=2;\x1b\\").into_bytes()
}

/// 完全删除图片（d=I 大写：同时释放 image data）
/// 用于内存淘汰（超过 10 张时）
pub fn delete_image(id: u32) -> Vec<u8> {
    format!("\x1b_Ga=d,d=I,i={id},q=2;\x1b\\").into_bytes()
}
