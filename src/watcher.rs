//! 看守线程：独立等待 唤起/退出/刷新 信号并直接 Win32 驱动主窗口。
//! 关键结论（实测）：egui 在窗口隐藏时事件循环休眠，放在 UI update 里的信号轮询与
//! ViewportCommand 都不可靠；QQ/微信式后台驻留的标准做法是看守线程直接 Win32
//! ShowWindow/SW_HIDE（QQ/微信/Everything 同款）。

#[cfg(windows)]
mod imp {
    use super::super::single_instance::{find_current_process_window, Guard};
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetForegroundWindow, ShowWindow, SW_HIDE, SW_RESTORE, SW_SHOW,
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

    /// 启动看守线程。`background=true` 时找到主窗口后立刻 Win32 隐藏（静默自启）。
    /// guard 移入线程持有（Drop 会释放单实例互斥体，不能提前析构）。
    pub fn spawn(guard: Guard, background: bool) {
        std::thread::spawn(move || {
            let guard = guard;
            let mut hwnd: HWND = std::ptr::null_mut();
            // 轮询等待主窗口创建完成
            for _ in 0..50 {
                hwnd = find_current_process_window();
                if !hwnd.is_null() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            if background && !hwnd.is_null() {
                unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                }
            }
            let show = |hwnd: HWND| unsafe {
                if !hwnd.is_null() {
                    ShowWindow(hwnd, SW_RESTORE);
                    ShowWindow(hwnd, SW_SHOW);
                    SetForegroundWindow(hwnd);
                }
            };
            loop {
                if guard.wait_show(200) {
                    show(hwnd);
                }
                if guard.wait_quit(100) {
                    QUIT_REQUESTED.store(true, Ordering::SeqCst);
                }
                if guard.wait_refresh(50) {
                    REFRESH_REQUESTED.store(true, Ordering::SeqCst);
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
