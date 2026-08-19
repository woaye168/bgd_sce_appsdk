# AGENTS.md — bgd_sce_appsdk

> 本文件面向在本仓库工作的 AI 编程代理。

## 项目定位

BGD SCE 应用生态的**公共 SDK 仓库**（原 bgd_sce_plugins），双职责：

1. **`registry.json`**：应用市场分发清单（宿主 [bgd_sce_tools](https://github.com/woaye168/bgd_sce_tools) 读取，列出应用 id/name/repo）。
2. **Rust crate `bgd_appsdk`**（`src/`）：sce_app_* 应用的公共基建，避免重复实现。

## SDK 内容（src/）

| 模块 | 内容 |
| --- | --- |
| `single_instance` | 命名互斥体单实例 + 唤起/退出/刷新命名事件 + 本进程主窗口查找（Win32） |
| `watcher` | 看守线程：等信号并 Win32 驱动主窗口（隐藏/唤起）；退出/刷新标志（`take_quit`/`take_refresh`） |
| `log` | 按日期分文件的应用日志（`<项目>/.bgd/log/<app>-YYYY-MM-DD.log`） |
| `config` | 应用配置持久化（exe 旁 `<app>.config.json`；最近项目；路径正斜杠统一） |

**使用方约定**：GUI 路径启动先 `single_instance::acquire(prefix)`（已存在实例会收到唤起信号并退出）；`watcher::spawn(guard, background)`；UI 主循环轮询 `take_quit`/`take_refresh`。CLI 短进程（mcp/notify 等）应在 acquire 前分发，不受单实例限制。宿主侧协议：`--background` 静默驻留、`--quit` 优雅退出、`notify key=value` 解耦通知。

**关键结论（改动前必读）**：egui 窗口隐藏时事件循环休眠，信号处理不能放 UI update，也不能依赖 ViewportCommand——必须看守线程直接 Win32（QQ/微信式驻留同款做法，实测定稿）。

## 消费方式

```toml
bgd_appsdk = { path = "../bgd_sce_plugins" }   # 本地相邻目录
```

CI 侧：actions/checkout 本仓库到工作区内 `bgd_sce_appsdk` 子目录（私有仓库用 BGD_CROSS_REPO_PAT），构建前把 path 改写为 `bgd_sce_appsdk`。

## 构建与测试

```bash
cargo build
cargo test
```

## 现有使用方

- [sce_app_editor-patch](https://github.com/woaye168/sce_app_editor-patch)（首个迁移方）
- sce_app_visual-injector（待迁移）
