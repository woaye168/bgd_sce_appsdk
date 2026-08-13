# bgd_sce_plugins（应用清单仓库）

bgd_sce_tools 官方**应用清单**仓库（WeGame 模式）。

> 架构已从「DLL 插件」重构为「独立 EXE 应用」。本仓库只存应用清单，不再存应用代码。

## 结构

```
registry.json    # 应用清单（bgd_sce_tools 应用市场拉取）
```

## 应用清单格式

> 仓库已转私有：release asset 直链（releases/download/...）带 token 也无法下载，清单不再存 URL，
> 改用 `repo` + `tag` + `asset_name` 三字段，由 bgd_sce_tools 走 GitHub API 定位并下载（需在工具设置中配置 GitHub Token）。

```json
{
  "apps": [
    {
      "id": "visual-injector",
      "name": "模块To触编",
      "version": "0.4.0",
      "description": "把api下的Lua模块注入到触发编辑器中供触编调用。",
      "author": "BGD",
      "repo": "woaye168/sce_app_visual-injector",
      "tag": "v0.4.0",
      "asset_name": "sce_app_visual-injector.exe"
    }
  ]
}
```

- `repo`：应用所在的 GitHub 仓库（owner/repo）
- `tag`：Release tag；填 `"latest"` 表示始终取最新 Release
- `asset_name`：Release 附件文件名（必须与 CI 上传的 asset 名一致）

## 应用仓库

应用代码在独立仓库（命名约定 `sce_app_*`），各自维护 CI 发布 EXE 到 Release：

- [sce_app_visual-injector](https://github.com/woaye168/sce_app_visual-injector)：模块To触编

## 更新清单

应用发布新 Release 后，更新 `registry.json` 的 `version` / `tag`（`asset_name` 一般不变）。

## 安装

在 bgd_sce_tools 的「应用」页中，从应用市场安装（需先在工具设置中配置 GitHub Token）。
