#![recursion_limit = "256"]
//! 可视化注入插件：通过插件 UI 手动把 API 模块注册到星火编辑器触发编辑器
//!
//! v0.2：移除构建期自动注册（BuildHook），改为在插件界面勾选模块后手动注册/卸载。

use bgd_sce_tools_sdk::{Plugin, SettingsHook, UiHook};
use rand::Rng;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// ---------------------------------------------------------------- 插件主体

/// 插件设置（持久化到 .bgd/plugins/visual-injector.json）
#[derive(Debug, Clone)]
struct PluginSettings {
    /// 通用搜索关键词（写入 keyword 前缀）
    common_keywords: String,
    /// 生成文件前缀（卸载时按此前缀匹配删除）
    file_prefix: String,
}

impl Default for PluginSettings {
    fn default() -> Self {
        Self {
            common_keywords: "woaye".to_string(),
            file_prefix: "__bgd_".to_string(),
        }
    }
}

/// 可视化注入插件
pub struct VisualInjectorPlugin {
    settings: Mutex<PluginSettings>,
}

impl VisualInjectorPlugin {
    pub fn new() -> Self {
        let settings = load_settings().unwrap_or_default();
        Self {
            settings: Mutex::new(settings),
        }
    }

    fn current_settings(&self) -> PluginSettings {
        self.settings.lock().unwrap().clone()
    }
}

impl Default for VisualInjectorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for VisualInjectorPlugin {
    fn name(&self) -> &str {
        "模块To触编"
    }
    fn version(&self) -> &str {
        "0.2.0"
    }
    fn description(&self) -> &str {
        "把api下的Lua模块注入到触发编辑器中供触编调用。"
    }
    fn author(&self) -> &str {
        "BGD"
    }
}

// ---------------------------------------------------------------- 导出符号

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn plugin_create() -> *mut dyn Plugin {
    Box::into_raw(Box::new(VisualInjectorPlugin::new()))
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn plugin_create_ui_hook() -> *mut dyn UiHook {
    Box::into_raw(Box::new(VisualInjectorPlugin::new()))
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn plugin_create_settings_hook() -> *mut dyn SettingsHook {
    Box::into_raw(Box::new(VisualInjectorPlugin::new()))
}

// ---------------------------------------------------------------- 路径与设置持久化

/// 从当前工作目录向上查找包含 `.bgd` 的项目目录
fn find_bgd_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(".bgd");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn settings_path(bgd_root: &Path) -> PathBuf {
    bgd_root.join("plugins").join("visual-injector.json")
}

fn load_settings() -> Option<PluginSettings> {
    let bgd_root = find_bgd_root()?;
    let content = fs::read_to_string(settings_path(&bgd_root)).ok()?;
    let v: Value = serde_json::from_str(&content).ok()?;
    let defaults = PluginSettings::default();
    Some(PluginSettings {
        common_keywords: v
            .get("common_keywords")
            .and_then(|s| s.as_str())
            .unwrap_or(&defaults.common_keywords)
            .to_string(),
        file_prefix: v
            .get("file_prefix")
            .and_then(|s| s.as_str())
            .unwrap_or(&defaults.file_prefix)
            .to_string(),
    })
}

fn save_settings(bgd_root: &Path, settings: &PluginSettings) {
    let path = settings_path(bgd_root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = json!({
        "common_keywords": settings.common_keywords,
        "file_prefix": settings.file_prefix,
    });
    let _ = fs::write(&path, serde_json::to_string_pretty(&content).unwrap_or_default());
}

// ---------------------------------------------------------------- UiHook

/// 扫描到的 API 模块条目
struct ModuleEntry {
    /// 模块标识：`<set>/<side>/<name>`（如 `libs/common/damage_api`）
    id: String,
    set: String,
    side: String,
    name: String,
}

/// 扫描 .bgd/{libs,src}/{client,common,server}/api/*.lua
fn scan_modules(bgd_root: &Path) -> Vec<ModuleEntry> {
    let mut modules = Vec::new();
    for set in ["libs", "src"] {
        for side in ["common", "server", "client"] {
            let api_dir = bgd_root.join(set).join(side).join("api");
            if !api_dir.is_dir() {
                continue;
            }
            let entries = match fs::read_dir(&api_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("lua") {
                    continue;
                }
                let name = match path.file_stem().and_then(|s| s.to_str()) {
                    Some(n) if !n.is_empty() => n.to_string(),
                    _ => continue,
                };
                if SKIP_MODULES.contains(&name.as_str()) {
                    continue;
                }
                modules.push(ModuleEntry {
                    id: format!("{set}/{side}/{name}"),
                    set: set.to_string(),
                    side: side.to_string(),
                    name,
                });
            }
        }
    }
    modules.sort_by(|a, b| a.id.cmp(&b.id));
    modules
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

impl UiHook for VisualInjectorPlugin {
    fn render_ui(&self) -> String {
        let settings = self.current_settings();
        let modules = find_bgd_root()
            .map(|root| scan_modules(&root))
            .unwrap_or_default();

        let mut module_items = String::new();
        for m in &modules {
            module_items.push_str(&format!(
                "<label class=\"vi-module-item\"><input type=\"checkbox\" class=\"vi-module\" value=\"{id}\"/> <span class=\"vi-tag\">{set}/{side}</span> {name}</label>\n",
                id = html_escape(&m.id),
                set = m.set,
                side = m.side,
                name = html_escape(&m.name),
            ));
        }
        if modules.is_empty() {
            module_items = "<div class=\"vi-empty\">未扫描到 API 模块（.bgd/{libs,src}/{common,server,client}/api/*.lua）</div>".to_string();
        }

        UI_TEMPLATE
            .replace("__MODULE_COUNT__", &modules.len().to_string())
            .replace("__MODULES__", &module_items)
            .replace("__COMMON_KEYWORDS__", &html_escape(&settings.common_keywords))
            .replace("__FILE_PREFIX__", &html_escape(&settings.file_prefix))
    }
}

const UI_TEMPLATE: &str = r##"<div class="vi-root">
<style>
.vi-root { background:#1e293b; color:#e2e8f0; font-family:"Microsoft YaHei",system-ui,sans-serif; padding:16px; border-radius:8px; }
.vi-root h2 { margin:0 0 12px; font-size:18px; color:#f1f5f9; }
.vi-section { background:#0f172a; border:1px solid #334155; border-radius:6px; padding:12px; margin-bottom:12px; }
.vi-section-title { font-size:14px; font-weight:600; color:#cbd5e1; margin-bottom:8px; }
.vi-module-list { max-height:320px; overflow-y:auto; display:flex; flex-direction:column; gap:4px; }
.vi-module-item { display:flex; align-items:center; gap:6px; padding:4px 6px; border-radius:4px; cursor:pointer; font-size:13px; }
.vi-module-item:hover { background:#1e293b; }
.vi-module-item input { accent-color:#4f46e5; }
.vi-tag { color:#818cf8; font-size:12px; }
.vi-empty { color:#64748b; font-size:13px; padding:8px 0; }
.vi-actions { display:flex; gap:8px; margin-bottom:12px; flex-wrap:wrap; }
.vi-btn { background:#4f46e5; color:#fff; border:none; border-radius:6px; padding:8px 16px; font-size:13px; cursor:pointer; }
.vi-btn:hover { background:#4338ca; }
.vi-btn-secondary { background:#334155; }
.vi-btn-secondary:hover { background:#475569; }
.vi-btn-danger { background:#be123c; }
.vi-btn-danger:hover { background:#9f1239; }
.vi-field { display:flex; align-items:center; gap:8px; margin-bottom:8px; font-size:13px; }
.vi-field label { width:120px; color:#cbd5e1; }
.vi-field input { flex:1; background:#1e293b; border:1px solid #334155; border-radius:4px; color:#e2e8f0; padding:6px 8px; font-size:13px; }
.vi-field input:focus { outline:none; border-color:#4f46e5; }
.vi-status { font-size:13px; color:#94a3b8; min-height:18px; }
</style>
<h2>模块To触编</h2>
<div class="vi-section">
  <div class="vi-section-title">API 模块（共 __MODULE_COUNT__ 个，勾选要注册的模块）</div>
  <div class="vi-actions">
    <button class="vi-btn vi-btn-secondary" id="vi-check-all">全选</button>
    <button class="vi-btn vi-btn-secondary" id="vi-uncheck-all">全不选</button>
  </div>
  <div class="vi-module-list">
__MODULES__  </div>
</div>
<div class="vi-actions">
  <button class="vi-btn" id="vi-register">注册到触编</button>
  <button class="vi-btn vi-btn-danger" id="vi-uninstall">卸载已注册</button>
</div>
<div class="vi-section">
  <div class="vi-section-title">设置</div>
  <div class="vi-field"><label for="vi-common-keywords">通用搜索关键词</label><input id="vi-common-keywords" value="__COMMON_KEYWORDS__"/></div>
  <div class="vi-field"><label for="vi-file-prefix">生成文件前缀</label><input id="vi-file-prefix" value="__FILE_PREFIX__"/></div>
  <div class="vi-actions" style="margin-bottom:0;">
    <button class="vi-btn" id="vi-save-settings">保存设置</button>
  </div>
</div>
<div class="vi-status" id="vi-status"></div>
<script>
(function () {
  function $(id) { return document.getElementById(id); }
  function bridge() { return window.bgdPlugin || {}; }
  function setStatus(msg) { var el = $("vi-status"); if (el) { el.textContent = msg; } }
  function checkedModules() {
    var boxes = document.querySelectorAll(".vi-module:checked");
    var arr = [];
    for (var i = 0; i < boxes.length; i++) { arr.push(boxes[i].value); }
    return arr;
  }
  function setAll(checked) {
    var boxes = document.querySelectorAll(".vi-module");
    for (var i = 0; i < boxes.length; i++) { boxes[i].checked = checked; }
  }
  $("vi-check-all").onclick = function () { setAll(true); };
  $("vi-uncheck-all").onclick = function () { setAll(false); };
  $("vi-register").onclick = function () {
    var modules = checkedModules();
    if (modules.length === 0) { setStatus("请先勾选要注册的模块"); return; }
    if (bridge().register) {
      bridge().register(modules);
      setStatus("已发送注册请求：" + modules.length + " 个模块");
    } else { setStatus("宿主桥接 window.bgdPlugin 不可用"); }
  };
  $("vi-uninstall").onclick = function () {
    if (bridge().uninstall) {
      bridge().uninstall();
      setStatus("已发送卸载请求");
    } else { setStatus("宿主桥接 window.bgdPlugin 不可用"); }
  };
  $("vi-save-settings").onclick = function () {
    var settings = {
      common_keywords: $("vi-common-keywords").value,
      file_prefix: $("vi-file-prefix").value
    };
    if (bridge().saveSettings) {
      bridge().saveSettings(settings);
      setStatus("设置已保存");
    } else { setStatus("宿主桥接 window.bgdPlugin 不可用"); }
  };
})();
</script>
</div>"##;

// ---------------------------------------------------------------- SettingsHook

impl SettingsHook for VisualInjectorPlugin {
    fn get_settings(&self) -> String {
        let s = self.current_settings();
        json!({
            "common_keywords": s.common_keywords,
            "file_prefix": s.file_prefix,
        })
        .to_string()
    }

    fn on_settings_changed(&self, settings: &str) {
        let v: Value = match serde_json::from_str(settings) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[visual-injector] 无法解析消息: {e}");
                return;
            }
        };

        // 带 action 的为操作消息（register/uninstall），否则视为设置更新
        match v.get("action").and_then(|a| a.as_str()) {
            Some("register") => {
                let modules = parse_module_ids(&v);
                if modules.is_empty() {
                    eprintln!("[visual-injector] register：未勾选任何模块");
                    return;
                }
                let settings = self.current_settings();
                match register_modules(&modules, &settings) {
                    Ok(count) => println!("[visual-injector] 注册完成，生成可视化 JSON {count} 个"),
                    Err(e) => eprintln!("[visual-injector] 注册失败: {e}"),
                }
            }
            Some("uninstall") => {
                let settings = self.current_settings();
                match uninstall_registered(&settings) {
                    Ok(count) => println!("[visual-injector] 卸载完成，删除文件 {count} 个"),
                    Err(e) => eprintln!("[visual-injector] 卸载失败: {e}"),
                }
            }
            _ => {
                let mut current = self.current_settings();
                if let Some(s) = v.get("common_keywords").and_then(|s| s.as_str()) {
                    current.common_keywords = s.to_string();
                }
                if let Some(s) = v.get("file_prefix").and_then(|s| s.as_str()) {
                    current.file_prefix = s.to_string();
                }
                *self.settings.lock().unwrap() = current.clone();
                if let Some(bgd_root) = find_bgd_root() {
                    save_settings(&bgd_root, &current);
                }
            }
        }
    }
}

/// 从 register 消息中解析模块标识列表（支持字符串数组或 {set,side,name} 对象数组）
fn parse_module_ids(v: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(arr) = v.get("modules").and_then(|m| m.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                ids.push(s.to_string());
            } else if let (Some(set), Some(side), Some(name)) = (
                item.get("set").and_then(|s| s.as_str()),
                item.get("side").and_then(|s| s.as_str()),
                item.get("name").and_then(|s| s.as_str()),
            ) {
                ids.push(format!("{set}/{side}/{name}"));
            }
        }
    }
    ids
}

// ---------------------------------------------------------------- 注册 / 卸载

/// 跳过的基础模块（不生成可视化 JSON）
const SKIP_MODULES: [&str; 10] = [
    "json", "log", "class", "co", "event", "promise", "exception", "deque", "event_deque", "init",
];

/// 手动注册：生成勾选模块的可视化 JSON 到 src/data 与 ui/src/data
fn register_modules(module_ids: &[String], settings: &PluginSettings) -> Result<usize, String> {
    let bgd_root = find_bgd_root().ok_or("未找到 .bgd 目录（当前目录不在游戏项目内）")?;
    let project_root = bgd_root.parent().unwrap_or(bgd_root.as_path()).to_path_buf();
    let mut count = 0;

    // 先全量扫描一遍收集用户自定义类名（用于参数类型映射）
    let mut user_classes: HashMap<String, String> = HashMap::new();
    for entry in scan_modules(&bgd_root) {
        let path = bgd_root.join(&entry.set).join(&entry.side).join("api").join(format!("{}.lua", entry.name));
        if let Ok(content) = fs::read_to_string(&path) {
            for (class_name, _) in extract_classes(&content) {
                user_classes.insert(class_name.to_lowercase(), class_name);
            }
        }
    }

    // 再按勾选的模块生成 JSON
    for id in module_ids {
        let parts: Vec<&str> = id.split('/').collect();
        if parts.len() != 3 {
            eprintln!("[visual-injector] 跳过无效模块标识: {id}");
            continue;
        }
        let (set, side, module) = (parts[0], parts[1], parts[2]);
        if SKIP_MODULES.contains(&module) {
            continue;
        }
        let path = bgd_root.join(set).join(side).join("api").join(format!("{module}.lua"));
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取模块失败 {}: {e}", path.display()))?;
        count += generate_module(&project_root, side, module, &content, &user_classes, settings)?;
    }

    Ok(count)
}

/// 卸载：删除四个数据目录下所有 `{file_prefix}*.json`
fn uninstall_registered(settings: &PluginSettings) -> Result<usize, String> {
    let bgd_root = find_bgd_root().ok_or("未找到 .bgd 目录（当前目录不在游戏项目内）")?;
    let project_root = bgd_root.parent().unwrap_or(bgd_root.as_path());
    let mut count = 0;

    let dirs = [
        project_root.join("src").join("data").join("methods"),
        project_root.join("ui").join("src").join("data").join("methods"),
        project_root.join("src").join("data").join("classes"),
        project_root.join("ui").join("src").join("data").join("classes"),
    ];

    for dir in &dirs {
        if !dir.is_dir() {
            continue;
        }
        let entries = fs::read_dir(dir).map_err(|e| format!("读取目录失败 {}: {e}", dir.display()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if file_name.starts_with(&settings.file_prefix) && file_name.ends_with(".json") {
                fs::remove_file(&path).map_err(|e| format!("删除失败 {}: {e}", path.display()))?;
                count += 1;
            }
        }
    }

    Ok(count)
}

/// 提取类名（---@class 注解）
fn extract_classes(content: &str) -> Vec<(String, bool)> {
    let mut classes = Vec::new();
    for line in content.lines() {
        if let Some(rest) = line.trim().strip_prefix("---@class ") {
            let name = rest.split_whitespace().next().unwrap_or("").to_string();
            if !name.is_empty() {
                classes.push((name, true));
            }
        }
    }
    classes
}

/// 生成单个模块的可视化 JSON（common 双端各一份，server 只服务端，client 只客户端）
fn generate_module(
    project_root: &Path,
    side: &str,
    module: &str,
    content: &str,
    user_classes: &HashMap<String, String>,
    settings: &PluginSettings,
) -> Result<usize, String> {
    let mut count = 0;
    let functions = parse_functions(content);
    let classes = parse_classes(content);

    let sides: Vec<&str> = match side {
        "common" => vec!["server", "client"],
        "server" => vec!["server"],
        "client" => vec!["client"],
        _ => vec![],
    };

    for target_side in sides {
        let (methods_dir, classes_dir) = if target_side == "server" {
            (
                project_root.join("src").join("data").join("methods"),
                project_root.join("src").join("data").join("classes"),
            )
        } else {
            (
                project_root.join("ui").join("src").join("data").join("methods"),
                project_root.join("ui").join("src").join("data").join("classes"),
            )
        };

        fs::create_dir_all(&methods_dir).map_err(|e| e.to_string())?;
        fs::create_dir_all(&classes_dir).map_err(|e| e.to_string())?;

        // 函数 JSON：{file_prefix}{Module}_{Func}.json
        for func in &functions {
            let json = function_to_json(func, module, target_side, user_classes, settings);
            let file = methods_dir.join(format!("{}{}_{}.json", settings.file_prefix, module, func.name));
            fs::write(&file, serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?)
                .map_err(|e| format!("写入失败 {}: {e}", file.display()))?;
            count += 1;
        }

        // 类 JSON：{file_prefix}{Class}.json
        for class in &classes {
            let json = class_to_json(class, target_side, user_classes);
            let file = classes_dir.join(format!("{}{}.json", settings.file_prefix, class.name));
            fs::write(&file, serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?)
                .map_err(|e| format!("写入失败 {}: {e}", file.display()))?;
            count += 1;
        }
    }

    Ok(count)
}

// ---------------------------------------------------------------- 函数解析

#[derive(Debug)]
struct FunctionInfo {
    name: String,
    params: Vec<ParamInfo>,
    returns: Vec<String>,
    description: String,
    ui_text: String,
    is_static: bool,
    is_event: bool,
    has_rest: bool,
}

#[derive(Debug)]
struct ParamInfo {
    name: String,
    lua_type: String,
    description: String,
}

/// 解析函数（function M.Xxx / function M:Xxx）
fn parse_functions(content: &str) -> Vec<FunctionInfo> {
    let mut functions = Vec::new();
    let mut doc_block = String::new();
    let mut in_doc = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("---") {
            in_doc = true;
            doc_block.push_str(line);
            doc_block.push('\n');
            continue;
        }

        if in_doc && !trimmed.is_empty() {
            // 文档块结束，检查是否是函数定义
            if let Some(func) = parse_function_line(trimmed, &doc_block) {
                if !func.is_event {
                    functions.push(func);
                }
            }
            in_doc = false;
            doc_block.clear();
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        // 无文档的函数定义
        if let Some(func) = parse_function_line(trimmed, "") {
            if !func.is_event {
                functions.push(func);
            }
        }
    }

    functions
}

/// 解析单行函数定义
fn parse_function_line(line: &str, doc: &str) -> Option<FunctionInfo> {
    // function M.Xxx(a, b) 或 function M:Xxx(a, b)
    let line = line.strip_prefix("function ")?;
    let line = line.strip_prefix("M.").or_else(|| line.strip_prefix("M:"))?;
    let name_end = line.find('(')?;
    let name = line[..name_end].trim().to_string();
    if name.is_empty() || name.starts_with('_') {
        return None;
    }

    // 跳过事件注册函数（OnXxx 开头）
    let is_event = name.starts_with("On") && name.len() > 2 && name[2..].chars().next()?.is_uppercase();

    // 解析参数
    let params_str = &line[name_end + 1..line.rfind(')')?];
    let mut params = Vec::new();
    let mut has_rest = false;
    for p in params_str.split(',') {
        let p = p.trim();
        if p == "..." {
            has_rest = true;
            continue;
        }
        if p.is_empty() {
            continue;
        }
        params.push(ParamInfo {
            name: p.to_string(),
            lua_type: "any".to_string(),
            description: String::new(),
        });
    }

    // 从文档块提取信息
    let mut description = String::new();
    let mut ui_text = String::new();
    let mut returns = Vec::new();
    let mut is_static = false;

    for doc_line in doc.lines() {
        let doc_line = doc_line.trim();
        if let Some(rest) = doc_line.strip_prefix("---@param ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                let param_name = parts[0];
                let lua_type = parts[1];
                let desc = parts[2..].join(" ");
                if let Some(param) = params.iter_mut().find(|p| p.name == param_name) {
                    param.lua_type = lua_type.to_string();
                    param.description = desc;
                }
            }
        } else if let Some(rest) = doc_line.strip_prefix("---@return ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if !parts.is_empty() {
                returns.push(parts[0].to_string());
            }
        } else if let Some(rest) = doc_line.strip_prefix("---@visual.uiText ") {
            ui_text = rest.to_string();
        } else if doc_line.starts_with("---@visual.static") {
            is_static = true;
        } else if !doc_line.starts_with("---@") && description.is_empty() {
            // 普通注释行第一行作为描述
            description = doc_line.trim_start_matches("---").trim().to_string();
        }
    }

    Some(FunctionInfo {
        name,
        params,
        returns,
        description,
        ui_text,
        is_static,
        is_event,
        has_rest,
    })
}

// ---------------------------------------------------------------- 类解析

#[derive(Debug)]
struct ClassInfo {
    name: String,
    description: String,
    methods: Vec<FunctionInfo>,
    has_constructor: bool,
}

/// 解析类（---@class + M.New / prototype:____constructor / prototype:Method / M:Method）
fn parse_classes(content: &str) -> Vec<ClassInfo> {
    let mut classes = Vec::new();
    let mut current_class: Option<ClassInfo> = None;
    let mut doc_block = String::new();
    let mut in_doc = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("---") {
            in_doc = true;
            doc_block.push_str(line);
            doc_block.push('\n');
            continue;
        }

        if in_doc && !trimmed.is_empty() {
            // 检查是否是类定义
            if trimmed.contains("M.New(") || trimmed.contains("prototype:____constructor(") {
                let class_name = extract_class_name_from_doc(&doc_block);
                if !class_name.is_empty() {
                    current_class = Some(ClassInfo {
                        name: class_name,
                        description: extract_description_from_doc(&doc_block),
                        methods: Vec::new(),
                        has_constructor: true,
                    });
                }
            }
            // 检查是否是方法定义
            else if trimmed.contains("prototype:") || trimmed.contains("M:") {
                if let Some(ref mut class) = current_class {
                    if let Some(func) = parse_function_line(trimmed, &doc_block) {
                        class.methods.push(func);
                    }
                }
            }
            in_doc = false;
            doc_block.clear();
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }
    }

    if let Some(class) = current_class {
        classes.push(class);
    }

    classes
}

fn extract_class_name_from_doc(doc: &str) -> String {
    for line in doc.lines() {
        if let Some(rest) = line.trim().strip_prefix("---@class ") {
            return rest.split_whitespace().next().unwrap_or("").to_string();
        }
    }
    String::new()
}

fn extract_description_from_doc(doc: &str) -> String {
    for line in doc.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("---@") {
            return trimmed.trim_start_matches("---").trim().to_string();
        }
    }
    String::new()
}

// ---------------------------------------------------------------- JSON 生成

fn simple_type(name: &str, display: &str) -> Value {
    json!({
        "ElementName": "SimpleType",
        "name": name,
        "displayName": display,
        "flags": {},
        "tips": "",
        "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
        "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
    })
}

fn void_type() -> Value {
    simple_type("void", "void")
}

/// 函数 → FunctionDefine JSON
fn function_to_json(
    func: &FunctionInfo,
    module: &str,
    side: &str,
    user_classes: &HashMap<String, String>,
    settings: &PluginSettings,
) -> Value {
    let mut rng = rand::thread_rng();
    let id: u64 = rng.gen_range(10_000_000..=9_999_999_999u64);

    let params: Vec<Value> = func
        .params
        .iter()
        .map(|p| param_to_json(p, side, user_classes))
        .collect();

    let return_type = if func.returns.is_empty() {
        void_type()
    } else {
        type_to_json(&func.returns[0], side, user_classes)
    };

    let mut flags = json!({
        "enableDisplayName": true,
        "isDeclare": true,
        "noSelf": true,
        "pop": true
    });
    if func.is_static {
        flags["isStatic"] = json!(true);
    }

    // uiText：优先 ---@visual.uiText 注解，否则默认 `{Func}(~1~, ~2~, ...)`
    let ui_text = if func.ui_text.is_empty() {
        let placeholders: Vec<String> = (1..=func.params.len()).map(|i| format!("~{i}~")).collect();
        format!("{}({})", func.name, placeholders.join(", "))
    } else {
        func.ui_text.clone()
    };

    let mut result = json!({
        "ElementName": "FunctionDefine",
        "name": format!("{}_{}", module, func.name),
        "packageName": "p_eelc",
        "id": format!("FunctionDefine:{}_{}:{}", module, func.name, id),
        "displayName": func.description,
        "description": func.description,
        "keyword": format!("{} {} {}", settings.common_keywords, module, func.name),
        "tips": func.description,
        "uiText": ui_text,
        "rankOrder": 2,
        "v2_version": 0.9,
        "s_or_c": side,
        "flags": flags,
        "parameters": { "__TYPE": "Array", "contents": params },
        "realReturnType": return_type,
        "actions": { "__TYPE": "Array", "contents": {} },
        "subsections": { "__TYPE": "Array", "contents": {} },
        "typeParameters": { "__TYPE": "Array", "contents": {} },
        "typeParametersExtends": { "__TYPE": "Map", "contents": {} },
        "variables": { "__TYPE": "Array", "contents": {} },
        "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
        "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 },
        "defaultResetParameterCount": 0
    });

    if func.has_rest {
        result["restParameter"] = json!({
            "ElementName": "Parameter",
            "name": "args",
            "displayName": "args",
            "realType": {
                "ElementName": "InstanceType",
                "source": {
                    "ElementName": "Source",
                    "targetUninit": {
                        "id": "Class:Array",
                        "packageName": "__common__",
                        "s_or_c": "common"
                    }
                },
                "typeArgs": {
                    "__TYPE": "Array",
                    "contents": [{
                        "ElementName": "SimpleType",
                        "name": "any",
                        "displayName": "任意",
                        "flags": {},
                        "tips": "",
                        "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                        "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
                    }]
                }
            }
        });
    }

    result
}

/// 参数 → Parameter JSON
fn param_to_json(param: &ParamInfo, side: &str, user_classes: &HashMap<String, String>) -> Value {
    let mut rng = rand::thread_rng();
    let id: u64 = rng.gen_range(1_000_000_000..=9_999_999_999u64);

    json!({
        "ElementName": "Parameter",
        "name": param.name,
        "displayName": param.description,
        "id": format!("Variable:{}:{}", param.name, id),
        "keyword": "",
        "label": "默认",
        "packageName": "p_eelc",
        "tips": param.description,
        "s_or_c": side,
        "flags": {},
        "realType": type_to_json(&param.lua_type, side, user_classes),
        "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
        "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
    })
}

/// Lua 类型 → 类型 JSON
fn type_to_json(lua_type: &str, side: &str, user_classes: &HashMap<String, String>) -> Value {
    match lua_type {
        "number" => simple_type("number", "数值"),
        "string" => simple_type("string", "字符串"),
        "boolean" => simple_type("boolean", "布尔"),
        "table" => simple_type("table", "表格"),
        "void" => void_type(),
        t if t.ends_with("[]") => {
            let elem_type = &t[..t.len() - 2];
            json!({
                "ElementName": "InstanceType",
                "displayName": "",
                "flags": {},
                "tips": "",
                "source": {
                    "ElementName": "Source",
                    "targetUninit": {
                        "id": "Class:Array",
                        "packageName": "__common__",
                        "s_or_c": "common"
                    }
                },
                "typeArgs": {
                    "__TYPE": "Array",
                    "contents": [type_to_json(elem_type, side, user_classes)]
                },
                "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
            })
        }
        t if t.starts_with("fun(") => {
            json!({
                "ElementName": "FuncType",
                "displayName": "",
                "flags": {},
                "isArrowFunc": false,
                "noSelf": true,
                "optionalParams": { "__TYPE": "Array", "contents": {} },
                "params": { "__TYPE": "Array", "contents": {} },
                "returnType": {
                    "ElementName": "SimpleType",
                    "name": "void",
                    "displayName": "void",
                    "flags": {},
                    "tips": "",
                    "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                    "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
                },
                "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                "tips": "",
                "typeParams": { "__TYPE": "Array", "contents": {} },
                "typeParamsExtends": { "__TYPE": "Map", "contents": {} },
                "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
            })
        }
        t => {
            // 引擎对象或用户自定义类
            let lower = t.to_lowercase();
            if let Some((_, class_name)) = ENGINE_CLASSES.iter().find(|(k, _)| *k == lower) {
                let pkg = if side == "server" { "__server__" } else { "__client__" };
                json!({
                    "ElementName": "InstanceType",
                    "displayName": "",
                    "flags": {},
                    "tips": "",
                    "source": {
                        "ElementName": "Source",
                        "targetUninit": {
                            "id": format!("Class:{}", class_name),
                            "packageName": pkg,
                            "s_or_c": side
                        }
                    },
                    "typeArgs": { "__TYPE": "Array", "contents": {} },
                    "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                    "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
                })
            } else if let Some(class_name) = user_classes.get(&lower) {
                json!({
                    "ElementName": "InstanceType",
                    "displayName": "",
                    "flags": {},
                    "tips": "",
                    "source": {
                        "ElementName": "Source",
                        "targetUninit": {
                            "id": format!("Class:{}", class_name),
                            "packageName": "p_eelc",
                            "s_or_c": side
                        }
                    },
                    "typeArgs": { "__TYPE": "Array", "contents": {} },
                    "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                    "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
                })
            } else {
                // 未知类型 → table 近似
                simple_type("table", "表格")
            }
        }
    }
}

/// 引擎对象类型映射（Lua 类型 → Class:Xxx）
const ENGINE_CLASSES: [(&str, &str); 27] = [
    ("unit", "Unit"),
    ("player", "Player"),
    ("item", "Item"),
    ("skill", "Skill"),
    ("buff", "Buff"),
    ("timer", "Timer"),
    ("point", "Point"),
    ("target", "Target"),
    ("trigger", "Trigger"),
    ("region", "Region"),
    ("actor", "Actor"),
    ("slot", "Slot"),
    ("camera", "Camera"),
    ("vector", "Vector"),
    ("screenpos", "ScreenPos"),
    ("aisearcher", "AISearcher"),
    ("scorecommitter", "ScoreCommitter"),
    ("team", "Team"),
    ("mover", "Mover"),
    ("unitgroup", "UnitGroup"),
    ("line", "Line"),
    ("effectparam", "EffectParam"),
    ("effectparamshared", "EffectParamShared"),
    ("damageinstance", "DamageInstance"),
    ("healinstance", "HealInstance"),
    ("datacache", "DataCache"),
    ("ieventnotify", "IEventNotify"),
];

/// 类 → Class JSON
fn class_to_json(class: &ClassInfo, side: &str, user_classes: &HashMap<String, String>) -> Value {
    let mut rng = rand::thread_rng();
    let id: u64 = rng.gen_range(1_000_000_000..=9_999_999_999u64);

    let constructor = if class.has_constructor {
        json!({
            "ElementName": "ConstructorDefine",
            "name": "____constructor",
            "id": format!("Class:{}:{}, ConstructorDefine:____constructor", class.name, id),
            "packageName": "p_eelc",
            "s_or_c": side,
            "flags": { "isDeclare": true },
            "parameters": { "__TYPE": "Array", "contents": [] },
            "realReturnType": {
                "ElementName": "InstanceType",
                "source": {
                    "ElementName": "Source",
                    "targetUninit": {
                        "id": format!("Class:{}:{}", class.name, id),
                        "packageName": "p_eelc",
                        "s_or_c": side
                    }
                },
                "typeArgs": { "__TYPE": "Array", "contents": {} },
                "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
            }
        })
    } else {
        Value::Null
    };

    let methods: Vec<Value> = class
        .methods
        .iter()
        .map(|m| {
            let mut flags = json!({ "isDeclare": true });
            if m.is_static {
                flags["isStatic"] = json!(true);
                flags["noSelf"] = json!(true);
            }
            json!({
                "ElementName": "MethodDefine",
                "name": m.name,
                "id": format!("Class:{}:{}, MethodDefine:{}", class.name, id, m.name),
                "packageName": "p_eelc",
                "s_or_c": side,
                "flags": flags,
                "parameters": { "__TYPE": "Array", "contents": m.params.iter().map(|p| param_to_json(p, side, user_classes)).collect::<Vec<_>>() },
                "realReturnType": if m.returns.is_empty() {
                    void_type()
                } else {
                    type_to_json(&m.returns[0], side, user_classes)
                }
            })
        })
        .collect();

    json!({
        "ElementName": "Class",
        "name": class.name,
        "id": format!("Class:{}:{}", class.name, id),
        "packageName": "p_eelc",
        "displayName": class.description,
        "description": class.description,
        "keyword": class.name,
        "tips": class.description,
        "s_or_c": side,
        "flags": {},
        "_constructor": constructor,
        "_methods": { "__TYPE": "Array", "contents": methods },
        "folder": {
            "ElementName": "Folder",
            "name": class.name,
            "id": format!("Folder:{}:{}", class.name, id),
            "packageName": "p_eelc",
            "s_or_c": side
        },
        "thisVariable": {
            "ElementName": "Variable",
            "name": "this",
            "id": format!("Variable:this:{}", id),
            "packageName": "p_eelc",
            "s_or_c": side
        },
        "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
        "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
    })
}
