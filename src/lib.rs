//! BGD SCE 应用公共 SDK（bgd_appsdk）
//!
//! 为 sce_app_* 应用提供公共基建，避免各应用重复实现：
//! - `single_instance`：命名互斥体单实例 + 唤起/退出/刷新命名事件 + 本进程主窗口查找（Win32）
//! - `watcher`：看守线程（Win32 驱动主窗口隐藏/唤起；退出/刷新标志置位）
//! - `log`：按日期分文件的应用日志
//! - `config`：应用配置持久化（最近项目等，exe 旁 JSON）
//!
//! 用法要点（各应用约定）：
//! - GUI 路径启动时 `single_instance::acquire()`；已存在实例则退出（唤起信号已发）
//! - `watcher::spawn(guard, background)` 后，在 UI 主循环轮询 `watcher::take_quit()` /
//!   `watcher::take_refresh()` 处理退出与刷新
//! - CLI 子命令（mcp/editor/logs/capture/notify 等短进程）应在 acquire 之前分发，不受单实例限制

pub mod config;
pub mod log;
pub mod single_instance;
pub mod watcher;
