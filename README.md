# bgd_sce_plugins（应用清单仓库）

bgd_sce_tools 官方**应用清单**仓库（WeGame 模式）。

> 架构已从「DLL 插件」重构为「独立 EXE 应用」。本仓库只存应用清单，不再存应用代码。

## 结构

```
registry.json    # 应用清单（bgd_sce_tools 应用市场拉取）
```

## 应用清单格式

```json
{
  "apps": [
    {
      "id": "visual-injector",
      "name": "模块To触编",
      "version": "0.4.0",
      "description": "把api下的Lua模块注入到触发编辑器中供触编调用。",
      "author": "BGD",
      "download_url": "https://github.com/woaye168/sce_app_visual-injector/releases/download/v0.4.0/sce_app_visual-injector.exe",
      "checksum": ""
    }
  ]
}
```

## 应用仓库

应用代码在独立仓库（命名约定 `sce_app_*`），各自维护 CI 发布 EXE 到 Release：

- [sce_app_visual-injector](https://github.com/woaye168/sce_app_visual-injector)：模块To触编

## 更新清单

应用发布新 Release 后，更新 `registry.json` 的 `version` / `download_url`。

## 安装

在 bgd_sce_tools 的「应用」页中，从应用市场安装。
