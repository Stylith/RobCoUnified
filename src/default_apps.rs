use std::path::Path;

use crate::config::{
    get_settings, load_apps, load_games, load_networks, DefaultAppBinding, DefaultAppMenuSource,
    Settings,
};
use crate::launcher::json_to_cmd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultAppSlot {
    TextCode,
    Ebook,
}

#[derive(Debug, Clone)]
pub enum DefaultAppChoiceAction {
    Set(DefaultAppBinding),
    PromptCustom,
}

#[derive(Debug, Clone)]
pub enum ResolvedDocumentOpen {
    BuiltinRobcoTerminalWriter,
    ExternalArgv(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct DefaultAppChoice {
    pub label: String,
    pub action: DefaultAppChoiceAction,
}

pub fn slot_label(slot: DefaultAppSlot) -> &'static str {
    match slot {
        DefaultAppSlot::TextCode => "Text/Code Files",
        DefaultAppSlot::Ebook => "Ebook Files",
    }
}

fn is_text_code_ext(ext: &str) -> bool {
    matches!(
        ext,
        "txt"
            | "md"
            | "rs"
            | "c"
            | "h"
            | "cpp"
            | "hpp"
            | "cc"
            | "py"
            | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "json"
            | "toml"
            | "yaml"
            | "yml"
            | "ini"
            | "conf"
            | "log"
            | "csv"
            | "xml"
            | "html"
            | "css"
            | "java"
            | "go"
            | "rb"
            | "sh"
            | "zsh"
            | "bash"
            | "lua"
            | "sql"
    )
}

fn is_ebook_ext(ext: &str) -> bool {
    matches!(ext, "epub" | "pdf" | "mobi" | "azw3")
}

pub fn slot_for_path(path: &Path) -> Option<DefaultAppSlot> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    if is_text_code_ext(&ext) {
        return Some(DefaultAppSlot::TextCode);
    }
    if is_ebook_ext(&ext) {
        return Some(DefaultAppSlot::Ebook);
    }
    None
}

pub fn binding_for_slot(settings: &Settings, slot: DefaultAppSlot) -> DefaultAppBinding {
    match slot {
        DefaultAppSlot::TextCode => settings.default_apps.text_code.clone(),
        DefaultAppSlot::Ebook => settings.default_apps.ebook.clone(),
    }
}

pub fn set_binding_for_slot(
    settings: &mut Settings,
    slot: DefaultAppSlot,
    binding: DefaultAppBinding,
) {
    match slot {
        DefaultAppSlot::TextCode => settings.default_apps.text_code = binding,
        DefaultAppSlot::Ebook => settings.default_apps.ebook = binding,
    }
}

fn source_label(source: DefaultAppMenuSource) -> &'static str {
    match source {
        DefaultAppMenuSource::Applications => "Applications",
        DefaultAppMenuSource::Games => "Games",
        DefaultAppMenuSource::Network => "Network",
    }
}

pub fn binding_label(binding: &DefaultAppBinding) -> String {
    match binding {
        DefaultAppBinding::Builtin { id } => {
            if id.eq_ignore_ascii_case("robco_terminal_writer") {
                "Built-in: ROBCO Terminal Writer".to_string()
            } else {
                format!("Built-in: {id}")
            }
        }
        DefaultAppBinding::MenuEntry { source, name } => {
            format!("{}: {}", source_label(*source), name)
        }
        DefaultAppBinding::CustomArgv { argv } => {
            if argv.is_empty() {
                "Custom: (empty)".to_string()
            } else if argv[0].eq_ignore_ascii_case("epy") {
                "External: epy".to_string()
            } else {
                format!("Custom: {}", argv.join(" "))
            }
        }
    }
}

fn sorted_json_keys(map: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let mut out: Vec<String> = map.keys().cloned().collect();
    out.sort();
    out
}

fn command_exists_in_path(bin: &str) -> bool {
    let candidate = std::path::Path::new(bin);
    if candidate.components().count() > 1 {
        return candidate.is_file();
    }

    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    #[cfg(windows)]
    let exts: Vec<String> = std::env::var_os("PATHEXT")
        .map(|v| {
            std::env::split_paths(&std::path::PathBuf::from(v))
                .filter_map(|p| p.to_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                ".EXE".to_string(),
                ".BAT".to_string(),
                ".CMD".to_string(),
                ".COM".to_string(),
            ]
        });

    for dir in std::env::split_paths(&path_var) {
        let p = dir.join(bin);
        if p.is_file() {
            return true;
        }
        #[cfg(windows)]
        {
            for ext in &exts {
                let with_ext = dir.join(format!("{bin}{ext}"));
                if with_ext.is_file() {
                    return true;
                }
            }
        }
    }
    false
}

pub fn default_app_choices(slot: DefaultAppSlot) -> Vec<DefaultAppChoice> {
    let mut rows = Vec::new();
    if matches!(slot, DefaultAppSlot::TextCode) {
        rows.push(DefaultAppChoice {
            label: "Built-in: ROBCO Terminal Writer".to_string(),
            action: DefaultAppChoiceAction::Set(DefaultAppBinding::Builtin {
                id: "robco_terminal_writer".to_string(),
            }),
        });
    }
    if command_exists_in_path("epy") {
        rows.push(DefaultAppChoice {
            label: "External: epy".to_string(),
            action: DefaultAppChoiceAction::Set(DefaultAppBinding::CustomArgv {
                argv: vec!["epy".to_string()],
            }),
        });
    }

    for key in sorted_json_keys(&load_apps()) {
        rows.push(DefaultAppChoice {
            label: format!("Applications: {key}"),
            action: DefaultAppChoiceAction::Set(DefaultAppBinding::MenuEntry {
                source: DefaultAppMenuSource::Applications,
                name: key,
            }),
        });
    }
    for key in sorted_json_keys(&load_games()) {
        rows.push(DefaultAppChoice {
            label: format!("Games: {key}"),
            action: DefaultAppChoiceAction::Set(DefaultAppBinding::MenuEntry {
                source: DefaultAppMenuSource::Games,
                name: key,
            }),
        });
    }
    for key in sorted_json_keys(&load_networks()) {
        rows.push(DefaultAppChoice {
            label: format!("Network: {key}"),
            action: DefaultAppChoiceAction::Set(DefaultAppBinding::MenuEntry {
                source: DefaultAppMenuSource::Network,
                name: key,
            }),
        });
    }

    rows.push(DefaultAppChoice {
        label: "Custom Command (argv JSON)...".to_string(),
        action: DefaultAppChoiceAction::PromptCustom,
    });
    rows
}

pub fn parse_custom_argv_json(input: &str) -> Option<Vec<String>> {
    let argv: Vec<String> = serde_json::from_str(input).ok()?;
    if argv.is_empty() {
        return None;
    }
    if argv[0].trim().is_empty() {
        return None;
    }
    Some(argv)
}

fn resolve_menu_entry(source: DefaultAppMenuSource, name: &str) -> Option<Vec<String>> {
    let map = match source {
        DefaultAppMenuSource::Applications => load_apps(),
        DefaultAppMenuSource::Games => load_games(),
        DefaultAppMenuSource::Network => load_networks(),
    };
    let cmd = map.get(name).map(json_to_cmd).unwrap_or_default();
    if cmd.is_empty() {
        None
    } else {
        Some(cmd)
    }
}

fn resolve_binding_open(
    binding: &DefaultAppBinding,
    slot: DefaultAppSlot,
) -> Option<ResolvedDocumentOpen> {
    match binding {
        DefaultAppBinding::Builtin { id } => {
            if matches!(slot, DefaultAppSlot::TextCode)
                && id.eq_ignore_ascii_case("robco_terminal_writer")
            {
                Some(ResolvedDocumentOpen::BuiltinRobcoTerminalWriter)
            } else if id.eq_ignore_ascii_case("epy") {
                // Legacy compatibility for older configs.
                Some(ResolvedDocumentOpen::ExternalArgv(vec!["epy".to_string()]))
            } else {
                None
            }
        }
        DefaultAppBinding::MenuEntry { source, name } => {
            resolve_menu_entry(*source, name).map(ResolvedDocumentOpen::ExternalArgv)
        }
        DefaultAppBinding::CustomArgv { argv } => {
            if argv.is_empty() {
                None
            } else {
                Some(ResolvedDocumentOpen::ExternalArgv(argv.clone()))
            }
        }
    }
}

pub fn resolve_document_open(path: &Path) -> Option<ResolvedDocumentOpen> {
    let slot = slot_for_path(path)?;
    let settings = get_settings();
    let binding = binding_for_slot(&settings, slot);
    let resolved = resolve_binding_open(&binding, slot)?;
    match resolved {
        ResolvedDocumentOpen::BuiltinRobcoTerminalWriter => Some(resolved),
        ResolvedDocumentOpen::ExternalArgv(mut cmd) => {
            cmd.push(path.display().to_string());
            Some(ResolvedDocumentOpen::ExternalArgv(cmd))
        }
    }
}
