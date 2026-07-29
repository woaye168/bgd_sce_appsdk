# bgd_sce_plugins

bgd_sce_tools 官方插件仓库。

## 结构

```
plugins/
  visual-injector/     # 模块To触编：把api下的Lua模块注入到触发编辑器中供触编调用
    src/               # Rust 源码
    Cargo.toml
    plugin.json        # 插件元数据（id/name/version/description/author）
registry.json          # 插件注册表（tools 拉取检查更新）
```

## 插件开发

1. 在 `plugins/` 下新建插件目录（如 `plugins/my-plugin/`）
2. 实现 `bgd_sce_tools_sdk` 的 trait（参考 `visual-injector`）
3. 编写 `plugin.json`（id/name/version/description/author）
4. 提交后 CI/CD 自动编译 `.dll` 并发布到 Release
5. `registry.json` 更新指向最新 Release

## 安装

在 bgd_sce_tools 的「插件」页中，从「woaye168/bgd_sce_plugins」市场安装。
