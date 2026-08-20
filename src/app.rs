//! 应用统一入口（bgd_appsdk 全托管）：
//! CLI 分发（--quit / notify project_path）→ 单实例 → 看守线程 → 项目路径解析 → AppShell。
//! 应用只需实现 [`ShellApp`]（业务标签页）并调用 [`run`]——公共逻辑零代码。

#[cfg(windows)]
use crate::single_instance;
use crate::ui::{AppShell, ShellApp};
use std::path::PathBuf;

/// 应用入口配置
pub struct AppOptions<A: ShellApp> {
    /// 窗口标题（不含版本号）
    pub app_name: &'static str,
    /// 初始/最小窗口尺寸
    pub inner_size: [f32; 2],
    pub min_size: [f32; 2],
    /// 单实例前缀（默认取 exe 文件名，sce_app_<id> 场景一般不用改）
    pub si_prefix: Option<&'static str>,
    /// 项目路径有效性校验（notify/--project-path 的过滤；默认仅判目录存在）
    pub is_valid_project: Option<fn(&std::path::Path) -> bool>,
    /// 应用业务实例（通常 `A::default()`）
    pub app: A,
}

/// 从原始参数中解析一个 flag 的值
fn arg_value(raw: &[String], key: &str) -> Option<String> {
    raw.windows(2).find(|w| w[0] == key).map(|w| w[1].clone())
}

fn has_flag(raw: &[String], key: &str) -> bool {
    raw.iter().any(|a| a == key)
}

/// 默认单实例前缀：当前 exe 文件名（sce_app_<id>）。
/// 命名契约：宿主按 `<id>.exe` 落盘，单实例/信号前缀一律由本函数推导，应用方禁止硬编码。
pub fn default_si_prefix() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

/// 应用统一入口（整个 main 函数体）。内部完成全部公共逻辑并启动窗口。
/// `version` 传 `env!("CARGO_PKG_VERSION")`。
pub fn run<A: ShellApp>(opts: AppOptions<A>, version: &'static str) -> eframe::Result<()> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let is_valid = opts.is_valid_project.unwrap_or(|p| p.is_dir());

    #[cfg(windows)]
    let prefix_owned = opts
        .si_prefix
        .map(str::to_string)
        .unwrap_or_else(default_si_prefix);
    #[cfg(windows)]
    let prefix: &str = &prefix_owned;

    // notify CLI（宿主解耦通知）：notify project_path=<项目根>
    if raw.first().map(|s| s.as_str()) == Some("notify") {
        for pair in raw.iter().skip(1) {
            if let Some(v) = pair.strip_prefix("project_path=") {
                let root = PathBuf::from(v);
                if is_valid(&root) {
                    crate::config::set_last_project_path(&root);
                    #[cfg(windows)]
                    single_instance::signal_refresh(prefix);
                }
            }
        }
        return Ok(());
    }

    // --quit：向已运行实例发「退出」信号后退出（宿主升级/联动停止用）
    #[cfg(windows)]
    if has_flag(&raw, "--quit") {
        single_instance::signal_quit(prefix);
        return Ok(());
    }

    // GUI 路径单实例：已运行则只发「唤起窗口」信号并退出
    #[cfg(windows)]
    let single_guard = match single_instance::acquire(prefix) {
        Some(g) => Some(g),
        None => return Ok(()),
    };

    // background 由主线程按「运行环境」决定：宿主静默自启传入 --background；
    // 用户显式启动（GUI 双击/宿主「打开」）不带该参数则前台运行
    let background = has_flag(&raw, "--background");

    // 项目路径：--project-path 传入且有效时作为初始项目
    let project_path = arg_value(&raw, "--project-path")
        .map(PathBuf::from)
        .filter(|p| is_valid(p));

    // 看守线程（信号在窗口创建前发出并保持置位；守护 唤起/退出/刷新 + 静默自启隐藏）
    #[cfg(windows)]
    if let Some(g) = single_guard {
        crate::watcher::spawn(g, background, prefix);
    }

    let shell = AppShell::new(opts.app, version, project_path);
    shell.run(opts.inner_size, opts.min_size, background)
}
