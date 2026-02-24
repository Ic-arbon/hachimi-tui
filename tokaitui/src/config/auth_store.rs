use anyhow::Result;

pub use mambocore::AuthData;

use super::paths;

pub fn load() -> Result<Option<AuthData>> {
    let path = paths::auth_file()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let data: AuthData = serde_json::from_str(&content)?;
    Ok(Some(data))
}

pub fn save(data: &AuthData) -> Result<()> {
    let path = paths::auth_file()?;
    let content = serde_json::to_string_pretty(data)?;
    std::fs::write(&path, content)?;

    // 设置文件权限为 600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// 从 JWT access token 的 payload 中提取 uid（sub 字段）
pub fn extract_uid_from_token(token: &str) -> Result<i64, String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("JWT 格式错误：期望 3 段，实际 {} 段", parts.len()));
    }
    let payload = base64url_decode(parts[1])
        .ok_or_else(|| "JWT payload base64 解码失败".to_string())?;
    let json: serde_json::Value = serde_json::from_slice(&payload)
        .map_err(|e| format!("JWT payload JSON 解析失败：{e}"))?;
    let sub = json.get("sub")
        .ok_or_else(|| "JWT payload 缺少 sub 字段".to_string())?
        .as_str()
        .ok_or_else(|| "JWT sub 字段不是字符串".to_string())?;
    sub.parse().map_err(|e| format!("JWT sub 无法解析为 i64：{e}"))
}

fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    let lookup = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'-' | b'+' => Some(62),
            b'_' | b'/' => Some(63),
            _ => None,
        }
    };

    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=').collect();
    let mut output = Vec::with_capacity(bytes.len() * 3 / 4);

    for chunk in bytes.chunks(4) {
        let mut buf = 0u32;
        let mut count = 0;
        for &b in chunk {
            if let Some(val) = lookup(b) {
                buf = (buf << 6) | val as u32;
                count += 1;
            }
        }
        match count {
            4 => {
                output.push((buf >> 16) as u8);
                output.push((buf >> 8) as u8);
                output.push(buf as u8);
            }
            3 => {
                buf <<= 6;
                output.push((buf >> 16) as u8);
                output.push((buf >> 8) as u8);
            }
            2 => {
                buf <<= 12;
                output.push((buf >> 16) as u8);
            }
            _ => return None,
        }
    }

    Some(output)
}

pub fn clear() -> Result<()> {
    let path = paths::auth_file()?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
