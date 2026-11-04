// User settings with CotEditor's defaults, stored as JSON in ~/.config/proza/settings.json.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::path::PathBuf;

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
