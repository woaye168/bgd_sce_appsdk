//! 看守线程：独立等待 唤起/退出/刷新 信号并直接 Win32 驱动主窗口。
//! 关键结论（实测）：egui 在窗口隐藏时事件循环休眠，放在 UI update 里的信号轮询与
//! ViewportCommand 都不可靠；QQ/微信式后台驻留的标准做法是看守线程直接 Win32
//! ShowWindow/SW_HIDE（QQ/微信/Everything 同款）。

#[cfg(windows)]
mod imp {
    use super::super::single_instance::{find_current_process_window, Guard};
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows_sys::Win32::Foundation::{HWND, WPARAM, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindow, SW_HIDE, SW_RESTORE,
        SW_SHOW, SW_SHOWNA, WM_CLOSE,
    };

    static QUIT_REQUESTED: AtomicBool = AtomicBool::new(false);
    static REFRESH_REQUESTED: AtomicBool = AtomicBool::new(false);

    /// 取出退出请求（主循环轮询；取一次自动清零）
    pub fn take_quit() -> bool {
        QUIT_REQUESTED.swap(false, Ordering::SeqCst)
    }

    /// 取出刷新请求（主循环轮询；取一次自动清零）
    pub fn take_refresh() -> bool {
        REFRESH_REQUESTED.swap(false, Ordering::SeqCst)
    }

    /// 启动看守线程。`background=true`（静默自启）时：主循环的 show 分支对所有 show 信号
    /// 一律 SW_HIDE（信号在窗口创建前发出时，后续 wait_show 的超时循环也会在 hwnd 出现后立刻隐藏）；
    /// 普通模式 show = SW_RESTORE/SW_SHOW/SetForegroundWindow。guard 移入线程持有。
    pub fn spawn(guard: Guard, background: bool, prefix: &str) {
        // 静默自启：无需再发 self show 信号——窗口起步即不可见（with_visible(false)，
        // 彻底消除「先弹后隐」闪烁）；唤起信号由二次启动/宿主「打开」时发出
        let _ = prefix;
        std::thread::spawn(move || {
            let guard = guard;
            let mut hwnd: HWND = std::ptr::null_mut();
            loop {
                hwnd = find_current_process_window();
                if guard.wait_show(200) {
                    // show 信号 = 唤起窗口（二次启动/宿主「打开」）。background 模式下用于隐藏。
                    unsafe {
                        if !hwnd.is_null() {
                            if background {
                                ShowWindow(hwnd, SW_HIDE);
                            } else {
                                ShowWindow(hwnd, SW_RESTORE);
                                ShowWindow(hwnd, SW_SHOW);
                                SetForegroundWindow(hwnd);
                            }
                        }
                    }
                }
                // background 静默自启兜底：窗口晚于信号创建时，周期循环也会立刻隐藏
                if background && !hwnd.is_null() && unsafe { IsWindowVisible(hwnd) } != 0 {
                    unsafe {
                        ShowWindow(hwnd, SW_HIDE);
                    }
                }
                if guard.wait_quit(100) {
                    // 退出：先置标志给 UI 主循环一次正常关闭的机会（可见窗口会走 egui
                    // 正常退出）；隐藏驻留时 UI 循环挂起不可靠——短等待后直接退出进程
                    // （应用无未保存状态，宿主升级/联动停止场景）
                    QUIT_REQUESTED.store(true, Ordering::SeqCst);
                    unsafe {
                        if !hwnd.is_null() {
                            PostMessageW(hwnd, WM_CLOSE, 0 as WPARAM, 0 as LPARAM);
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    std::process::exit(0);
                }
                if guard.wait_refresh(50) {
                    REFRESH_REQUESTED.store(true, Ordering::SeqCst);
                    // 隐藏驻留时 UI 循环挂起不 tick——用 显示→隐藏 脉冲强制唤醒一帧
                    // （SW_SHOWNA 不激活窗口，用户无感知），让应用主循环消费刷新标志
                    unsafe {
                        if !hwnd.is_null() && IsWindowVisible(hwnd) == 0 {
                            ShowWindow(hwnd, SW_SHOWNA);
                            std::thread::sleep(std::time::Duration::from_millis(150));
                            ShowWindow(hwnd, SW_HIDE);
                        }
                    }
                }
            }
        });
    }
}

#[cfg(windows)]
pub use imp::*;

#[cfg(not(windows))]
mod stub {
    pub fn take_quit() -> bool {
        false
    }
    pub fn take_refresh() -> bool {
        false
    }
}

#[cfg(not(windows))]
pub use stub::*;
