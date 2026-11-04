// The Find & Replace panel: one app-wide floating panel acting on the last active document window, with
// CotEditor's fields, options, advanced options, Find All result list and search semantics.

use crate::document::LineEnding;
use crate::settings::{self, Settings};
use crate::window::DocWindow;
use adw::prelude::*;
use gtk::{gdk, gio, glib, pango};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

// ================================================================ search engine (pure)

/// One match in char offsets, with its capture groups.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub start: usize,
    pub end: usize,
    pub groups: Vec<Option<(usize, usize)>>,
}

pub struct Matcher {
    re: fancy_regex::Regex,
    regex_mode: bool,
    fold_diacritics: bool,
    fold_width: bool,
}

/// ICU's `\R` (any line break), which the regex engine doesn't know.
fn translate_line_break(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    let mut chars = p.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('R') => out.push_str(r"(?:\r\n|[\n\x0B\x0C\r\x{85}\x{2028}\x{2029}])"),
                Some(n) => {
                    out.push('\\');
                    out.push(n);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn fold_char(c: char, diacritics: bool, width: bool, out: &mut String) {
    let c = match c {
        '\u{FF01}'..='\u{FF5E}' if width => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
        '\u{3000}' if width => ' ',
        c => c,
    };
    if diacritics {
        out.extend(std::iter::once(c).nfd().filter(|d| !is_combining_mark(*d)));
    } else {
        out.push(c);
    }
}

/// The text with diacritics and/or width folded, and for each folded char the original char offset.
fn fold(text: &str, diacritics: bool, width: bool) -> (String, Vec<usize>) {
    let mut out = String::with_capacity(text.len());
    let mut map = Vec::with_capacity(text.len());
    let mut piece = String::new();
    for (i, c) in text.chars().enumerate() {
        piece.clear();
        fold_char(c, diacritics, width, &mut piece);
        for f in piece.chars() {
            out.push(f);
            map.push(i);
        }
    }
    map.push(text.chars().count());
    (out, map)
}

impl Matcher {
    /// `None` for an empty find string; `Err` carries the regex engine's reason.
    pub fn new(find: &str, s: &Settings, ending: LineEnding) -> Result<Option<Matcher>, String> {
        if find.is_empty() {
            return Ok(None);
        }
        let find = crate::document::convert_line_endings(find, ending);
        let regex_mode = s.find_regex;
        let (fold_diacritics, fold_width) = (!regex_mode && s.find_ignore_diacritics, !regex_mode && s.find_ignore_width);
        let mut flags = String::new();
        if s.find_ignore_case {
            flags.push('i');
        }
        let body = if regex_mode {
            if s.find_anchors_lines {
                flags.push('m');
            }
            if s.find_dot_all {
                flags.push('s');
            }
            translate_line_break(&find)
        } else {
            let (folded, _) = fold(&find, fold_diacritics, fold_width);
            let lit = regex::escape(&folded);
            if s.find_whole_words {
                format!(r"\b{lit}\b")
            } else {
                lit
            }
        };
        let pattern = if flags.is_empty() { body } else { format!("(?{flags}){body}") };
        let re = fancy_regex::Regex::new(&pattern).map_err(|e| e.to_string())?;
        Ok(Some(Matcher { re, regex_mode, fold_diacritics, fold_width }))
    }

    pub fn group_count(&self) -> usize {
        self.re.captures_len().saturating_sub(1)
    }

    /// All matches in `text`, in order (empty matches skipped, at most 100,000).
    pub fn find_all(&self, text: &str) -> Vec<Found> {
        let folding = self.fold_diacritics || self.fold_width;
        let (hay, map) = if folding { fold(text, self.fold_diacritics, self.fold_width) } else { (String::new(), Vec::new()) };
        let hay = if folding { hay.as_str() } else { text };
        // byte offset → char offset in `hay`
        let byte_to_char: Vec<usize> = {
            let mut v = vec![0; hay.len() + 1];
            let mut n = 0;
            for (b, _) in hay.char_indices() {
                v[b] = n;
                n += 1;
            }
            v[hay.len()] = n;
            v
        };
        let to_orig = |b: usize| {
            let c = byte_to_char[b];
            if folding {
                map[c]
            } else {
                c
            }
        };
        let to_orig_end = |b: usize| {
            let c = byte_to_char[b];
            if folding {
                if c == 0 {
                    0
                } else {
                    map[c - 1] + 1
                }
            } else {
                c
            }
        };
        let mut out = Vec::new();
        for caps in self.re.captures_iter(hay).flatten().take(100_000) {
            let Some(m) = caps.get(0) else { continue };
            if m.start() == m.end() {
                continue;
            }
            let groups = (1..caps.len()).map(|i| caps.get(i).map(|g| (to_orig(g.start()), to_orig_end(g.end())))).collect();
            out.push(Found { start: to_orig(m.start()), end: to_orig_end(m.end()), groups });
        }
        out
    }

    /// The replacement for `found`: `$n` expanded (ICU rules: `\$` is a literal dollar), escapes unescaped first
    /// when the option is on. Outside regex mode the replacement is literal.
    pub fn replacement(&self, template: &str, text_chars: &[char], found: &Found, unescape: bool) -> String {
        if !self.regex_mode {
            return template.to_string();
        }
        let group = |i: usize| -> String {
            let range = if i == 0 { Some((found.start, found.end)) } else { found.groups.get(i - 1).copied().flatten() };
            range.map(|(a, b)| text_chars[a..b].iter().collect()).unwrap_or_default()
        };
        let chars: Vec<char> = template.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '\\' && i + 1 < chars.len() {
                let n = chars[i + 1];
                let esc = match n {
                    '$' => Some('$'),
                    '\\' if unescape => Some('\\'),
                    't' if unescape => Some('\t'),
                    'n' if unescape => Some('\n'),
                    'r' if unescape => Some('\r'),
                    '0' if unescape => Some('\0'),
                    '"' if unescape => Some('"'),
                    '\'' if unescape => Some('\''),
                    _ => None,
                };
                if let Some(e) = esc {
                    out.push(e);
                    i += 2;
                    continue;
                }
            }
            if c == '$' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                // the longest number that is still a valid group
                let mut j = i + 1;
                let mut n = 0usize;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    let next = n * 10 + chars[j].to_digit(10).unwrap_or(0) as usize;
                    if j > i + 1 && next > self.group_count() {
                        break;
                    }
                    n = next;
                    j += 1;
                }
                out.push_str(&group(n));
                i = j;
                continue;
            }
            out.push(c);
            i += 1;
        }
        out
    }
}

// ================================================================ panel

pub struct Panel {
    win: adw::Window,
    find: gtk::TextView,
    replace: gtk::TextView,
    find_result: gtk::Label,
    replace_result: gtk::Label,
    results: gtk::Revealer,
    results_title: gtk::Label,
    results_find: gtk::Label,
    results_list: gtk::ListBox,
    regex_check: gtk::CheckButton,
    incremental_id: RefCell<Option<glib::SourceId>>,
    /// The document the Find All result list belongs to, and its matches.
    listed: RefCell<(Weak<DocWindow>, Vec<Found>)>,
    /// The document Find Next/Previous last reported a position for, so "3/12" can fall back to "12 matches".
    target: RefCell<Weak<DocWindow>>,
}

thread_local! {
    static PANEL: RefCell<Option<Rc<Panel>>> = const { RefCell::new(None) };
    static TARGET: RefCell<Weak<DocWindow>> = RefCell::new(Weak::new());
}

const FIND_TAG: &str = "proza-find";
const FIND_ALL_TAG: &str = "proza-find-all";

/// The document window the panel acts on: the last active one.
pub fn set_target(w: &Rc<DocWindow>) {
    TARGET.with(|t| *t.borrow_mut() = Rc::downgrade(w));
    if let Some(p) = PANEL.with(|p| p.borrow().clone()) {
        p.win.insert_action_group("win", Some(&w.actions));
        p.win.set_transient_for(Some(&w.win));
    }
}

fn target() -> Option<Rc<DocWindow>> {
    TARGET.with(|t| t.borrow().upgrade())
}

fn panel() -> Rc<Panel> {
    if let Some(p) = PANEL.with(|p| p.borrow().clone()) {
        return p;
    }
    let p = build();
    PANEL.with(|cell| *cell.borrow_mut() = Some(p.clone()));
    if let Some(t) = target() {
        set_target(&t);
    }
    p
}

fn field_text(v: &gtk::TextView) -> String {
    let b = v.buffer();
    b.text(&b.start_iter(), &b.end_iter(), false).to_string()
}

fn set_field_text(v: &gtk::TextView, s: &str) {
    v.buffer().set_text(s);
}

/// A multi-line field in a rounded frame, with the history button at the leading top and result + clear at the
/// trailing top (only while the field has text).
fn field(placeholder: &str, tooltip: &str, icon: &str, kind: &'static str) -> (gtk::Widget, gtk::TextView, gtk::Label) {
    let view = gtk::TextView::builder().wrap_mode(gtk::WrapMode::None).accepts_tab(false).tooltip_text(tooltip).build();
    view.add_css_class("find-field");
    view.set_left_margin(34);
    view.set_right_margin(90);
    view.set_top_margin(7);
    view.set_bottom_margin(5);
    let scroller = gtk::ScrolledWindow::builder().child(&view).min_content_height(44).max_content_height(120).propagate_natural_height(true).build();
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&scroller));
    overlay.add_css_class("find-field-frame");

    let hint = gtk::Label::new(Some(placeholder));
    hint.add_css_class("dim-label");
    hint.set_halign(gtk::Align::Start);
    hint.set_valign(gtk::Align::Start);
    hint.set_margin_start(36);
    hint.set_margin_top(7);
    hint.set_can_target(false);
    overlay.add_overlay(&hint);

    let history = gtk::MenuButton::builder().icon_name(icon).halign(gtk::Align::Start).valign(gtk::Align::Start).build();
    history.add_css_class("flat");
    history.add_css_class("history-button");
    history.set_focusable(false);
    if let Some(inner) = history.first_child() {
        inner.set_focusable(false);
    }
    history.set_create_popup_func(move |mb| mb.set_menu_model(Some(&history_menu(kind))));
    overlay.add_overlay(&history);

    let result = gtk::Label::new(None);
    result.add_css_class("dim-label");
    result.add_css_class("numeric");
    result.add_css_class("caption");
    let clear = gtk::Button::builder().icon_name("proza-clear-symbolic").tooltip_text("Clear").build();
    clear.add_css_class("flat");
    clear.add_css_class("circular");
    clear.set_focusable(false);
    let trailing = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    trailing.append(&result);
    trailing.append(&clear);
    trailing.set_halign(gtk::Align::End);
    trailing.set_valign(gtk::Align::Start);
    trailing.set_visible(false);
    overlay.add_overlay(&trailing);
    {
        let v = view.clone();
        clear.connect_clicked(move |_| v.buffer().set_text(""));
    }
    view.buffer().connect_changed(move |b| {
        let empty = b.char_count() == 0;
        hint.set_visible(empty);
        trailing.set_visible(!empty);
    });
    (overlay.upcast(), view, result)
}

fn history(kind: &str) -> Vec<String> {
    let s = settings::get();
    if kind == "find" {
        s.find_history
    } else {
        s.replace_history
    }
}

fn history_menu(kind: &'static str) -> gio::Menu {
    let menu = gio::Menu::new();
    let recent = gio::Menu::new();
    for h in history(kind).iter().rev() {
        let mut label: String = h.chars().take(64).collect();
        if h.chars().count() > 64 {
            label.push('…');
        }
        let item = gio::MenuItem::new(Some(&label.replace('\n', "⏎")), None);
        item.set_action_and_target_value(Some(&format!("find.use-{kind}")), Some(&h.to_variant()));
        recent.append_item(&item);
    }
    menu.append_section(Some(if kind == "find" { "Recent Searches" } else { "Recent Replacements" }), &recent);
    let clear = gio::Menu::new();
    clear.append(
        Some(if kind == "find" { "Clear Recent Searches" } else { "Clear Recent Replacements" }),
        Some(&format!("find.clear-{kind}")),
    );
    menu.append_section(None, &clear);
    menu
}

fn add_history(kind: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    settings::update(|s| {
        let h = if kind == "find" { &mut s.find_history } else { &mut s.replace_history };
        h.retain(|x| x != value);
        h.push(value.to_string());
        let n = h.len();
        if n > 20 {
            h.drain(..n - 20);
        }
    });
}

fn check(label: &str, tooltip: &str, get: fn(&Settings) -> bool, set: fn(&mut Settings, bool)) -> gtk::CheckButton {
    let c = gtk::CheckButton::builder().label(label).active(get(&settings::get())).build();
    if !tooltip.is_empty() {
        c.set_tooltip_text(Some(tooltip));
    }
    c.connect_toggled(move |c| {
        let on = c.is_active();
        settings::update(|s| set(s, on));
    });
    c
}

fn advanced_popover(regex_check: &gtk::CheckButton) -> gtk::Popover {
    let bx = gtk::Box::new(gtk::Orientation::Vertical, 4);
    bx.set_margin_top(8);
    bx.set_margin_bottom(8);
    bx.set_margin_start(8);
    bx.set_margin_end(8);
    let heading = |t: &str, dim: bool| {
        let l = gtk::Label::builder().label(t).xalign(0.0).build();
        l.add_css_class("heading");
        if dim {
            l.add_css_class("dim-label");
        }
        l.set_margin_top(6);
        l
    };
    bx.append(&heading("Advanced Find Options", true));
    bx.append(&check("Wrap search around", "", |s| s.find_wrap, |s, v| s.find_wrap = v));
    bx.append(&check("Search incrementally", "", |s| s.find_incremental, |s, v| s.find_incremental = v));
    let textual = gtk::Box::new(gtk::Orientation::Vertical, 2);
    textual.append(&heading("Textual Search", false));
    textual.append(&check("Match only whole words", "Restrict search results to whole words.", |s| s.find_whole_words, |s, v| s.find_whole_words = v));
    textual.append(&check("Distinguish characters strictly", "Exact character-by-character equivalence.", |s| s.find_strict, |s, v| s.find_strict = v));
    textual.append(&check("Ignore diacritical marks", "Search ignores diacritical marks (for example, ö = o).", |s| s.find_ignore_diacritics, |s, v| s.find_ignore_diacritics = v));
    textual.append(&check("Ignore width differences", "Search ignores width differences in character forms (for example, ａ = a).", |s| s.find_ignore_width, |s, v| s.find_ignore_width = v));
    let regex = gtk::Box::new(gtk::Orientation::Vertical, 2);
    regex.append(&heading("Regular Expression Search", false));
    regex.append(&check("Dot matches line separators", "Allow . to match any character, including newline characters (singleline).", |s| s.find_dot_all, |s, v| s.find_dot_all = v));
    regex.append(&check("Anchors match lines", "Allow ^ and $ to match the start and end of lines (multiline).", |s| s.find_anchors_lines, |s, v| s.find_anchors_lines = v));
    regex.append(&check("Use Unicode word boundaries", "Use Unicode TR#29 to specify word boundaries", |s| s.find_unicode_boundaries, |s, v| s.find_unicode_boundaries = v));
    regex.append(&check("Unescape replacement text", "Unescape metacharacters with backslash in replacement text.", |s| s.find_unescape, |s, v| s.find_unescape = v));
    bx.append(&textual);
    bx.append(&regex);
    let sync = move |on: bool| {
        textual.set_sensitive(!on);
        regex.set_sensitive(on);
    };
    sync(regex_check.is_active());
    regex_check.connect_toggled(move |c| sync(c.is_active()));
    let pop = gtk::Popover::builder().child(&bx).position(gtk::PositionType::Right).build();
    pop
}

fn regex_reference() -> gtk::Popover {
    let grid = gtk::Grid::builder().column_spacing(10).row_spacing(2).margin_top(10).margin_bottom(10).margin_start(12).margin_end(12).build();
    let title = gtk::Label::builder().label("Basic Regular Expression Syntax").xalign(0.0).build();
    title.add_css_class("heading");
    grid.attach(&title, 0, 0, 5, 1);
    let columns: [&[(&str, &[(&str, &str)])]; 2] = [
        &[
            ("Characters", &[(".", "any character"), ("\\R", "new line character"), ("\\t", "tab character"), ("\\w", "word character"), ("\\s", "whitespace character"), ("\\S", "non-whitespace character"), ("\\d", "decimal number"), ("[A-Z]", "any character in range A to Z"), ("[^A-Z]", "any character not in range A to Z")]),
            ("Anchors", &[("^", "beginning of the line"), ("$", "end of the line"), ("\\b", "word boundary")]),
        ],
        &[
            ("Quantifiers", &[("?", "1 or 0 times"), ("*", "0 or more times"), ("+", "1 or more times"), ("{n,m}", "at least n but not more than m times"), ("{n,}", "at least n times"), ("{,n}", "at least 0 but not more than n times"), ("{n}", "n times")]),
            ("Extended Groups", &[("(?=subexp)", "look-ahead"), ("(?<=subexp)", "look-behind")]),
            ("Back Reference", &[("$1", "first match")]),
        ],
    ];
    for (ci, col) in columns.iter().enumerate() {
        let mut row = 1;
        for (section, entries) in col.iter() {
            let h = gtk::Label::builder().label(*section).xalign(0.0).margin_top(6).build();
            h.add_css_class("dim-label");
            h.add_css_class("caption-heading");
            grid.attach(&h, ci as i32 * 3, row, 2, 1);
            row += 1;
            for (term, desc) in entries.iter() {
                let t = gtk::Label::builder().label(*term).xalign(0.0).build();
                t.add_css_class("monospace");
                t.set_attributes(Some(&{
                    let a = pango::AttrList::new();
                    a.insert(pango::AttrInt::new_weight(pango::Weight::Medium));
                    a
                }));
                let d = gtk::Label::builder().label(*desc).xalign(0.0).build();
                d.add_css_class("caption");
                grid.attach(&t, ci as i32 * 3, row, 1, 1);
                grid.attach(&d, ci as i32 * 3 + 1, row, 1, 1);
                row += 1;
            }
        }
    }
    grid.attach(&gtk::Separator::new(gtk::Orientation::Vertical), 2, 1, 1, 14);
    let foot = gtk::Label::builder().label("The syntax conforms to the ICU Regular Expressions specifications.").xalign(0.0).margin_top(8).build();
    foot.add_css_class("caption");
    grid.attach(&foot, 0, 20, 5, 1);
    gtk::Popover::builder().child(&grid).position(gtk::PositionType::Top).build()
}

fn build() -> Rc<Panel> {
    let win = adw::Window::builder().title("Find & Replace").default_width(470).hide_on_close(true).resizable(true).build();
    if let Some(app) = gio::Application::default().and_downcast::<gtk::Application>() {
        win.set_application(Some(&app));
    }
    win.add_css_class("find-panel");
    let header = adw::HeaderBar::new();
    header.set_show_start_title_buttons(false);
    header.add_css_class("flat");
    let title = gtk::Label::new(Some("Find & Replace"));
    title.add_css_class("caption-heading");
    header.set_title_widget(Some(&title));

    let (find_w, find, find_result) = field("Find", "Type the text to search for.", "system-search-symbolic", "find");
    let (replace_w, replace, replace_result) = field("Replace with", "Type the text to replace the found text.", "document-edit-symbolic", "replace");
    let s = settings::get();

    // option row
    let regex_check = check("Regular Expression", "Select to search with regular expression.", |s| s.find_regex, |s, v| s.find_regex = v);
    let help = gtk::MenuButton::builder().child(&gtk::Label::new(Some("?"))).tooltip_text("Show quick reference for regular expression syntax.").popover(&regex_reference()).build();
    help.add_css_class("circular");
    help.add_css_class("help-button");
    let ignore_case = check("Ignore Case", "Select to ignore character case on search.", |s| s.find_ignore_case, |s, v| s.find_ignore_case = v);
    let in_selection = check("In Selection", "Select to search text only from selection.", |s| s.find_in_selection, |s, v| s.find_in_selection = v);
    let more = gtk::MenuButton::builder().icon_name("view-more-horizontal-symbolic").tooltip_text("Show advanced options").popover(&advanced_popover(&regex_check)).build();
    more.add_css_class("circular");
    more.update_property(&[gtk::accessible::Property::Label("Advanced options")]);
    let options = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    options.append(&regex_check);
    options.append(&help);
    options.append(&ignore_case);
    options.append(&in_selection);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    options.append(&spacer);
    options.append(&more);
    let _ = s;

    // Find All results
    let close_results = gtk::Button::builder().icon_name("go-up-symbolic").tooltip_text("Close find result.").build();
    close_results.add_css_class("flat");
    let results_title = gtk::Label::builder().xalign(0.0).build();
    results_title.add_css_class("heading");
    let results_head = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    results_head.append(&close_results);
    results_head.append(&results_title);
    let results_find = gtk::Label::builder().xalign(0.0).ellipsize(pango::EllipsizeMode::End).build();
    results_find.add_css_class("caption");
    let results_list = gtk::ListBox::new();
    results_list.add_css_class("find-results");
    results_list.set_selection_mode(gtk::SelectionMode::Single);
    let list_scroller = gtk::ScrolledWindow::builder().child(&results_list).min_content_height(200).vexpand(true).build();
    list_scroller.add_css_class("frame");
    let results_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    results_box.append(&results_head);
    results_box.append(&results_find);
    results_box.append(&list_scroller);
    let results = gtk::Revealer::builder().child(&results_box).reveal_child(false).transition_type(gtk::RevealerTransitionType::SlideDown).vexpand(true).build();

    // button row
    let find_all_menu = gio::Menu::new();
    find_all_menu.append(Some("Highlight All"), Some("win.highlight-all"));
    find_all_menu.append(Some("Select All"), Some("win.select-all-matches"));
    let find_all = adw::SplitButton::builder().label("Find All").tooltip_text("Find and list all matches.").menu_model(&find_all_menu).action_name("win.find-all").build();
    let replace_all = gtk::Button::builder().label("Replace All").tooltip_text("Replace all matches with the replacement text.").action_name("win.replace-all").build();
    let replace_btn = gtk::Button::builder().label("Replace").tooltip_text("Replace the current selection with the replacement text, then find the next match.").action_name("win.replace-and-find").build();
    let prev = gtk::Button::builder().icon_name("go-previous-symbolic").tooltip_text("Find previous match.").action_name("win.find-previous").build();
    let next = gtk::Button::builder().icon_name("go-next-symbolic").tooltip_text("Find next match.").action_name("win.find-next").build();
    let nav = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    nav.add_css_class("linked");
    nav.append(&prev);
    nav.append(&next);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.append(&find_all);
    buttons.append(&replace_all);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    buttons.append(&spacer);
    buttons.append(&replace_btn);
    buttons.append(&nav);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.set_margin_start(14);
    content.set_margin_end(14);
    content.set_margin_bottom(14);
    content.append(&find_w);
    content.append(&replace_w);
    content.append(&options);
    content.append(&results);
    content.append(&buttons);
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&header);
    tv.set_content(Some(&content));
    win.set_content(Some(&tv));

    let p = Rc::new(Panel {
        win,
        find,
        replace,
        find_result,
        replace_result,
        results,
        results_title,
        results_find,
        results_list,
        regex_check,
        incremental_id: RefCell::new(None),
        listed: RefCell::new((Weak::new(), Vec::new())),
        target: RefCell::new(Weak::new()),
    });

    // panel actions: history
    let group = gio::SimpleActionGroup::new();
    for kind in ["find", "replace"] {
        let use_ = gio::SimpleAction::new(&format!("use-{kind}"), Some(glib::VariantTy::STRING));
        let weak = Rc::downgrade(&p);
        use_.connect_activate(move |_, v| {
            if let (Some(p), Some(v)) = (weak.upgrade(), v.and_then(|v| v.get::<String>())) {
                set_field_text(if kind == "find" { &p.find } else { &p.replace }, &v);
            }
        });
        group.add_action(&use_);
        let clear = gio::SimpleAction::new(&format!("clear-{kind}"), None);
        clear.connect_activate(move |_, _| {
            settings::update(|s| if kind == "find" { s.find_history.clear() } else { s.replace_history.clear() });
        });
        group.add_action(&clear);
    }
    p.win.insert_action_group("find", Some(&group));

    // Return = Find Next, Shift+Return = Find Previous, Alt/Ctrl+Return = a newline; Return in Replace does nothing
    for (view, is_find) in [(p.find.clone(), true), (p.replace.clone(), false)] {
        let keys = gtk::EventControllerKey::new();
        keys.connect_key_pressed(move |_, key, _, state| {
            if key != gdk::Key::Return && key != gdk::Key::KP_Enter {
                return glib::Propagation::Proceed;
            }
            if state.intersects(gdk::ModifierType::ALT_MASK | gdk::ModifierType::CONTROL_MASK) {
                return glib::Propagation::Proceed;
            }
            if is_find {
                if let Some(t) = target() {
                    let back = state.contains(gdk::ModifierType::SHIFT_MASK);
                    gtk::prelude::ActionGroupExt::activate_action(&t.actions, if back { "find-previous" } else { "find-next" }, None);
                }
            }
            glib::Propagation::Stop
        });
        view.add_controller(keys);
    }
    // focus selects the whole field (like an entry)
    for view in [p.find.clone(), p.replace.clone()] {
        let focus = gtk::EventControllerFocus::new();
        let v = view.clone();
        focus.connect_enter(move |_| {
            let b = v.buffer();
            b.select_range(&b.start_iter(), &b.end_iter());
        });
        view.add_controller(focus);
    }
    // the find string changed: results go stale, incremental search runs
    {
        let weak = Rc::downgrade(&p);
        p.find.buffer().connect_changed(move |_| {
            if let Some(p) = weak.upgrade() {
                p.find_result.set_label("");
                schedule_incremental(&p);
            }
        });
        let weak = Rc::downgrade(&p);
        p.replace.buffer().connect_changed(move |_| {
            if let Some(p) = weak.upgrade() {
                p.replace_result.set_label("");
            }
        });
    }
    {
        let weak = Rc::downgrade(&p);
        close_results.connect_clicked(move |_| {
            if let Some(p) = weak.upgrade() {
                collapse_results(&p);
            }
        });
        let weak = Rc::downgrade(&p);
        p.win.connect_close_request(move |_| {
            if let Some(p) = weak.upgrade() {
                collapse_results(&p);
                if let Some(t) = target() {
                    clear_tag(&t, FIND_TAG);
                }
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(&p);
        p.results_list.connect_row_selected(move |_, row| {
            let (Some(p), Some(row)) = (weak.upgrade(), row) else { return };
            let (doc, found) = p.listed.borrow().clone();
            let (Some(w), Some(f)) = (doc.upgrade(), found.get(row.index() as usize).cloned()) else { return };
            if f.end as i32 > w.buffer.char_count() {
                return;
            }
            select_found(&w, &f);
        });
    }
    p
}

fn collapse_results(p: &Panel) {
    p.results.set_reveal_child(false);
    if let Some(w) = p.listed.borrow().0.upgrade() {
        clear_tag(&w, FIND_ALL_TAG);
    }
    while let Some(row) = p.results_list.row_at_index(0) {
        p.results_list.remove(&row);
    }
    p.win.set_default_size(p.win.width(), -1);
}

// ================================================================ actions on a document window

pub fn install(w: &Rc<DocWindow>) {
    w.add_action("find", |_| show());
    w.add_action("find-next", |w| find_step(w, true, false));
    w.add_action("find-previous", |w| find_step(w, false, false));
    w.add_action("find-all", |w| find_all(w, true));
    w.add_action("highlight-all", |w| find_all(w, false));
    w.add_action("use-for-find", |w| {
        let b = &w.buffer;
        if let Some((s, e)) = b.selection_bounds() {
            set_field_text(&panel().find, &b.text(&s, &e, false));
            panel().regex_check.set_active(false);
        }
    });
    w.add_action("use-for-replace", |w| {
        let b = &w.buffer;
        if let Some((s, e)) = b.selection_bounds() {
            set_field_text(&panel().replace, &b.text(&s, &e, false));
        }
    });
    w.add_action("jump-to-selection", |w| {
        w.view().scroll_to_mark(&w.buffer.get_insert(), 0.0, true, 0.0, 0.5);
    });
    w.add_action("replace", |w| {
        if !replace_current(w) {
            w.win.error_bell();
        }
    });
    w.add_action("replace-and-find", |w| {
        replace_current(w);
        find_step(w, true, false);
    });
    w.add_action("replace-all", replace_all);
    w.add_action("remove-highlights", |w| {
        clear_tag(w, FIND_ALL_TAG);
        clear_tag(w, FIND_TAG);
    });
}

/// Find…: shows the panel and selects the find field's text.
pub fn show() {
    let p = panel();
    if let Some(t) = target() {
        set_target(&t);
    }
    p.win.present();
    p.find.grab_focus();
    let b = p.find.buffer();
    b.select_range(&b.start_iter(), &b.end_iter());
}

fn tag(w: &DocWindow, name: &str) -> gtk::TextTag {
    let table = w.buffer.tag_table();
    table.lookup(name).unwrap_or_else(|| {
        let t = gtk::TextTag::new(Some(name));
        let rgba = adw::StyleManager::default().accent_color_rgba();
        let (alpha, color) = if name == FIND_ALL_TAG { (0.4, rgba) } else { (0.25, gdk::RGBA::new(0.5, 0.5, 0.5, 1.0)) };
        t.set_background_rgba(Some(&gdk::RGBA::new(color.red(), color.green(), color.blue(), alpha)));
        table.add(&t);
        t
    })
}

fn clear_tag(w: &DocWindow, name: &str) {
    if w.buffer.tag_table().lookup(name).is_some() {
        w.buffer.remove_tag(&tag(w, name), &w.buffer.start_iter(), &w.buffer.end_iter());
    }
}

fn select_found(w: &DocWindow, f: &Found) {
    let b = &w.buffer;
    b.select_range(&b.iter_at_offset(f.start as i32), &b.iter_at_offset(f.end as i32));
    w.view().scroll_to_mark(&b.get_insert(), 0.1, false, 0.0, 0.0);
}

/// An alert attached to the panel (shown first).
fn panel_alert(heading: &str, body: &str) {
    let p = panel();
    p.win.present();
    let dlg = adw::AlertDialog::new(Some(heading), Some(body));
    dlg.add_response("ok", "OK");
    dlg.present(Some(&p.win));
}

/// The matcher for the panel's find string, or `None` after beeping / alerting.
fn matcher(w: &DocWindow, quiet: bool) -> Option<Matcher> {
    let p = panel();
    let find = field_text(&p.find);
    match Matcher::new(&find, &settings::get(), w.doc.borrow().ending) {
        Ok(Some(m)) => Some(m),
        Ok(None) => {
            if !quiet {
                w.win.error_bell();
            }
            None
        }
        Err(reason) => {
            if !quiet {
                panel_alert("Invalid regular expression", &reason);
            }
            None
        }
    }
}

/// The ranges searched: the selection with "In Selection", else the whole text. `None` (after the alert) when
/// "In Selection" is on with nothing selected.
fn scope(w: &DocWindow, quiet: bool) -> Option<(usize, usize)> {
    if !settings::get().find_in_selection {
        return Some((0, w.buffer.char_count() as usize));
    }
    match w.buffer.selection_bounds() {
        Some((s, e)) => Some((s.offset() as usize, e.offset() as usize)),
        None => {
            if !quiet {
                panel_alert(
                    "The option “in selection” is selected, although nothing is selected.",
                    "Select the search scope in the document or deselect the “in selection” option.",
                );
            }
            None
        }
    }
}

fn matches_in(w: &DocWindow, m: &Matcher, (a, z): (usize, usize)) -> Vec<Found> {
    m.find_all(&w.text()).into_iter().filter(|f| f.start >= a && f.end <= z).collect()
}

fn count_label(n: usize) -> String {
    match n {
        0 => "Not found".into(),
        1 => "1 match".into(),
        n => format!("{} matches", crate::window::group(n)),
    }
}

/// Find Next / Previous. `incremental`: from the selection start, no beep, no history.
fn find_step(w: &Rc<DocWindow>, forward: bool, incremental: bool) {
    let p = panel();
    let Some(m) = matcher(w, incremental) else { return };
    let Some(range) = scope(w, incremental) else { return };
    let s = settings::get();
    let all = matches_in(w, &m, range);
    clear_tag(w, FIND_TAG);
    if !incremental {
        add_history("find", &field_text(&p.find));
    }
    if all.is_empty() {
        p.find_result.set_label("Not found");
        if !incremental {
            w.win.error_bell();
        }
        return;
    }
    let b = &w.buffer;
    let (ss, se) = b.selection_bounds().map(|(a, b)| (a.offset() as usize, b.offset() as usize)).unwrap_or_else(|| {
        let c = b.iter_at_mark(&b.get_insert()).offset() as usize;
        (c, c)
    });
    let index = if s.find_in_selection {
        // inside the selection: the first (next) or last (previous) match
        Some(if forward { 0 } else { all.len() - 1 })
    } else if forward {
        let from = if incremental { ss } else { se };
        all.iter().position(|f| f.start >= from && !(f.start == ss && f.end == se && !incremental)).or(s.find_wrap.then_some(0))
    } else {
        all.iter().rposition(|f| f.end <= ss && !(f.start == ss && f.end == se)).or(s.find_wrap.then(|| all.len() - 1))
    };
    match index {
        Some(i) => {
            select_found(w, &all[i]);
            p.find_result.set_label(&format!("{}/{}", i + 1, all.len()));
            if incremental {
                let t = tag(w, FIND_TAG);
                for f in &all {
                    b.apply_tag(&t, &b.iter_at_offset(f.start as i32), &b.iter_at_offset(f.end as i32));
                }
            }
        }
        None => {
            p.find_result.set_label(&count_label(all.len()));
            if !incremental {
                w.win.error_bell();
            }
        }
    }
    p.target.replace(Rc::downgrade(w));
}

fn schedule_incremental(p: &Rc<Panel>) {
    if let Some(id) = p.incremental_id.borrow_mut().take() {
        id.remove();
    }
    let s = settings::get();
    if !s.find_incremental || s.find_in_selection || field_text(&p.find).is_empty() {
        return;
    }
    let weak = Rc::downgrade(p);
    let id = glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
        let Some(p) = weak.upgrade() else { return };
        p.incremental_id.borrow_mut().take();
        if let Some(w) = target() {
            find_step(&w, true, true);
        }
    });
    p.incremental_id.replace(Some(id));
}

/// Find All (`list`) or Highlight All: every match highlighted; Find All also lists them in the panel.
fn find_all(w: &Rc<DocWindow>, list: bool) {
    let p = panel();
    let Some(m) = matcher(w, false) else { return };
    let Some(range) = scope(w, false) else { return };
    let all = matches_in(w, &m, range);
    add_history("find", &field_text(&p.find));
    // previous results go, also from another document
    if let Some(prev) = p.listed.borrow().0.upgrade() {
        clear_tag(&prev, FIND_ALL_TAG);
    }
    clear_tag(w, FIND_ALL_TAG);
    clear_tag(w, FIND_TAG);
    p.find_result.set_label(&count_label(all.len()));
    if all.is_empty() {
        w.win.error_bell();
        return;
    }
    let b = &w.buffer;
    let t = tag(w, FIND_ALL_TAG);
    for f in &all {
        b.apply_tag(&t, &b.iter_at_offset(f.start as i32), &b.iter_at_offset(f.end as i32));
    }
    *p.listed.borrow_mut() = (Rc::downgrade(w), all.clone());
    if !list {
        return;
    }
    while let Some(row) = p.results_list.row_at_index(0) {
        p.results_list.remove(&row);
    }
    let name = w.win.title().map(|t| t.to_string()).unwrap_or_default();
    p.results_title.set_label(&match all.len() {
        1 => format!("Found one match in “{name}.”"),
        n => format!("Found {} matches in “{name}.”", crate::window::group(n)),
    });
    p.results_find.set_label(&format!("Find text: {}", field_text(&p.find)));
    let accent = adw::StyleManager::default().accent_color_rgba();
    let bg = format!("#{:02x}{:02x}{:02x}66", (accent.red() * 255.0) as u8, (accent.green() * 255.0) as u8, (accent.blue() * 255.0) as u8);
    for f in all.iter().take(5_000) {
        let start = b.iter_at_offset(f.start as i32);
        let mut line_start = start;
        line_start.set_line_offset(0);
        let mut line_end = line_start;
        if !line_end.ends_line() {
            line_end.forward_to_line_end();
        }
        let col = (f.start as i32 - line_start.offset()) as usize;
        let line: Vec<char> = b.text(&line_start, &line_end, false).chars().collect();
        let len = (f.end - f.start).min(line.len().saturating_sub(col));
        // keep about 16 characters before the match
        let from = col.saturating_sub(16);
        let esc = |cs: &[char]| glib::markup_escape_text(&cs.iter().collect::<String>()).to_string();
        let head = if from > 0 { "…" } else { "" };
        let markup = format!(
            "{head}{}<span background='{bg}'>{}</span>{}",
            esc(&line[from..col]),
            esc(&line[col..col + len]),
            esc(&line[(col + len).min(line.len())..line.len().min(col + len + 1024)])
        );
        let num = gtk::Label::builder().label(crate::window::group(start.line() as usize + 1)).xalign(1.0).width_chars(4).build();
        num.add_css_class("numeric");
        num.add_css_class("caption");
        let text = gtk::Label::builder().xalign(0.0).ellipsize(pango::EllipsizeMode::End).build();
        text.set_markup(&markup);
        text.add_css_class("caption");
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.append(&num);
        row.append(&text);
        p.results_list.append(&row);
    }
    p.results.set_reveal_child(true);
    p.win.present();
}

/// Replace: the selection, if it is exactly a match. Returns whether it replaced.
fn replace_current(w: &Rc<DocWindow>) -> bool {
    if !w.view().is_editable() {
        return false;
    }
    let p = panel();
    let Some(m) = matcher(w, false) else { return false };
    let b = &w.buffer;
    let Some((s, e)) = b.selection_bounds() else { return false };
    let text = w.text();
    let Some(found) = m.find_all(&text).into_iter().find(|f| f.start == s.offset() as usize && f.end == e.offset() as usize) else {
        return false;
    };
    let template = field_text(&p.replace);
    let chars: Vec<char> = text.chars().collect();
    let new = crate::document::convert_line_endings(&m.replacement(&template, &chars, &found, settings::get().find_unescape), w.doc.borrow().ending);
    add_history("replace", &template);
    let (mut si, mut ei) = (s, e);
    b.begin_user_action();
    b.delete(&mut si, &mut ei);
    b.insert(&mut si, &new);
    b.end_user_action();
    true
}

/// Replace All: one undo step; with "In Selection", the selection then covers the replaced range.
fn replace_all(w: &Rc<DocWindow>) {
    if !w.view().is_editable() {
        w.win.error_bell();
        return;
    }
    let p = panel();
    let Some(m) = matcher(w, false) else { return };
    let Some(range) = scope(w, false) else { return };
    let text = w.text();
    let chars: Vec<char> = text.chars().collect();
    let all = matches_in(w, &m, range);
    let template = field_text(&p.replace);
    add_history("replace", &template);
    if all.is_empty() {
        p.replace_result.set_label("Not replaced");
        w.win.error_bell();
        return;
    }
    let unescape = settings::get().find_unescape;
    let ending = w.doc.borrow().ending;
    let b = &w.buffer;
    let mut delta: i64 = 0;
    b.begin_user_action();
    for f in all.iter().rev() {
        let new = crate::document::convert_line_endings(&m.replacement(&template, &chars, f, unescape), ending);
        let (mut s, mut e) = (b.iter_at_offset(f.start as i32), b.iter_at_offset(f.end as i32));
        b.delete(&mut s, &mut e);
        b.insert(&mut s, &new);
        delta += new.chars().count() as i64 - (f.end - f.start) as i64;
    }
    b.end_user_action();
    if settings::get().find_in_selection {
        b.select_range(&b.iter_at_offset(range.0 as i32), &b.iter_at_offset((range.1 as i64 + delta) as i32));
    }
    p.replace_result.set_label(&if all.len() == 1 { "1 replaced".to_string() } else { format!("{} replaced", crate::window::group(all.len())) });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(f: impl FnOnce(&mut Settings)) -> Settings {
        let mut s = Settings::default();
        f(&mut s);
        s
    }

    #[test]
    fn textual_search() {
        let m = Matcher::new("a.b", &Settings::default(), LineEnding::Lf).unwrap().unwrap();
        assert_eq!(m.find_all("xa.b a-b").len(), 1);
        let m = Matcher::new("Cafe", &opts(|s| {
            s.find_ignore_case = true;
            s.find_ignore_diacritics = true;
        }), LineEnding::Lf).unwrap().unwrap();
        let f = m.find_all("un café, CAFÉ");
        assert_eq!(f.iter().map(|f| (f.start, f.end)).collect::<Vec<_>>(), vec![(3, 7), (9, 13)]);
        let m = Matcher::new("ab", &opts(|s| s.find_ignore_width = true), LineEnding::Lf).unwrap().unwrap();
        assert_eq!(m.find_all("ａｂ ab").len(), 2);
        let m = Matcher::new("cat", &opts(|s| s.find_whole_words = true), LineEnding::Lf).unwrap().unwrap();
        assert_eq!(m.find_all("cat concat cat.").len(), 2);
    }

    #[test]
    fn regex_search_and_replace() {
        let s = opts(|s| s.find_regex = true);
        let m = Matcher::new(r#"stroke-width="(\d+)""#, &s, LineEnding::Lf).unwrap().unwrap();
        let text = r#"a stroke-width="2" b stroke-width="10""#;
        let found = m.find_all(text);
        assert_eq!(found.len(), 2);
        let chars: Vec<char> = text.chars().collect();
        assert_eq!(m.replacement("$1px", &chars, &found[1], true), "10px");
        assert_eq!(m.replacement("\\$1\\t", &chars, &found[0], true), "$1\t");
        assert!(Matcher::new("(", &s, LineEnding::Lf).is_err());
        let m = Matcher::new(r"^b\R", &s, LineEnding::Lf).unwrap().unwrap();
        assert_eq!(m.find_all("a\nb\nc").len(), 1);
        let m = Matcher::new(r"(?<=a)b", &s, LineEnding::Lf).unwrap().unwrap();
        assert_eq!(m.find_all("ab cb").len(), 1);
    }
}
