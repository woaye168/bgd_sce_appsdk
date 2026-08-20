//! bgd_appsdk CLI 脚手架：快速创建标准 sce_app_* 应用项目。
//!
//! 用法：
//!   bgd_appsdk new <app-id> [--name <中文名>] [--dir <父目录>]
//!
//! 生成：标准应用骨架（Cargo.toml / src/main.rs（AppShell + 单实例 + --background/--quit/notify
//! 全接入的最小可运行示例）/ app.json / release.yml / AGENTS.md），可直接 cargo run 起壳。

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
clap = {{ version = "4", features = ["derive"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
rfd = "0.15"
bgd_appsdk = "0.1"

[profile.release]
opt-level = 2
strip = true
"#))?;

    w("src/main.rs", &format!(r#"//! {name}（sce_app_{app_id}）：基于 bgd_appsdk 的标准应用骨架
//!
//! 公共逻辑（CLI 分发/单实例/看守线程/--background/--quit/notify/窗口壳）由
//! bgd_appsdk::app::run 全托管——业务只需实现 ShellApp（标签页渲染）。

// Windows 下不弹出黑色控制台窗口
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_NAME: &str = "{name}";

fn main() -> eframe::Result<()> {{
    bgd_appsdk::app::run(
        bgd_appsdk::app::AppOptions {{
            app_name: APP_NAME,
            inner_size: [720.0, 560.0],
            min_size: [600.0, 480.0],
            si_prefix: None,
            is_valid_project: Some(|p| p.join(".bgd").is_dir()),
            app: App::default(),
        }},
        APP_VERSION,
    )
}}

const TABS: &[bgd_appsdk::ui::ShellTab] = &[
    bgd_appsdk::ui::ShellTab {{ id: "main", label: "主页" }},
    bgd_appsdk::ui::ShellTab {{ id: "settings", label: "设置" }},
];

#[derive(Default)]
struct App {{
    status: String,
}}

impl bgd_appsdk::ui::ShellApp for App {{
    fn app_title(&self) -> &'static str {{
        APP_NAME
    }}

    fn tabs(&self) -> &[bgd_appsdk::ui::ShellTab] {{
        TABS
    }}

    fn ui_tab(&mut self, ui: &mut egui::Ui, tab: &str) {{
        match tab {{
            "main" => {{
                ui.heading("{name}");
                ui.label("基于 bgd_appsdk 的标准应用骨架。在这里实现你的功能。");
                if ui.button("点我").clicked() {{
                    self.status = "按钮被点击了".to_string();
                }}
            }}
            "settings" => {{
                ui.heading("设置");
                ui.label("在这里实现设置项。");
            }}
            _ => {{}}
        }}
    }}

    fn on_project_changed(&mut self, project: Option<&std::path::Path>) {{
        if let Some(p) = project {{
            self.status = format!("当前项目: {{}}", p.display());
        }}
    }}

    fn status_text(&self) -> String {{
        self.status.clone()
    }}
}}
"#))?;

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

{name}（bgd_sce_tools 应用市场的独立应用）。基于 **bgd_appsdk**（crates.io 公开包，仓库 [bgd_sce_appsdk](https://github.com/woaye168/bgd_sce_appsdk)）：单实例/看守线程/日志/应用配置/通用窗口壳 AppShell 等公共基建，禁止在本仓库重复实现（UI 经 `ShellApp` trait 注册标签页即可）。

## 构建与发布

```bash
cargo build --release
git tag v0.x.0 && git push origin v0.x.0   # CI 出包（exe + app-release.json）
```

版本号唯一来源是 git tag（Cargo.toml 固定 `0.0.0-dev`）。bgd_appsdk 来自 crates.io 公开包，无需任何私有凭据。
"#))?;

    Ok(())
}
