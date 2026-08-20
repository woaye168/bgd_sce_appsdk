//! 应用日志：按日期分文件（防无限增大），失败静默不影响主流程。
//! 路径优先级：<项目>/.bgd/log/<app>-YYYY-MM-DD.log → <编辑器根>/bgd_editor_patch/log/ → <exe目录>/log/

use std::fs;
use std::path::{Path, PathBuf};

/// 日志文件路径（按当天日期）
pub fn log_path(app_name: &str, project_root: Option<&Path>, editor_root: Option<&Path>) -> PathBuf {
    let file = format!("{app_name}-{}.log", today());
    if let Some(project) = project_root {
        let bgd = project.join(".bgd");
        if bgd.is_dir() {
            return bgd.join("log").join(file);
        }
    }
    if let Some(root) = editor_root {
        return root.join("bgd_editor_patch").join("log").join(file);
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    exe_dir.join("log").join(file)
}

/// 追加一行日志（失败静默）
pub fn log(app_name: &str, project_root: Option<&Path>, editor_root: Option<&Path>, level: &str, message: &str) {
    let path = log_path(app_name, project_root, editor_root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64
        + 8 * 3600;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    let line = format!(
        "[{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}] [{level}] {message}\n",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    );
    use std::io::Write;
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// 当天日期（YYYY-MM-DD，UTC+8）
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64
        + 8 * 3600;
    let (y, m, d) = civil_from_days(secs.div_euclid(86400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// days-from-unix-epoch → 年月日（Howard Hinnant 算法）
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_today_format() {
        let t = today();
        assert_eq!(t.len(), 10);
        assert!(t.starts_with("20"));
    }
}
