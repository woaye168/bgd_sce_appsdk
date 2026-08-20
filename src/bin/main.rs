//! bgd_appsdk CLI 脚手架：快速创建标准 sce_app_* 应用项目。
//!
//! 用法：
//!   bgd_appsdk new <app-id> [--name <中文名>] [--dir <父目录>]
//!
//! 生成：标准应用骨架（Cargo.toml（bgd_appsdk = "0.2"）/ src/main.rs（入口 + 应用状态 +
//! ShellApp 壳实现，ui_tab 只做分发）/ src/ui/（mod.rs + 示例页面文件，impl 分散定义）/
//! app.json / release.yml / AGENTS.md），可直接 cargo run 起壳。

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) != Some("new") {
        eprintln!("用法: bgd_appsdk new <app-id> [--name <中文名>] [--dir <父目录>]");
        std::process::exit(2);
    }
    let app_id = match args.get(1) {
        Some(s) if !s.starts_with("--") => s.clone(),
        _ => {
            eprintln!("缺少 <app-id>（如 my-tool）");
            std::process::exit(2);
        }
    };
    let opt = |key: &str| -> Option<String> {
        args.windows(2).find(|w| w[0] == key).map(|w| w[1].clone())
    };
    let name = opt("--name").unwrap_or_else(|| app_id.clone());
    let parent = opt("--dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = parent.join(format!("sce_app_{app_id}"));
    let exe_name = format!("sce_app_{app_id}");

    match scaffold(&dir, &app_id, &name, &exe_name) {
        Ok(()) => {
            println!("已创建 {}/", dir.display());
            println!("下一步：cd {} && cargo run", dir.display());
        }
        Err(e) => {
            eprintln!("创建失败: {e}");
            std::process::exit(1);
        }
    }
}

fn scaffold(dir: &Path, app_id: &str, name: &str, exe_name: &str) -> Result<(), String> {
    if dir.exists() {
        return Err(format!("目录已存在: {}", dir.display()));
    }
    let w = |rel: &str, content: &str| -> Result<(), String> {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).map_err(|e| format!("创建目录失败: {e}"))?;
        fs::write(&p, content).map_err(|e| format!("写入 {} 失败: {e}", p.display()))
    };

    w("Cargo.toml", &format!(r#"[package]
name = "sce_app_{app_id}"
version = "0.0.0-dev"
edition = "2021"
description = "{name}"
license = "AGPL-3.0"

[[bin]]
name = "{exe_name}"
path = "src/main.rs"

[dependencies]
eframe = "0.29"
egui = "0.29"
bgd_appsdk = "0.2"

[profile.release]
opt-level = 2
strip = true
"#))?;

    w("src/main.rs", &format!(r#"//! {name}（sce_app_{app_id}）：基于 bgd_appsdk 的标准应用骨架
//!
//! 本文件为入口聚合：应用状态 + ShellApp 壳实现（ui_tab 只做分发）；
//! 标签页 UI 分散在 src/ui/ 各页面文件（impl App）。
//!
//! 公共逻辑（CLI 分发 --quit/notify、单实例、看守线程、--background、项目解析、窗口壳）
//! 由 bgd_appsdk::app::run 全托管——业务只需实现 ShellApp（标签页渲染）。

// Windows 下不弹出黑色控制台窗口
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod ui;

use std::path::PathBuf;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_NAME: &str = "{name}";

fn main() -> eframe::Result<()> {{
    bgd_appsdk::app::run(
        bgd_appsdk::app::AppOptions {{
            app_name: APP_NAME,
            inner_size: [720.0, 560.0],
            min_size: [600.0, 480.0],
            // 单实例/信号前缀一律由 appsdk 按 exe 名推导，禁止硬编码
            si_prefix: None,
            is_valid_project: Some(|p| p.join(".bgd").is_dir()),
            app: App::default(),
        }},
        APP_VERSION,
    )
}}

/// 应用状态（壳只负责框架，业务状态都放这里）
#[derive(Default)]
struct App {{
    /// 当前项目根（on_project_changed 回调维护）
    project_root: Option<PathBuf>,
    /// 状态栏文本
    status: String,
}}

const TABS: &[bgd_appsdk::ui::ShellTab] = &[
    bgd_appsdk::ui::ShellTab {{ id: "main", label: "主页" }},
    bgd_appsdk::ui::ShellTab {{ id: "settings", label: "设置" }},
];

impl bgd_appsdk::ui::ShellApp for App {{
    fn app_title(&self) -> &'static str {{
        APP_NAME
    }}

    fn tabs(&self) -> &[bgd_appsdk::ui::ShellTab] {{
        TABS
    }}

    fn ui_tab(&mut self, ui: &mut egui::Ui, tab: &str) {{
        match tab {{
            "main" => self.ui_main(ui),
            "settings" => self.ui_settings(ui),
            _ => {{}}
        }}
    }}

    fn on_project_changed(&mut self, project: Option<&std::path::Path>) {{
        self.project_root = project.map(|p| p.to_path_buf());
        if let Some(p) = project {{
            self.status = format!("当前项目: {{}}", p.display());
        }}
    }}

    fn status_text(&self) -> String {{
        self.status.clone()
    }}
}}
"#))?;

    w("src/ui/mod.rs", r#"//! 业务 UI 按标签页拆分：每个页面文件里 `impl App` 定义对应渲染函数，
//! main.rs 的 `ShellApp::ui_tab` 只做分发。
//! 新增页面 = 本目录加文件 + 此处 mod 声明 + main.rs 的 TABS / ui_tab 分发各加一行。
mod main_page;
mod settings;
"#)?;

    w("src/ui/main_page.rs", &format!(r#"//! 主页标签页

use crate::App;

impl App {{
    pub(crate) fn ui_main(&mut self, ui: &mut egui::Ui) {{
        ui.heading("{name}");
        ui.label("基于 bgd_appsdk 的标准应用骨架。在这里实现你的功能。");
        ui.add_space(8.0);
        if ui.button("点我").clicked() {{
            self.status = "按钮被点击了".to_string();
        }}
    }}
}}
"#))?;

    w("src/ui/settings.rs", r#"//! 设置标签页

use crate::App;

impl App {
    pub(crate) fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("设置");
        ui.label("在这里实现设置项。");
        if let Some(p) = &self.project_root {
            ui.add_space(8.0);
            ui.label(format!("当前项目：{}", p.display()));
        }
    }
}
"#)?;

    w("app.json", &format!(r#"{{
  "id": "{app_id}",
  "name": "{name}",
  "description": "{name}",
  "author": "BGD",
  "asset_name": "{exe_name}.exe"
}}
"#))?;

    w(".github/workflows/release.yml", &format!(r#"name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  release:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0

      - uses: dtolnay/rust-toolchain@stable

      - name: Inject version from tag
        shell: pwsh
        run: |
          $ver = "${{{{ github.ref_name }}}}".TrimStart('v')
          $cargo = Get-Content Cargo.toml -Raw
          $cargo = $cargo -replace '(?m)^version = "[^"]*"', "version = `"$ver`""
          Set-Content Cargo.toml $cargo -NoNewline -Encoding utf8
          $notes = git log --pretty=format:"- %s (``%h``)" -n 20 --no-merges | Out-String
          $notes | Set-Content release_notes.md -Encoding utf8

      - name: Build release
        run: cargo build --release

      - name: Generate app-release.json
        shell: pwsh
        run: |
          $meta = Get-Content app.json -Raw -Encoding utf8 | ConvertFrom-Json
          $meta | Add-Member -NotePropertyName version -NotePropertyValue ("${{{{ github.ref_name }}}}".TrimStart('v'))
          $meta | Add-Member -NotePropertyName release_notes -NotePropertyValue (Get-Content release_notes.md -Raw -Encoding utf8)
          $meta | ConvertTo-Json -Depth 5 | Set-Content app-release.json -Encoding utf8

      - name: Upload Release
        uses: softprops/action-gh-release@v2
        env:
          GITHUB_TOKEN: ${{{{ secrets.GITHUB_TOKEN }}}}
        with:
          tag_name: ${{{{ github.ref_name }}}}
          name: sce_app_{app_id} ${{{{ github.ref_name }}}}
          files: |
            target/release/{exe_name}.exe
            app-release.json
"#))?;

    w("AGENTS.md", &format!(r#"# AGENTS.md — sce_app_{app_id}

> 本文件为 AI 助手提供项目上下文。修改代码前请先阅读。

## 项目定位

{name}（sce_app_{app_id}）：独立的 egui 桌面应用。通过宿主 [bgd_sce_tools](https://github.com/woaye168/bgd_sce_tools) 的「应用市场」安装分发，宿主启动时传 `--project-path <项目根>`。应用单实例；`--background` 静默驻留；`--quit` 优雅退出；窗口 X = 正常退出。

## 技术栈与规范

- Rust 2021；eframe/egui 0.29；CLI 由 bgd_appsdk 统一入口托管（`--project-path` / `--background` / `--quit` / `notify`），本仓库不引入 clap
- **bgd_appsdk**（crates.io 公开包 `bgd_appsdk = "0.2"`，仓库 [bgd_sce_appsdk](https://github.com/woaye168/bgd_sce_appsdk)）：单实例/看守线程/日志/应用配置/**通用窗口壳 AppShell** 等公共基建，禁止在本仓库重复实现（UI 经 `ShellApp` trait 注册标签页即可）
- **模块拆分**：单文件接近 500 行必须按职责拆分

## 目录结构

```
src/main.rs            # 入口（bgd_appsdk::app::run 统一入口）+ 应用状态 + ShellApp 壳实现（ui_tab 只做分发）
src/ui/mod.rs          # 页面模块声明
src/ui/main_page.rs    # 主页标签页（impl App 分散定义）
src/ui/settings.rs     # 设置标签页
app.json               # 应用市场静态元数据（不含版本；CI 合成 app-release.json）
.github/workflows/release.yml  # tag 触发构建发布
```

## 使用方约定（改代码前必读）

- 应用只需实现 `ShellApp` 并调 `bgd_appsdk::app::run`——公共逻辑（CLI 分发、单实例、看守线程、项目解析、窗口壳）全托管，禁止自己再写一套。
- 新增标签页 = `src/ui/` 加页面文件（`impl App` 定义 `ui_xxx`）+ `ui/mod.rs` 加 mod 声明 + main.rs 的 `TABS` / `ui_tab` 分发各加一行。
- 宿主协议：`--background` 静默驻留、`--quit` 优雅退出、`notify key=value` 解耦通知（切项目时宿主会发 `notify project_path=<路径>`，壳自动刷新并回调 `on_project_changed`）。
- **命名契约**：宿主按 `<id>.exe` 落盘，单实例/信号前缀一律由 appsdk 按 exe 名推导（`app::default_si_prefix`），应用方禁止硬编码（`AppOptions.si_prefix` 保持 `None`）。
- **关键结论**：egui 窗口隐藏时事件循环休眠，任何信号处理不能放 UI update，也不能依赖 ViewportCommand——这类需求一律提到 bgd_appsdk 看守线程里实现。

## 构建与发布

```bash
cargo build --release
git tag v0.x.0 && git push origin v0.x.0   # CI 注入版本号 → 构建 → 上传 exe + app-release.json
```

- 版本号唯一来源是 git tag（Cargo.toml 固定 `0.0.0-dev`，CI 构建时注入）。
- **本应用无自我更新**：版本更新统一由宿主 bgd_sce_tools 应用市场负责（registry 在 bgd_sce_appsdk，元数据来自本仓库 CI 合成的 app-release.json）。

## 修改守则

- 公共基建（单实例/看守线程/日志/配置/窗口壳）禁止在本仓库重复实现；缺能力先改 bgd_appsdk 并升版本。
- 单文件接近 500 行必须按职责拆分（页面进 `src/ui/`，非 UI 逻辑进 `src/core/` 之类按职责建立的目录）。
- 提交规范：Conventional Commits（`feat: / fix: / docs: / ci: / refactor: / chore:` 前缀，Release notes 依赖）。
"#))?;

    Ok(())
}
