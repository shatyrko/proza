// User settings with CotEditor's defaults, stored as JSON in ~/.config/proza/settings.json.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Per-mode editor behaviour (Settings ▸ Mode): "General" (prose syntaxes), "Code", or a syntax id.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Mode {
    pub monospaced: bool,
    pub smart_quotes: bool,
    pub smart_dashes: bool,
    pub auto_close: bool,
    pub spell_check: bool,
    pub complete_standard: bool,
    pub complete_document: bool,
    pub complete_syntax: bool,
    pub suggest: bool,
    pub custom_indent: bool,
    pub soft_tabs: bool,
    pub tab_width: u32,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::general()
    }
}

impl Mode {
    pub fn general() -> Mode {
        Mode {
            monospaced: false,
            smart_quotes: false,
            smart_dashes: false,
            auto_close: false,
            spell_check: true,
            complete_standard: true,
            complete_document: true,
            complete_syntax: false,
            suggest: false,
            custom_indent: false,
            soft_tabs: false,
            tab_width: 4,
        }
    }

    pub fn code() -> Mode {
        Mode { monospaced: true, auto_close: true, spell_check: false, complete_standard: false, complete_syntax: true, ..Mode::general() }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Snippet {
    /// A syntax id, or "" for all syntaxes.
    pub scope: String,
    pub name: String,
    /// GTK accelerator, or "".
    pub key: String,
    pub format: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FileDrop {
    pub scope: String,
    /// Comma-separated, "" = all.
    pub extensions: String,
    pub format: String,
    pub description: String,
}

/// CotEditor's factory file-drop rules.
pub fn default_file_drops() -> Vec<FileDrop> {
    let rule = |scope: &str, ext: &str, format: &str| FileDrop { scope: scope.into(), extensions: ext.into(), format: format.into(), description: String::new() };
    vec![
        rule("markdown", "jpg, jpeg, gif, png", "![<<<FILENAME-NOSUFFIX>>>](<<<RELATIVE-PATH>>>)"),
        rule("markdown", "", "[<<<FILENAME-NOSUFFIX>>>](<<<RELATIVE-PATH>>>)"),
        rule("html", "jpg, jpeg, gif, png", "<img src=\"<<<RELATIVE-PATH>>>\" alt=\"<<<FILENAME-NOSUFFIX>>>\" title=\"<<<FILENAME-NOSUFFIX>>>\" width=\"<<<IMAGEWIDTH>>>\" height=\"<<<IMAGEHEIGHT>>>\" />"),
        rule("html", "js", "<script type=\"text/javascript\" src=\"<<<RELATIVE-PATH>>>\"></script>"),
        rule("html", "html, htm, php", "<a href=\"<<<RELATIVE-PATH>>>\" title=\"<<<FILENAME-NOSUFFIX>>>\"></a>"),
        rule("css", "", "url(\"<<<RELATIVE-PATH>>>\")"),
    ]
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // General
    pub reopen_windows: bool,
    pub auto_save: bool,
    // Appearance
    pub font: String,
    pub mono_font: String,
    pub line_height: f64,
    pub theme: String,
    pub appearance: String, // "default" | "light" | "dark"
    // Window
    pub show_line_numbers: bool,
    pub show_invisibles: bool,
    pub show_page_guide: bool,
    pub page_guide_column: u32,
    pub show_indent_guides: bool,
    pub highlight_current_line: bool,
    pub wrap_lines: bool,
    pub show_status_bar: bool,
    pub show_navigation_bar: bool,
    pub split_vertical: bool,
    pub status_lines: bool,
    pub status_chars: bool,
    pub status_words: bool,
    pub status_location: bool,
    pub status_line: bool,
    pub status_column: bool,
    // Edit
    pub tab_width: u32,
    pub auto_expand_tab: bool,
    pub auto_indent: bool,
    pub highlight_braces: bool,
    pub highlight_selection_instance: bool,
    pub auto_trim_trailing_whitespace: bool,
    pub comment_after_indent: bool,
    pub space_after_comment: bool,
    pub balance_symbols: bool,
    // Format
    pub line_ending: String,
    pub encoding: String,
    pub encoding_bom: bool,
    pub encodings: Vec<String>, // iconv names, "-" = separator
    pub refer_to_declaration: bool,
    // other state
    pub recent_syntaxes: Vec<String>,
    pub find_regex: bool,
    pub find_ignore_case: bool,
    pub find_in_selection: bool,
    pub find_wrap: bool,
    pub find_incremental: bool,
    pub find_whole_words: bool,
    pub find_strict: bool,
    pub find_ignore_diacritics: bool,
    pub find_ignore_width: bool,
    pub find_dot_all: bool,
    pub find_anchors_lines: bool,
    pub find_unicode_boundaries: bool,
    pub find_unescape: bool,
    pub find_history: Vec<String>,
    pub replace_history: Vec<String>,
    pub inspector_pane: u32,
    pub file_browser_hidden: bool,
    // General (more)
    pub nothing_open: String,    // "new" | "open" | "none"
    pub external_change: String, // "keep" | "ask" | "update"
    pub suppress_inconsistent_endings: bool,
    // Appearance (more)
    pub font_antialias: bool,
    pub font_ligatures: bool,
    pub mono_antialias: bool,
    pub mono_ligatures: bool,
    pub pin_theme_appearance: bool,
    pub opaque_bars: bool,
    pub editor_opacity: f64,
    // Window (more)
    pub window_width: u32, // 0 = Auto
    pub window_height: u32,
    pub invisible_newline: bool,
    pub invisible_tab: bool,
    pub invisible_space: bool,
    pub invisible_other_whitespace: bool,
    pub invisible_control: bool,
    pub hanging_indent: bool,
    pub hanging_indent_width: u32,
    pub writing_direction: String, // "ltr" | "rtl"
    pub overscroll: f64,
    // Edit (more)
    pub detect_indent: bool,
    pub indent_with_tab_key: bool,
    pub trim_whitespace_only_lines: bool,
    pub link_urls: bool,
    pub instance_delay: f64,
    // Mode, Format, Snippets, Shortcuts, Integration
    pub modes: BTreeMap<String, Mode>,
    pub default_syntax: String,
    pub snippets: Vec<Snippet>,
    pub file_drops: Vec<FileDrop>,
    pub shortcuts: BTreeMap<String, Vec<String>>,
    pub rmate: bool,
    pub rmate_port: u16,
    pub settings_pane: u32,
    pub console_zoom: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            reopen_windows: true,
            auto_save: true,
            font: "Sans 13".into(),
            mono_font: "Monospace 12".into(),
            line_height: 1.3,
            theme: crate::theme::DEFAULT_LIGHT.into(),
            appearance: "default".into(),
            show_line_numbers: true,
            show_invisibles: false,
            show_page_guide: false,
            page_guide_column: 80,
            show_indent_guides: false,
            highlight_current_line: false,
            wrap_lines: true,
            show_status_bar: true,
            show_navigation_bar: true,
            split_vertical: false,
            status_lines: true,
            status_chars: true,
            status_words: false,
            status_location: true,
            status_line: true,
            status_column: false,
            tab_width: 4,
            auto_expand_tab: false,
            auto_indent: true,
            highlight_braces: true,
            highlight_selection_instance: true,
            auto_trim_trailing_whitespace: false,
            comment_after_indent: true,
            space_after_comment: true,
            balance_symbols: true,
            line_ending: "LF".into(),
            encoding: "UTF-8".into(),
            encoding_bom: false,
            encodings: crate::document::DEFAULT_ENCODINGS
                .iter()
                .map(|e| e.map(|(_, i)| i.to_string()).unwrap_or_else(|| "-".into()))
                .collect(),
            refer_to_declaration: true,
            recent_syntaxes: Vec::new(),
            find_regex: false,
            find_ignore_case: false,
            find_in_selection: false,
            find_wrap: true,
            find_incremental: true,
            find_whole_words: false,
            find_strict: false,
            find_ignore_diacritics: false,
            find_ignore_width: false,
            find_dot_all: false,
            find_anchors_lines: true,
            find_unicode_boundaries: false,
            find_unescape: true,
            find_history: Vec::new(),
            replace_history: Vec::new(),
            inspector_pane: 0,
            file_browser_hidden: false,
            nothing_open: "new".into(),
            external_change: "update".into(),
            suppress_inconsistent_endings: false,
            font_antialias: true,
            font_ligatures: true,
            mono_antialias: true,
            mono_ligatures: false,
            pin_theme_appearance: false,
            opaque_bars: false,
            editor_opacity: 1.0,
            window_width: 0,
            window_height: 0,
            invisible_newline: true,
            invisible_tab: true,
            invisible_space: false,
            invisible_other_whitespace: true,
            invisible_control: true,
            hanging_indent: true,
            hanging_indent_width: 0,
            writing_direction: "ltr".into(),
            overscroll: 0.0,
            detect_indent: true,
            indent_with_tab_key: false,
            trim_whitespace_only_lines: false,
            link_urls: false,
            instance_delay: 0.5,
            modes: BTreeMap::from([("General".to_string(), Mode::general()), ("Code".to_string(), Mode::code())]),
            default_syntax: String::new(),
            snippets: Vec::new(),
            file_drops: default_file_drops(),
            shortcuts: BTreeMap::new(),
            rmate: false,
            rmate_port: 52698,
            settings_pane: 0,
            console_zoom: 1.0,
        }
    }
}

fn path() -> PathBuf {
    gtk::glib::user_config_dir().join("proza").join("settings.json")
}

thread_local! {
    static CURRENT: RefCell<Settings> = RefCell::new(load());
}

fn load() -> Settings {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// A copy of the current settings.
pub fn get() -> Settings {
    CURRENT.with(|c| c.borrow().clone())
}

/// Change settings and save them.
pub fn update(f: impl FnOnce(&mut Settings)) {
    CURRENT.with(|c| {
        f(&mut c.borrow_mut());
        let p = path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(&*c.borrow()) {
            let tmp = p.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(tmp, p);
            }
        }
    });
}

/// The candidate encodings for detection, in priority order (separators dropped).
pub fn candidate_encodings() -> Vec<String> {
    get().encodings.into_iter().filter(|e| e != "-").collect()
}

/// The mode for a syntax: its own mode if there is one, else General (prose syntaxes) or Code.
pub fn mode_for(syntax: &str) -> Mode {
    let s = get();
    if let Some(m) = s.modes.get(syntax) {
        return m.clone();
    }
    let kind = if crate::window::GENERAL_SYNTAXES.contains(&syntax) { "General" } else { "Code" };
    s.modes.get(kind).cloned().unwrap_or_else(|| if kind == "General" { Mode::general() } else { Mode::code() })
}

/// "General" or "Code".
pub fn kind_of(syntax: &str) -> &'static str {
    if crate::window::GENERAL_SYNTAXES.contains(&syntax) {
        "General"
    } else {
        "Code"
    }
}
