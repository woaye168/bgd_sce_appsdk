# bgd_sce_appsdk（应用公共 SDK + 分发清单）

BGD SCE 应用生态的公共仓库（原 bgd_sce_plugins），双职责：

1. **`registry.json`**：应用市场分发清单（宿主 [bgd_sce_tools](https://github.com/woaye168/bgd_sce_tools) 读取）。**极简格式**——只登记应用 id/name/repo，版本/描述/作者/版本说明等元数据来自各应用仓库 CI 合成的 `app-release.json` asset，**应用发版不需要改动本清单**。
2. **Rust crate `bgd_appsdk`**（`src/`）：sce_app_* 应用的公共基建，避免重复实现。

## registry.json 格式

```json
{
  "apps": [
    { "id": "editor-patch", "name": "编辑器补丁", "repo": "woaye168/sce_app_editor-patch" }
  ]
}
```

宿主读取链：registry（raw）→ `repos/<repo>/releases/latest` → 读 `app-release.json` asset（版本/描述/作者/asset 名/版本说明/默认自启）→ 下载 exe asset。

## 应用侧约定（新应用接入清单）

1. 仓库根放静态元数据 `app.json`（不含版本）：

```json
{ "id": "...", "name": "...", "description": "...", "author": "BGD", "asset_name": "xxx.exe", "default_auto_start": false }
```

2. release 工作流里 CI 合成 `app-release.json` asset（= app.json + version(tag) + release_notes），随 exe 一起上传。
3. 在 registry.json 登记一行（仅此一次，之后发版不再动清单）。

## bgd_appsdk 内容（src/）

| 模块 | 内容 |
| --- | --- |
| `single_instance` | 命名互斥体单实例 + 唤起/退出/刷新命名事件 + 本进程主窗口查找（Win32） |
| `watcher` | 看守线程：等信号并 Win32 驱动主窗口（隐藏/唤起）；退出/刷新标志；--quit 不依赖 UI tick（exit 兜底） |
| `ui` | 通用窗口壳 AppShell：中文字体/标题尺寸约定/项目栏/选项卡/状态栏/标志轮询（应用实现 ShellApp trait 注册标签即可） |
| `log` | 按日期分文件的应用日志（`<项目>/.bgd/log/<app>-YYYY-MM-DD.log`） |
| `config` | 应用配置持久化（exe 旁 `<app>.config.json`；最近项目；路径正斜杠统一） |

宿主协议（各应用遵循）：`--background` 静默驻留、`--quit` 优雅退出、`notify key=value` 解耦通知。

## 消费方式

```toml
bgd_appsdk = "0.1"   # crates.io 公开包（tag 触发 publish.yml 自动发布）
```

CLI 脚手架：`cargo install bgd_appsdk` 后 `bgd_appsdk new <app-id> --name <中文名>` 生成标准应用骨架。

## 现有使用方

- [sce_app_editor-patch](https://github.com/woaye168/sce_app_editor-patch)
- [sce_app_visual-injector](https://github.com/woaye168/sce_app_visual-injector)
