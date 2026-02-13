use std::path::PathBuf;

use anyhow::{Context, Result};

pub fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("无法获取配置目录")?
        .join("hachimi-tui");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn cache_dir() -> Result<PathBuf> {
    let dir = dirs::cache_dir()
        .context("无法获取缓存目录")?
        .join("hachimi-tui");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn audio_cache_dir() -> Result<PathBuf> {
    let dir = cache_dir()?.join("audio");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn config_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn auth_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("auth.json"))
}

pub fn queue_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("queue.json"))
}
