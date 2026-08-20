//! 应用通用窗口壳（bgd_appsdk 统一实现）：
//! 中文字体 / 标题尺寸约定 / 顶部项目栏（当前项目 + 选择按钮）/ 选项卡栏 / 底部状态栏 /
//! 单实例接入 / 看守线程退出与刷新标志轮询。
//! 应用只需实现 [`ShellApp`]：注册标签页、渲染各标签内容、处理项目变化。

use eframe::egui;

/// 应用标签定义（id 稳定用于当前选中态）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ShellTab {
    pub id: &'static str,
    pub label: &'static str,
}

/// 应用侧实现：壳负责框架，业务只关心标签与内容。
pub trait ShellApp {
    /// 窗口标题（不含版本号，壳自动拼 ` v{version}`）
    fn app_title(&self) -> &'static str;
    /// 标签页列表（顺序即显示顺序）
    fn tabs(&self) -> &[ShellTab];
    /// 渲染某个标签页内容（中央面板内）
    fn ui_tab(&mut self, ui: &mut egui::Ui, tab: &str);
    /// 项目变化回调（初始加载与用户切换都会触发；无项目时为 None）
    fn on_project_changed(&mut self, project: Option<&std::path::Path>);
    /// 状态栏文本（默认空）
    fn status_text(&self) -> String {
        String::new()
    }
    /// 当前项目（壳的项目栏展示；默认 None，应用可用自己状态覆盖）
    fn current_project(&self) -> Option<std::path::PathBuf> {
        None
    }
}

/// 应用窗口壳
pub struct AppShell<A: ShellApp> {
    app: A,
    version: &'static str,
    tabs: Vec<ShellTab>,
    active: String,
    project: Option<std::path::PathBuf>,
}

impl<A: ShellApp> AppShell<A> {
    /// 创建壳。`initial_project` 为 --project-path 传入的初始项目（无效则为 None）。
    pub fn new(app: A, version: &'static str, initial_project: Option<std::path::PathBuf>) -> Self {
        let tabs = app.tabs().to_vec();
        let active = tabs.first().map(|t| t.id.to_string()).unwrap_or_default();
        let mut shell = Self {
            app,
            version,
            tabs,
            active,
            project: None,
        };
        if let Some(p) = initial_project {
            shell.set_project(Some(p));
        }
        shell
    }

    fn set_project(&mut self, project: Option<std::path::PathBuf>) {
        self.project = project.clone();
        if let Some(p) = &project {
            crate::config::set_last_project_path(p);
        }
        self.app.on_project_changed(project.as_deref());
    }

    /// 运行窗口（标题/尺寸约定 + 中文字体）。
    /// `background=true`（静默自启）时：窗口正常创建，隐藏由看守线程确定性处理——
    /// 找到本进程主窗口后连续多拍 SW_HIDE，覆盖 egui 初始化期间的重新显示
    /// （egui 的 with_visible(false) 起步不可靠，实测会重新显示一次）。
    pub fn run(mut self, inner_size: [f32; 2], min_size: [f32; 2], background: bool) -> eframe::Result<()> {
        let title = format!("{} v{}", self.app.app_title(), self.version);
        // 屏幕正中显示：按主屏（1920x1080 兜底）与窗口尺寸估算左上角（egui 默认定位偏左上）
        let (sw, sh) = (1920.0f32, 1080.0f32);
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title(title)
                .with_inner_size(inner_size)
                .with_min_inner_size(min_size)
                .with_position([((sw - inner_size[0]) / 2.0).max(0.0), ((sh - inner_size[1]) / 2.0).max(0.0)]),
            ..Default::default()
        };
        eframe::run_native(
            self.app.app_title(),
            options,
            Box::new(move |cc| {
                setup_chinese_font(&cc.egui_ctx);
                let _ = background; // 静默自启隐藏由看守线程确定性处理（信号在窗口创建前发出）
                Ok(Box::new(self))
            }),
        )
    }
}

impl<A: ShellApp> eframe::App for AppShell<A> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 退出请求（看守线程 --quit 置位）：正常关闭
        #[cfg(windows)]
        if crate::watcher::take_quit() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        // 刷新请求（宿主 notify 切换项目）：重新加载最近项目
        #[cfg(windows)]
        if crate::watcher::take_refresh() {
            if let Some(p) = crate::config::last_project_path() {
                if self.project.as_ref() != Some(&p) {
                    self.set_project(Some(p));
                }
            }
        }
        // 周期唤醒（隐藏驻留时保持 update 触发）
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // 顶部：项目栏 + 选项卡
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("项目：");
                match &self.project {
                    Some(p) => {
                        ui.monospace(crate::config::to_slash(p));
                    }
                    None => {
                        ui.label("（未选择）");
                    }
                }
                if ui.button("选择项目…").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        self.set_project(Some(dir));
                    }
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for t in &self.tabs {
                    if ui
                        .selectable_label(self.active == t.id, t.label)
                        .clicked()
                    {
                        self.active = t.id.to_string();
                    }
                }
            });
            ui.add_space(4.0);
        });

        // 底部：状态栏
        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.add_space(4.0);
            let text = self.app.status_text();
            if !text.is_empty() {
                ui.small(text);
            }
            ui.add_space(4.0);
        });

        // 中央：当前标签内容
        egui::CentralPanel::default().show(ctx, |ui| {
            let tab = self.active.clone();
            self.app.ui_tab(ui, &tab);
        });
    }
}

/// 加载系统中文字体（微软雅黑），egui 默认字体不含中文
pub fn setup_chinese_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for candidate in ["C:/Windows/Fonts/msyh.ttc", "C:/Windows/Fonts/simhei.ttf"] {
        if let Ok(data) = std::fs::read(candidate) {
            fonts
                .font_data
                .insert("chinese".to_string(), egui::FontData::from_owned(data));
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "chinese".to_string());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("chinese".to_string());
            break;
        }
    }
    ctx.set_fonts(fonts);
}
