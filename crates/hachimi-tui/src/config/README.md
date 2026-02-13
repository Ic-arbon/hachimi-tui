# config — 配置与持久化

管理应用配置文件、认证凭据的存储与读取。配置目录为 `~/.config/hachimi-tui/`。

## 文件索引

| 文件 | 职责 |
|------|------|
| `settings.rs` | `Settings` 结构体（TOML 序列化）：播放器设置（音量、播放模式）、缓存设置（大小上限）、显示设置（语言、儿童模式）；`load()`/`save()` 读写 `config.toml` |
| `auth_store.rs` | `AuthData` 凭据管理：access_token/refresh_token 的持久化（`auth.json`，Unix 权限 600）；JWT payload 解析提取 uid；token 过期判断 |
| `paths.rs` | 路径工具函数：`config_dir()`、`cache_dir()`、各配置文件路径（`config.toml`、`auth.json`、`queue.json`）；自动创建目录 |

## 文件布局

```
~/.config/hachimi-tui/
├── config.toml    # 用户设置（settings.rs）
├── auth.json      # 认证凭据（auth_store.rs，权限 600）
└── queue.json     # 播放队列持久化（model/queue.rs 使用）
```
