//! 应用配置持久化：exe 旁的 `<exe文件名去掉exe>.config.json`（最近项目等公共字段）。
//! 约定：MCP/CLI 解析项目路径时缺省取最近项目；宿主 notify 切换项目时更新。

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// 配置文件路径（exe 旁 <app>.config.json）
fn config_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.with_extension("config.json"))
}

/// 读整个配置 JSON（不存在返回空对象）
pub fn read() -> Value {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}))
}

/// 写整个配置 JSON
pub fn write(cfg: &Value) {
    if let Some(p) = config_path() {
        let _ = std::fs::write(&p, serde_json::to_string_pretty(cfg).unwrap_or_default());
    }
}

/// 最近项目路径
pub fn last_project_path() -> Option<PathBuf> {
    read()["last_project_path"]
        .as_str()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
}

/// 记录最近项目路径（统一正斜杠）
pub fn set_last_project_path(project_root: &Path) {
    let mut cfg = read();
    cfg["last_project_path"] = Value::String(project_root.display().to_string().replace('\\', "/"));
    write(&cfg);
}

/// 对外输出路径统一正斜杠
pub fn to_slash(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
