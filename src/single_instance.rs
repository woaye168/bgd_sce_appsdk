//! 应用级单实例（命名互斥体）+ 唤起/退出/刷新命名事件 + 本进程主窗口查找。
//! 机制：CreateMutexW 判活；第二实例发「唤起窗口」事件后退出；
//! 宿主升级用「退出」事件优雅停止；宿主 notify 用「刷新」事件让 GUI 重新加载状态。

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
    use windows_sys::Win32::System::Threading::{
        CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject,
    };

    /// 事件名后缀（完整名 = `<prefix>_<suffix>`）
    pub const EV_SHOW: &str = "show";
    pub const EV_QUIT: &str = "quit";
    pub const EV_REFRESH: &str = "refresh";

    /// 单实例守卫（GUI 驻留期间持有；Drop 释放互斥体与事件）
    pub struct Guard {
        pub show_event: HANDLE,
        pub quit_event: HANDLE,
        pub refresh_event: HANDLE,
        _mutex: HANDLE,
    }
    // HANDLE 是内核对象句柄（值语义），跨线程 WaitForSingleObject 安全
    unsafe impl Send for Guard {}
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.show_event);
                CloseHandle(self.quit_event);
                CloseHandle(self.refresh_event);
                CloseHandle(self._mutex);
            }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn create_event(name: &str) -> HANDLE {
        let n = wide(name);
        unsafe { CreateEventW(std::ptr::null(), 0, 0, n.as_ptr()) }
    }

    fn set_event(name: &str) {
        let ev = create_event(name);
        if !ev.is_null() {
            unsafe {
                SetEvent(ev);
                CloseHandle(ev);
            }
        }
    }

    /// 获取单实例守卫；已存在实例时发送「唤起窗口」信号并返回 None（调用方应退出）。
    /// `prefix`：应用唯一前缀（如 "sce_app_editor-patch"）
    pub fn acquire(prefix: &str) -> Option<Guard> {
        unsafe {
            let name = wide(&format!("{prefix}_single"));
            let mutex = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
            if mutex.is_null() {
                return None;
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                set_event(&format!("{prefix}_{EV_SHOW}"));
                CloseHandle(mutex);
                return None;
            }
            Some(Guard {
                show_event: create_event(&format!("{prefix}_{EV_SHOW}")),
                quit_event: create_event(&format!("{prefix}_{EV_QUIT}")),
                refresh_event: create_event(&format!("{prefix}_{EV_REFRESH}")),
                _mutex: mutex,
            })
        }
    }

    /// 阻塞等待事件（毫秒超时轮询）
    fn wait(handle: HANDLE, ms: u32) -> bool {
        unsafe { WaitForSingleObject(handle, ms) == 0 }
    }

    impl Guard {
        pub fn wait_show(&self, ms: u32) -> bool {
            wait(self.show_event, ms)
        }
        pub fn wait_quit(&self, ms: u32) -> bool {
            wait(self.quit_event, ms)
        }
        pub fn wait_refresh(&self, ms: u32) -> bool {
            wait(self.refresh_event, ms)
        }
    }

    /// 向已运行实例发送「退出」信号（宿主升级/联动停止用）
    pub fn signal_quit(prefix: &str) {
        set_event(&format!("{prefix}_{EV_QUIT}"));
    }

    /// 向已运行实例发送「刷新」信号（宿主 notify 用）
    pub fn signal_refresh(prefix: &str) {
        set_event(&format!("{prefix}_{EV_REFRESH}"));
    }

    /// 自身发「唤起窗口」信号（静默自启用：通知本进程看守线程立刻隐藏窗口）
    pub fn signal_show_self(prefix: &str) {
        set_event(&format!("{prefix}_{EV_SHOW}"));
    }

    /// 精确获取本进程主窗口（PID 匹配 + 标题非空；egui 主窗口有标题，辅助窗口无标题）
    pub fn find_current_process_window() -> HANDLE {
        use windows_sys::Win32::Foundation::{HWND, LPARAM};
        use windows_sys::Win32::System::Threading::GetCurrentProcessId;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextW, GetWindowThreadProcessId,
        };
        struct Ctx {
            pid: u32,
            found: HWND,
        }
        unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> i32 {
            let ctx = &mut *(lparam as *mut Ctx);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == ctx.pid {
                let mut t = [0u16; 16];
                if GetWindowTextW(hwnd, t.as_mut_ptr(), t.len() as i32) > 0 {
                    ctx.found = hwnd;
                    return 0; // 找到即停
                }
            }
            1
        }
        let mut ctx = Ctx {
            pid: unsafe { GetCurrentProcessId() },
            found: std::ptr::null_mut(),
        };
        unsafe {
            EnumWindows(Some(cb), &mut ctx as *mut Ctx as LPARAM);
        }
        ctx.found
    }
}

#[cfg(windows)]
pub use imp::*;
