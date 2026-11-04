// CotEditor's main menu (docs/coteditor-ui-spec.md, "Main Menu") as one menu model for the title bar's menu
// button, plus the keyboard shortcuts. macOS keys map as: ⌘ → Ctrl, ⇧ → Shift, ⌥ → Alt; ⌃ combinations go to
// free Ctrl+Alt / Alt combinations (noted per item). Items whose action isn't registered show greyed out.

use gtk::gio;

/// (action, label, accelerators)
type Item = (&'static str, &'static str, &'static [&'static str]);

fn section(items: &[Item]) -> gio::Menu {
    let m = gio::Menu::new();
    for (action, label, _) in items {
        m.append(Some(label), Some(action));
    }
    m
}

fn add_sections(menu: &gio::Menu, sections: &[&[Item]]) {
    for s in sections {
        menu.append_section(None, &section(s));
    }
}

fn labelled(label: &str, items: &[Item]) -> gio::Menu {
    let m = gio::Menu::new();
    m.append_section(Some(label), &section(items));
    m
}

// ---------------------------------------------------------------- the menus, in CotEditor's order

pub const FILE: &[&[Item]] = &[
    &[("app.new", "New", &["<Ctrl>n"]), ("app.new-window", "New Window", &["<Ctrl><Shift>n"])],
    &[("app.open", "Open…", &["<Ctrl>o"])],
    &[("win.close", "Close", &["<Ctrl>w"]), ("win.save", "Save", &["<Ctrl>s"]),
      ("win.save-as", "Save As…", &["<Ctrl><Shift>s"]), ("win.revert", "Revert to Saved", &[])],
    &[("win.show-in-files", "Show in Files", &[]), ("win.copy-path", "Copy File Path", &[])],
    &[("win.page-setup", "Page Setup…", &["<Ctrl><Shift>p"]), ("win.print", "Print…", &["<Ctrl>p"])],
];

pub const EDIT: &[&[Item]] = &[
    &[("win.undo", "Undo", &["<Ctrl>z"]), ("win.redo", "Redo", &["<Ctrl><Shift>z"])],
    &[("win.cut", "Cut", &["<Ctrl>x"]), ("win.copy", "Copy", &["<Ctrl>c"]),
      ("win.copy-rich", "Copy as Rich Text", &["<Ctrl><Shift>c"]), ("win.paste", "Paste", &["<Ctrl>v"]),
      ("win.paste-exactly", "Paste Exactly", &["<Ctrl><Alt>v"]), ("win.delete", "Delete", &[])],
];
pub const EDIT_SELECT: &[&[Item]] = &[
    &[("win.select-all", "Select All", &["<Ctrl>a"]), ("win.select-line", "Select Line", &["<Ctrl><Shift>a"]),
      ("win.select-word", "Select Word", &["<Ctrl>d"]), ("win.select-enclosing", "Select Enclosing Symbols", &[])],
    &[("win.select-column-up", "Select Column Up", &[]), ("win.select-column-down", "Select Column Down", &[])],
    &[("win.split-selection", "Split Selection by Lines", &[])],
];
pub const EDIT_REST: &[&[Item]] = &[
    &[("win.complete", "Complete", &["F5"])],
    &[("win.insert-unicode", "Insert by Unicode Code Point…", &[]), ("win.emoji", "Emoji & Symbols", &["<Ctrl>period"])],
];
pub const EDIT_SUBSTITUTIONS: &[&[Item]] = &[
    &[("win.straighten-quotes", "Straighten Quotes", &[]), ("win.replace-quotes", "Replace Quotes", &[]),
      ("win.replace-dashes", "Replace Dashes", &[])],
];

pub const FORMAT_TOP: &[&[Item]] = &[&[("win.reset-syntax", "Reset Syntax Analysis", &["<Ctrl><Alt>r"])]];
pub const FORMAT_FONT: &[&[Item]] = &[
    &[("win.show-fonts", "Show Fonts", &[])],
    &[("win.bigger", "Bigger", &["<Ctrl>plus", "<Ctrl>equal", "<Ctrl>KP_Add"]),
      ("win.smaller", "Smaller", &["<Ctrl>minus", "<Ctrl>KP_Subtract"]),
      ("win.reset-zoom", "Reset to Default", &["<Ctrl>0"])],
    &[("win.antialias", "Antialias", &[]), ("win.ligatures", "Ligatures", &[])],
];
pub const FORMAT_TABS: &[&[Item]] = &[
    &[("win.tab-width::2", "2", &[]), ("win.tab-width::4", "4", &[]), ("win.tab-width::8", "8", &[]),
      ("win.custom-tab-width", "Custom…", &[])],
    &[("win.soft-tabs", "Use Spaces for Indentation", &[])],
];
pub const FORMAT_TOGGLES: &[&[Item]] = &[
    &[("win.invisibles", "Show Invisibles", &[]), ("win.indent-guides", "Show Indent Guides", &[]),
      ("win.page-guide", "Show Page Guide", &[])],
];
pub const FORMAT_WRAP: &[&[Item]] = &[&[("win.wrap", "Wrap Lines", &[])]];
pub const FORMAT_LOCK: &[&[Item]] = &[&[("win.prevent-editing", "Prevent Editing", &[])]];

pub const VIEW_INSPECTOR: &[&[Item]] = &[
    &[("win.inspector-pane::0", "Document Inspector", &[]), ("win.inspector-pane::1", "Outline", &[]),
      ("win.inspector-pane::2", "Warnings", &[])],
    &[("win.inspector", "Show Inspector", &["<Ctrl>i"])],
];
pub const VIEW_SIDEBAR: &[&[Item]] = &[&[("win.sidebar", "Show Sidebar", &["<Ctrl><Alt>s"])]];
pub const VIEW: &[&[Item]] = &[
    &[("win.navigation-bar", "Show Navigation Bar", &[]), ("win.line-numbers", "Show Line Numbers", &[]),
      ("win.status-bar", "Show Status Bar", &[])],
    // ⌃` / ⌃⇧` → Alt+` / Alt+Shift+`
    &[("win.focus-next-editor", "Move Focus to Next Editor", &["<Alt>grave"]),
      ("win.focus-previous-editor", "Move Focus to Previous Editor", &["<Alt><Shift>asciitilde"]),
      ("win.split-editor", "Split Editor", &[]), ("win.close-split", "Close Split Editor", &[]),
      ("win.split-orientation", "Stack Editors Vertically", &[])],
    &[("win.fullscreen", "Enter Full Screen", &["F11"])],
];

pub const TEXT_COMMENT: &[&[Item]] = &[
    &[("win.toggle-comment", "Toggle Comment", &["<Ctrl>slash"])],
    &[("win.inline-comment", "Inline Comment", &[]), ("win.block-comment", "Block Comment", &[]),
      ("win.uncomment", "Uncomment", &[])],
];
pub const TEXT_INDENT: &[&[Item]] = &[
    &[("win.shift-right", "Shift Right", &["<Ctrl>bracketright"]), ("win.shift-left", "Shift Left", &["<Ctrl>bracketleft"])],
    &[("win.indent-to-spaces", "Convert Indentation to Spaces", &[]), ("win.indent-to-tabs", "Convert Indentation to Tabs", &[])],
];
pub const TEXT_SURROUND: &[&[Item]] = &[
    &[("win.surround::'|'", "Single Quotes", &["<Ctrl>apostrophe"]),
      ("win.surround::\"|\"", "Double Quotes", &["<Ctrl><Shift>quotedbl"]),
      ("win.surround::(|)", "Parentheses", &["<Ctrl><Shift>parenleft"]),
      ("win.surround::{|}", "Braces", &[]), ("win.surround::[|]", "Brackets", &[])],
    &[("win.surround-custom", "Custom…", &["<Ctrl><Shift>l"])],
];
pub const TEXT_TRIM: &[&[Item]] = &[&[("win.trim-trailing", "Trim Trailing Whitespace", &[])]];
pub const TEXT_LINES: &[&[Item]] = &[
    // ⌃⌘↑/↓ → Alt+↑/↓
    &[("win.move-line-up", "Move Up", &["<Alt>Up"]), ("win.move-line-down", "Move Down", &["<Alt>Down"])],
    &[("win.sort-lines", "Sort", &[]), ("win.sort-pattern", "Sort by Pattern…", &[]),
      ("win.reverse-lines", "Reverse", &[]), ("win.shuffle-lines", "Shuffle", &[]),
      ("win.delete-duplicates", "Delete Duplicates", &[])],
    &[("win.duplicate-line", "Duplicate Line", &["<Ctrl><Shift>d"]), ("win.delete-line", "Delete Line", &[]),
      ("win.join-lines", "Join Lines", &["<Ctrl><Shift>j"])],
];
pub const TEXT_TRANSFORM: &[&[Item]] = &[
    &[("win.transform::upper", "Make Upper Case", &[]), ("win.transform::lower", "Make Lower Case", &[]),
      ("win.transform::capitalize", "Capitalize", &[]), ("win.transform::fullwidth", "Convert to Full Width", &[]),
      ("win.transform::halfwidth", "Convert to Half Width", &[]),
      ("win.transform::alnum-full", "Convert Alphanumeric Text to Full Width", &[]),
      ("win.transform::alnum-half", "Convert Alphanumeric Text to Half Width", &[])],
    &[("win.transform::url-encode", "URL Encode", &[]), ("win.transform::url-decode", "URL Decode", &[])],
];
pub const TEXT_NORMALIZE: &[&[Item]] = &[
    &[("win.normalize::nfd", "NFD", &[]), ("win.normalize::nfc", "NFC", &[]), ("win.normalize::nfkd", "NFKD", &[]),
      ("win.normalize::nfkc", "NFKC", &[]), ("win.normalize::nfkc-casefold", "NFKC Case-Fold", &[])],
];
pub const TEXT_END: &[&[Item]] = &[
    &[("win.inspect-character", "Inspect Character", &["<Ctrl><Alt>i"]),
      ("win.character-count", "Advanced Character Count…", &[]),
      ("win.color-code", "Edit Color Code…", &[])],
];

pub const FIND: &[&[Item]] = &[
    &[("win.find", "Find…", &["<Ctrl>f"]), ("win.find-next", "Find Next", &["<Ctrl>g"]),
      ("win.find-previous", "Find Previous", &["<Ctrl><Shift>g"]), ("win.find-all", "Find All", &["<Ctrl><Shift>f"]),
      ("win.select-all-matches", "Select All Find Matches", &[])],
    &[("win.use-for-find", "Use Selection for Find", &["<Ctrl>e"]),
      ("win.use-for-replace", "Use Selection for Replace", &["<Ctrl><Shift>e"]),
      ("win.jump-to-selection", "Jump to Selection", &["<Ctrl>j"])],
    &[("win.replace", "Replace", &[]), ("win.replace-and-find", "Replace and Find Next", &["<Ctrl><Alt>g"]),
      ("win.replace-all", "Replace All", &[])],
    &[("win.highlight-all", "Highlight All", &["<Ctrl><Shift>h"]),
      ("win.remove-highlights", "Remove Highlights", &["<Ctrl><Alt><Shift>h"])],
    // ⌥⌘↑/↓ → Ctrl+Alt+↑/↓
    &[("win.open-outline", "Open Outline Menu", &[]),
      ("win.outline-previous", "Select Previous Outline Item", &["<Ctrl><Alt>Up"]),
      ("win.outline-next", "Select Next Outline Item", &["<Ctrl><Alt>Down"])],
    &[("win.go-to-line", "Go to Line…", &["<Ctrl>l"])],
];

pub const WINDOW: &[&[Item]] = &[&[("win.console", "Console", &[])]];
pub const HELP: &[&[Item]] = &[
    &[("app.help", "Proza Help", &["F1"])],
    &[("app.report-issue", "Report Issue", &[]), ("app.website", "Proza Web Site", &[])],
];
pub const APP: &[&[Item]] = &[
    &[("app.settings", "Settings…", &["<Ctrl>comma"])],
    &[("app.about", "About Proza", &[])],
    &[("app.quit", "Quit Proza", &["<Ctrl>q"])],
];

/// Submenus filled at run time (encodings, syntaxes, themes, line endings, snippets, scripts, outline...).
pub struct Dynamic {
    pub line_endings: gio::Menu,
    pub encodings: gio::Menu,
    pub syntaxes: gio::Menu,
    pub themes: gio::Menu,
    pub scripts: gio::Menu,
    pub recent: gio::Menu,
}

/// The whole menu: File, Edit, Format, View, Text, Find, Window, Script, Help, then the app items.
pub fn build() -> (gio::Menu, Dynamic) {
    let dynm = Dynamic {
        line_endings: gio::Menu::new(),
        encodings: gio::Menu::new(),
        syntaxes: gio::Menu::new(),
        themes: gio::Menu::new(),
        scripts: gio::Menu::new(),
        recent: gio::Menu::new(),
    };
    let root = gio::Menu::new();

    let file = gio::Menu::new();
    add_sections(&file, &FILE[..1]);
    let open = gio::Menu::new();
    open.append(Some("Open…"), Some("app.open"));
    open.append_submenu(Some("Open Recent"), &dynm.recent);
    file.append_section(None, &open);
    add_sections(&file, &FILE[2..]);
    root.append_submenu(Some("File"), &file);

    let edit = gio::Menu::new();
    add_sections(&edit, EDIT);
    let select = gio::Menu::new();
    add_sections(&select, EDIT_SELECT);
    let s = gio::Menu::new();
    s.append_submenu(Some("Select"), &select);
    edit.append_section(None, &s);
    add_sections(&edit, EDIT_REST);
    let subst = gio::Menu::new();
    add_sections(&subst, EDIT_SUBSTITUTIONS);
    let s = gio::Menu::new();
    s.append_submenu(Some("Substitutions"), &subst);
    edit.append_section(None, &s);
    root.append_submenu(Some("Edit"), &edit);

    let format = gio::Menu::new();
    let s = gio::Menu::new();
    s.append_submenu(Some("Line Endings"), &dynm.line_endings);
    s.append_submenu(Some("Text Encoding"), &dynm.encodings);
    s.append_submenu(Some("Syntax"), &dynm.syntaxes);
    for (a, l, _) in FORMAT_TOP[0] {
        s.append(Some(l), Some(a));
    }
    format.append_section(None, &s);
    let font = gio::Menu::new();
    font.append_section(None, &section(FORMAT_FONT[0]));
    let ftype = gio::Menu::new();
    ftype.append(Some("Standard"), Some("win.font-type::standard"));
    ftype.append(Some("Monospaced"), Some("win.font-type::monospaced"));
    font.append_section(Some("Font Type"), &ftype);
    add_sections(&font, &FORMAT_FONT[1..]);
    let tabs = gio::Menu::new();
    add_sections(&tabs, FORMAT_TABS);
    let s = gio::Menu::new();
    s.append_submenu(Some("Font"), &font);
    s.append_submenu(Some("Theme"), &dynm.themes);
    format.append_section(None, &s);
    let s = gio::Menu::new();
    s.append_submenu(Some("Tab Width"), &tabs);
    for (a, l, _) in FORMAT_TOGGLES[0] {
        s.append(Some(l), Some(a));
    }
    format.append_section(None, &s);
    let s = gio::Menu::new();
    s.append(Some("Wrap Lines"), Some("win.wrap"));
    format.append_section(None, &s);
    add_sections(&format, FORMAT_LOCK);
    root.append_submenu(Some("Format"), &format);

    let view = gio::Menu::new();
    let s = gio::Menu::new();
    let sidebar = gio::Menu::new();
    sidebar.append(Some("File Browser"), Some("win.sidebar"));
    let inspector = gio::Menu::new();
    add_sections(&inspector, VIEW_INSPECTOR);
    s.append_submenu(Some("Sidebar"), &sidebar);
    s.append_submenu(Some("Inspector"), &inspector);
    view.append_section(None, &s);
    add_sections(&view, VIEW);
    root.append_submenu(Some("View"), &view);

    let text = gio::Menu::new();
    let s = gio::Menu::new();
    for (name, items) in [("Comment", TEXT_COMMENT), ("Indentation", TEXT_INDENT), ("Surround Selection With", TEXT_SURROUND)] {
        let m = gio::Menu::new();
        add_sections(&m, items);
        s.append_submenu(Some(name), &m);
    }
    for (a, l, _) in TEXT_TRIM[0] {
        s.append(Some(l), Some(a));
    }
    text.append_section(None, &s);
    let s = gio::Menu::new();
    for (name, items) in [("Lines", TEXT_LINES), ("Transformations", TEXT_TRANSFORM), ("Unicode Normalizations", TEXT_NORMALIZE)] {
        let m = gio::Menu::new();
        add_sections(&m, items);
        s.append_submenu(Some(name), &m);
    }
    text.append_section(None, &s);
    add_sections(&text, TEXT_END);
    root.append_submenu(Some("Text"), &text);

    let find = gio::Menu::new();
    add_sections(&find, FIND);
    root.append_submenu(Some("Find"), &find);

    let window = gio::Menu::new();
    add_sections(&window, WINDOW);
    root.append_submenu(Some("Window"), &window);

    let scripts = gio::Menu::new();
    scripts.append_section(None, &dynm.scripts);
    scripts.append_section(None, &section(&[("app.open-scripts-folder", "Open Scripts Folder", &[])]));
    root.append_submenu(Some("Script"), &scripts);

    let help = gio::Menu::new();
    add_sections(&help, HELP);
    root.append_submenu(Some("Help"), &help);

    add_sections(&root, APP);
    let _ = labelled; // (kept for section headers in dynamic menus)
    (root, dynm)
}

/// Every keyboard shortcut of the menus: (detailed action, accels).
pub fn accels() -> Vec<(&'static str, &'static [&'static str])> {
    let all: &[&[&[Item]]] = &[
        FILE, EDIT, EDIT_SELECT, EDIT_REST, EDIT_SUBSTITUTIONS, FORMAT_TOP, FORMAT_FONT, FORMAT_TABS, FORMAT_TOGGLES,
        FORMAT_WRAP, FORMAT_LOCK, VIEW_INSPECTOR, VIEW_SIDEBAR, VIEW, TEXT_COMMENT, TEXT_INDENT, TEXT_SURROUND,
        TEXT_TRIM, TEXT_LINES, TEXT_TRANSFORM, TEXT_NORMALIZE, TEXT_END, FIND, WINDOW, HELP, APP,
    ];
    let mut out = Vec::new();
    for menu in all {
        for sec in *menu {
            for (a, _, k) in *sec {
                if !k.is_empty() {
                    out.push((*a, *k));
                }
            }
        }
    }
    out
}
