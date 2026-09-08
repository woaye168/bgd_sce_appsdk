# AGENTS.md — bgd_sce_appsdk

> 本文件面向在本仓库工作的 AI 编程代理。

## 项目定位

BGD SCE 应用生态的**公共 SDK 仓库** ，双职责：

1. **`registry.json`**：应用市场分发清单（宿主 [bgd_sce_tools](https://github.com/woaye168/bgd_sce_tools) 读取，列出应用 id/name/repo）。
2. **Rust crate `bgd_appsdk`**（`src/`）：sce_app_* 应用的公共基建，避免重复实现。

## SDK 内容（src/）

| 模块 | 内容 |
| --- | --- |
| `app` | **应用统一入口**：CLI 分发（--quit/notify）→ 单实例 → 看守线程 → 项目解析 → AppShell（应用 main 一行 `app::run(AppOptions{...})`，公共逻辑零代码） |
| `single_instance` | 命名互斥体单实例 + 唤起/退出/刷新命名事件 + 本进程主窗口查找（Win32） |
| `watcher` | 看守线程：等信号并 Win32 驱动主窗口（隐藏/唤起）；退出/刷新标志；--quit 不依赖 UI tick（exit 兜底） |
| `ui` | 通用窗口壳 AppShell：中文字体/标题尺寸约定/项目栏/选项卡/状态栏/窗口居中（应用实现 `ShellApp` trait 注册标签页） |
| `log` | 按日期分文件的应用日志（`<项目>/.bgd/log/<app>-YYYY-MM-DD.log`） |
| `config` | 应用配置持久化（exe 旁 `<app>.config.json`；最近项目；路径正斜杠统一） |

**使用方约定**：应用只需实现 `ShellApp` 并调 `bgd_appsdk::app::run`（公共逻辑全托管）。宿主侧协议：`--background` 静默驻留、`--quit` 优雅退出、`notify key=value` 解耦通知。**命名契约**：宿主按 `<id>.exe` 落盘，单实例/信号前缀一律由 appsdk 按 exe 名推导（`app::default_si_prefix`），应用方禁止硬编码。**驻留语义**：`--background` 启动的应用窗口 X = 隐藏（假关闭，服务常开；再次打开经宿主/二次启动的 show 信号唤出），真退出只走宿主 `--quit`；不带 `--background` 的前台启动窗口 X = 正常退出。宿主「打开」不做单开拦截——第二实例向运行中实例发 show 信号唤出窗口后自行退出。

**关键结论（改动前必读）**：egui 窗口隐藏时事件循环休眠，信号处理不能放 UI update，也不能依赖 ViewportCommand——必须看守线程直接 Win32（QQ/微信式驻留同款做法，实测定稿）。

## 消费方式

```toml
bgd_appsdk = "0.2"   # crates.io 公开包（tag 触发 publish.yml 自动发布）
```

CLI 脚手架：`cargo install bgd_appsdk` 后 `bgd_appsdk new <app-id> --name <中文名>` 生成标准应用骨架（`src/bin/main.rs` 模板）：

- `Cargo.toml`：依赖 `bgd_appsdk = "0.2"`（crates.io），不引入 clap
- `src/main.rs`：入口（`app::run` 一行起壳）+ 应用状态 + `ShellApp` 壳实现（`ui_tab` 只做分发）
- `src/ui/`：`mod.rs` + 示例页面文件（main_page/settings），页面 impl 分散定义
- `app.json` / `.github/workflows/release.yml`（tag 出包）/ `AGENTS.md`（AI 优化模板：项目定位/技术栈/目录结构/使用方约定/构建发布/修改守则）

## 构建与测试

```bash
cargo build
cargo test
```

## 现有使用方

- [sce_app_editor-patch](https://github.com/woaye168/sce_app_editor-patch)
- [sce_app_visual-injector](https://github.com/woaye168/sce_app_visual-injector)

## 关联知识库：bgd_sce_knowledge（源码知识库）

SCE 官方 Lua 包的**可信源码底座**（私有仓库）：全部包的最新解密源码 + 历史版本归档 + 关键包技术文档。

- **本地**：`<workspace>/bgd_sce_knowledge`（当前工作区的同名 folder，存在即直接用；不在工作区则按仓库名 `bgd_sce_knowledge` 在本机查找其克隆）
- **远程**：`https://github.com/woaye168/bgd_sce_knowledge`（私有；本地缺失时可克隆）
- **内容**：`source_code/<包名>/`（最新完整包源码，lua 已解密，`_manifest.json` 记版本）、`source_code_old/<包名>/<版本>/`（历史版本归档）、`knowledge_documents/`（@base/@script/@xdeditor/@xdeditor_startup/@lib_lobby/@server_lua_plus/@smallcard_shop 文档）
- **何时用**：需要查官方包的**真实实现**（如 @base/@common 的实现库 client_base、script 库 API、xdeditor 界面机制）或做**跨版本差异对照**时以它为准——本机 `res/_m/` 现场可能被 sce_app_editor-patch 补丁污染，不可直接信任

## AI 工具纪律（Trae）

- **同一文件的多个编辑必须串行**：SearchReplace 并行编辑同一文件存在 read-modify-write 竞态——各调用均报成功，实际互相覆盖静默丢编辑（甚至新旧文本交织）。一条消息对同一文件只发一个编辑；不同文件才可并行；批量编辑后复读关键区域复核落盘。
