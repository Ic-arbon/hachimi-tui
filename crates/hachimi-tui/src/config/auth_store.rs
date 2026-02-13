use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthData {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

impl AuthData {
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now >= self.expires_at
    }
}

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

pub fn clear() -> Result<()> {
    let path = paths::auth_file()?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
