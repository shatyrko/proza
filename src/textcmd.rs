// Text commands (Edit ▸ Select, Text and Format menus): comments, indentation, surround, lines, transformations,
// Unicode normalization, Go to Line and the small sheets they open. GtkSourceView has one selection, so CotEditor's
// "every selection" rules apply to that one.

use crate::settings;
use crate::window::DocWindow;
use adw::prelude::*;
use gtk::{gio, glib};
use sourceview5::prelude::*;
use std::rc::Rc;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

pub fn install(w: &Rc<DocWindow>) {
    // Edit ▸ Select, Insert, Substitutions
    w.add_action("select-line", select_line);
    w.add_action("select-word", select_word);
    w.add_action("select-enclosing", select_enclosing);
    w.add_action("insert-unicode", insert_unicode);
    w.add_action("straighten-quotes", |w| map_selection(w, false, straighten_quotes));
    w.add_action("replace-quotes", |w| map_selection(w, false, smart_quotes));
    w.add_action("replace-dashes", |w| map_selection(w, false, |s| s.replace("--", "—")));
    // Format ▸ Tab Width ▸ Custom…
    w.add_action("custom-tab-width", custom_tab_width);
    // Text ▸ Comment
    w.add_action("toggle-comment", toggle_comment);
    w.add_action("inline-comment", inline_comment);
    w.add_action("block-comment", block_comment);
    w.add_action("uncomment", uncomment);
    // Text ▸ Indentation
    w.add_action("shift-right", |w| shift(w, true));
    w.add_action("shift-left", |w| shift(w, false));
    w.add_action("indent-to-spaces", |w| convert_indentation(w, true));
    w.add_action("indent-to-tabs", |w| convert_indentation(w, false));
    // Text ▸ Surround Selection With
    add_with_arg(w, "surround", |w, pair| {
        let (begin, end) = pair.split_once('|').unwrap_or((pair, pair));
        surround(w, begin, end);
    });
    w.add_action("surround-custom", custom_surround);
    w.add_action("trim-trailing", |w| {
        if editable(w) {
            trim_trailing_whitespace(w, false);
        }
    });
    // Text ▸ Lines
    w.add_action("move-line-up", |w| move_lines(w, true));
    w.add_action("move-line-down", |w| move_lines(w, false));
    w.add_action("sort-lines", |w| map_lines(w, sort_lines));
    w.add_action("reverse-lines", |w| map_lines(w, |lines| lines.into_iter().rev().collect()));
    w.add_action("shuffle-lines", |w| map_lines(w, shuffle_lines));
    w.add_action("delete-duplicates", |w| map_lines(w, delete_duplicates));
    w.add_action("duplicate-line", duplicate_line);
    w.add_action("delete-line", delete_line);
    w.add_action("join-lines", join_lines);
    // Text ▸ Transformations, Unicode Normalizations
    add_with_arg(w, "transform", |w, kind| map_selection(w, true, |s| transform(kind, s)));
    add_with_arg(w, "normalize", |w, form| map_selection(w, true, |s| normalize(form, s)));
    // Find ▸ Go to Line…
    w.add_action("go-to-line", go_to_line);
}

/// An action with a string parameter, for the menu's detailed names like `win.transform::upper`.
fn add_with_arg(w: &Rc<DocWindow>, name: &str, f: impl Fn(&Rc<DocWindow>, &str) + 'static) {
    let a = gio::SimpleAction::new(name, Some(glib::VariantTy::STRING));
    let weak = Rc::downgrade(w);
    a.connect_activate(move |_, v| {
        if let (Some(w), Some(arg)) = (weak.upgrade(), v.and_then(|v| v.get::<String>())) {
            f(&w, &arg)
        }
    });
    w.actions.add_action(&a);
}

// ---------------------------------------------------------------- helpers

fn editable(w: &DocWindow) -> bool {
    let ok = w.view().is_editable();
    if !ok {
        w.win.error_bell();
    }
    ok
}

fn selection(b: &sourceview5::Buffer) -> (gtk::TextIter, gtk::TextIter) {
    b.selection_bounds().unwrap_or_else(|| {
        let i = b.iter_at_mark(&b.get_insert());
        (i, i)
    })
}

/// First and last line touched by the selection; a selection ending at a line start doesn't touch that line.
fn touched_lines(b: &sourceview5::Buffer) -> (i32, i32) {
    let (s, e) = selection(b);
    let last = if e.starts_line() && e.line() > s.line() { e.line() - 1 } else { e.line() };
    (s.line(), last)
}

fn line_start(b: &sourceview5::Buffer, line: i32) -> gtk::TextIter {
    b.iter_at_line(line).unwrap_or_else(|| b.end_iter())
}

fn line_end(b: &sourceview5::Buffer, line: i32) -> gtk::TextIter {
    let mut it = line_start(b, line);
    if !it.ends_line() {
        it.forward_to_line_end();
    }
    it
}

fn line_text(b: &sourceview5::Buffer, line: i32) -> String {
    b.text(&line_start(b, line), &line_end(b, line), false).to_string()
}

fn at_column(b: &sourceview5::Buffer, line: i32, column: i32) -> gtk::TextIter {
    b.iter_at_line_offset(line, column).unwrap_or_else(|| line_start(b, line))
}

fn delete_at(b: &sourceview5::Buffer, line: i32, column: i32, len: i32) {
    let mut s = at_column(b, line, column);
    let mut e = s;
    e.forward_chars(len);
    b.delete(&mut s, &mut e);
}

/// The document's line ending as text.
fn eol(w: &DocWindow) -> &'static str {
    w.doc.borrow().ending.as_str()
}

/// Runs `f` as one undo step; the selection follows the edits (text inserted at its start is included, a caret
/// moves along with text inserted at it).
fn edit_keeping_selection(b: &sourceview5::Buffer, f: impl FnOnce()) {
    let (s, e) = selection(b);
    let ms = b.create_mark(None, &s, s != e);
    let me = b.create_mark(None, &e, false);
    b.begin_user_action();
    f();
    b.end_user_action();
    b.select_range(&b.iter_at_mark(&ms), &b.iter_at_mark(&me));
    b.delete_mark(&ms);
    b.delete_mark(&me);
}

/// Replaces [start, end) with `text` as one undo step and selects the new text.
fn replace_and_select(b: &sourceview5::Buffer, start: i32, end: i32, text: &str) {
    let (mut s, mut e) = (b.iter_at_offset(start), b.iter_at_offset(end));
    b.begin_user_action();
    b.delete(&mut s, &mut e);
    b.insert(&mut s, text);
    b.end_user_action();
    b.select_range(&b.iter_at_offset(start), &b.iter_at_offset(start + text.chars().count() as i32));
}

/// Replaces the selection (the word at the caret when `word_if_empty` and nothing is selected) with `f(text)`.
fn map_selection(w: &DocWindow, word_if_empty: bool, f: impl Fn(&str) -> String) {
    if !editable(w) {
        return;
    }
    let b = &w.buffer;
    let (mut s, mut e) = selection(b);
    if s == e {
        if !word_if_empty {
            return;
        }
        let Some(word) = word_at(&s) else { return };
        (s, e) = word;
    }
    let old = b.text(&s, &e, false).to_string();
    let new = f(&old);
    if new == old {
        b.select_range(&s, &e);
    } else {
        replace_and_select(b, s.offset(), e.offset(), &new);
    }
}

/// The word around (or just before) `at`.
fn word_at(at: &gtk::TextIter) -> Option<(gtk::TextIter, gtk::TextIter)> {
    let (mut s, mut e) = (*at, *at);
    if !s.inside_word() && !s.ends_word() {
        return None;
    }
    if !s.starts_word() {
        s.backward_word_start();
    }
    if !e.ends_word() {
        e.forward_word_end();
    }
    (s != e).then_some((s, e))
}

/// Replaces the touched lines (all lines when nothing is selected) with `f(lines)`; lines come without endings.
fn map_lines(w: &Rc<DocWindow>, f: impl FnOnce(Vec<String>) -> Vec<String>) {
    if !editable(w) {
        return;
    }
    let b = &w.buffer;
    let (first, last) = if b.has_selection() { touched_lines(b) } else { (0, b.line_count() - 1) };
    let lines: Vec<String> = (first..=last).map(|l| line_text(b, l)).collect();
    let new = f(lines.clone());
    if new != lines {
        replace_and_select(b, line_start(b, first).offset(), line_end(b, last).offset(), &new.join(eol(w)));
    }
}

/// A sheet on the window with `child` and Cancel / `ok_label`. `on_ok` returns false to beep and keep it open.
fn sheet(w: &Rc<DocWindow>, heading: Option<&str>, child: &impl IsA<gtk::Widget>, ok_label: &str, on_ok: impl Fn() -> bool + 'static) {
    let dlg = adw::AlertDialog::new(heading, None);
    dlg.set_extra_child(Some(child));
    dlg.add_responses(&[("cancel", "Cancel"), ("ok", ok_label)]);
    dlg.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
    dlg.set_default_response(Some("ok"));
    dlg.set_close_response("cancel");
    let weak = Rc::downgrade(w);
    dlg.connect_response(Some("ok"), move |dlg, _| {
        if on_ok() {
            return;
        }
        let Some(w) = weak.upgrade() else { return };
        w.win.error_bell();
        // CotEditor keeps the sheet open on invalid input
        let dlg = dlg.clone();
        glib::idle_add_local_once(move || dlg.present(Some(&w.win)));
    });
    dlg.present(Some(&w.win));
}

// ---------------------------------------------------------------- Edit ▸ Select

fn select_line(w: &Rc<DocWindow>) {
    let b = &w.buffer;
    let (first, last) = touched_lines(b);
    let mut e = line_start(b, last);
    if !e.forward_line() {
        e = b.end_iter();
    }
    b.select_range(&line_start(b, first), &e);
}

/// No selection: the word at the caret. With a selection: the next occurrence of the selected text (wrapping).
fn select_word(w: &Rc<DocWindow>) {
    let b = &w.buffer;
    let (s, e) = selection(b);
    if s == e {
        if let Some((ws, we)) = word_at(&s) {
            b.select_range(&ws, &we);
        }
        return;
    }
    let needle = b.text(&s, &e, false).to_string();
    let flags = gtk::TextSearchFlags::TEXT_ONLY;
    let found = e.forward_search(&needle, flags, None).or_else(|| b.start_iter().forward_search(&needle, flags, Some(&s)));
    match found {
        Some((fs, fe)) if fs != s => {
            b.select_range(&fs, &fe);
            w.view().scroll_mark_onscreen(&b.get_insert());
        }
        _ => w.win.error_bell(),
    }
}

/// The nearest () {} [] <> pair around the selection, symbols included.
fn select_enclosing(w: &Rc<DocWindow>) {
    let b = &w.buffer;
    let (s, e) = selection(b);
    let text: Vec<char> = w.text().chars().collect();
    match enclosing_pair(&text, s.offset() as usize, e.offset() as usize) {
        Some((a, z)) => b.select_range(&b.iter_at_offset(a as i32), &b.iter_at_offset(z as i32)),
        None => w.win.error_bell(),
    }
}

pub fn enclosing_pair(text: &[char], start: usize, end: usize) -> Option<(usize, usize)> {
    const PAIRS: [(char, char); 4] = [('(', ')'), ('{', '}'), ('[', ']'), ('<', '>')];
    for i in (0..start).rev() {
        let Some(&(open, close)) = PAIRS.iter().find(|(o, _)| *o == text[i]) else { continue };
        let mut depth = 0;
        let close_at = (i + 1..text.len()).find(|&j| {
            if text[j] == open {
                depth += 1;
            } else if text[j] == close {
                if depth == 0 {
                    return true;
                }
                depth -= 1;
            }
            false
        });
        if let Some(j) = close_at {
            if j + 1 >= end && (i, j + 1) != (start, end) {
                return Some((i, j + 1));
            }
        }
    }
    None
}

// ---------------------------------------------------------------- Edit ▸ Insert by Unicode Code Point…

/// "U+1F600", "0x1F600", "ὠ0" or bare hex (1–6 digits) → the character.
pub fn parse_code_point(s: &str) -> Option<char> {
    let t = s.trim();
    let hex = ["U+", "u+", "0x", "0X", "\\u", "\\U"].iter().find_map(|p| t.strip_prefix(p)).unwrap_or(t);
    if hex.is_empty() || hex.len() > 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    char::from_u32(u32::from_str_radix(hex, 16).ok()?)
}

/// A popover under the caret: preview, name and a code point field; Return inserts and clears the field.
fn insert_unicode(w: &Rc<DocWindow>) {
    if !editable(w) {
        return;
    }
    let view = w.view();
    let b = &w.buffer;
    let rect = view.iter_location(&b.iter_at_mark(&b.get_insert()));
    let (x, y) = view.buffer_to_window_coords(gtk::TextWindowType::Widget, rect.x(), rect.y() + rect.height());
    let pop = gtk::Popover::builder().position(gtk::PositionType::Bottom).build();
    pop.set_parent(&view);
    pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x, y, 1, 1)));

    let glyph = gtk::Label::new(Some("⬚"));
    glyph.add_css_class("title-1");
    let name = gtk::Label::builder().label("Invalid code").wrap(true).lines(2).max_width_chars(16).xalign(0.0).build();
    name.add_css_class("caption");
    name.add_css_class("dim-label");
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.append(&glyph);
    row.append(&name);
    let entry = gtk::Entry::builder().placeholder_text("U+1F600").build();
    let bx = gtk::Box::new(gtk::Orientation::Vertical, 8);
    bx.set_width_request(160);
    bx.append(&row);
    bx.append(&entry);
    pop.set_child(Some(&bx));

    entry.connect_changed(move |e| match parse_code_point(&e.text()) {
        Some(c) => {
            glyph.set_label(&c.to_string());
            name.set_label(&unicode_names2::name(c).map(|n| n.to_string()).unwrap_or_else(|| format!("U+{:04X}", c as u32)));
        }
        None => {
            glyph.set_label("⬚");
            name.set_label("Invalid code");
        }
    });
    let weak = Rc::downgrade(w);
    entry.connect_activate(move |e| {
        let Some(w) = weak.upgrade() else { return };
        match parse_code_point(&e.text()) {
            Some(c) => {
                let b = &w.buffer;
                b.begin_user_action();
                b.delete_selection(true, true);
                b.insert_at_cursor(&c.to_string());
                b.end_user_action();
                e.set_text("");
            }
            None => w.win.error_bell(),
        }
    });
    pop.connect_closed(|p| {
        let p = p.clone();
        glib::idle_add_local_once(move || p.unparent());
    });
    pop.popup();
    entry.grab_focus();
}

// ---------------------------------------------------------------- Edit ▸ Substitutions

pub fn straighten_quotes(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '‘' | '’' | '‚' | '‛' => '\'',
            '“' | '”' | '„' | '‟' => '"',
            c => c,
        })
        .collect()
}

/// Straight quotes → curly: opening after the start, whitespace or an opening bracket; closing otherwise.
pub fn smart_quotes(s: &str) -> String {
    let mut prev: Option<char> = None;
    s.chars()
        .map(|c| {
            let opening = prev.is_none_or(|p| p.is_whitespace() || "([{<“‘".contains(p));
            prev = Some(c);
            match (c, opening) {
                ('"', true) => '“',
                ('"', false) => '”',
                ('\'', true) => '‘',
                ('\'', false) => '’',
                (c, _) => c,
            }
        })
        .collect()
}

// ---------------------------------------------------------------- Format ▸ Tab Width ▸ Custom…

fn custom_tab_width(w: &Rc<DocWindow>) {
    let spin = gtk::SpinButton::with_range(1.0, 99.0, 1.0);
    spin.set_value(w.vs.borrow().tab_width as f64);
    spin.set_activates_default(true);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.set_halign(gtk::Align::Center);
    row.append(&gtk::Label::new(Some("Tab width:")));
    row.append(&spin);
    let weak = Rc::downgrade(w);
    let sp = spin.clone();
    sheet(w, None, &row, "OK", move || {
        sp.update();
        if let Some(w) = weak.upgrade() {
            w.actions.activate_action("tab-width", Some(&sp.value_as_int().clamp(1, 99).to_string().to_variant()));
        }
        true
    });
    spin.grab_focus();
}

// ---------------------------------------------------------------- Text ▸ Comment

struct Delimiters {
    inline: Option<String>,
    block: Option<(String, String)>,
}

fn delimiters(w: &DocWindow) -> Delimiters {
    let lang = w.buffer.language();
    let meta = |k: &str| lang.as_ref().and_then(|l| l.metadata(k)).map(|s| s.to_string()).filter(|s| !s.is_empty());
    Delimiters { inline: meta("line-comment-start"), block: meta("block-comment-start").zip(meta("block-comment-end")) }
}

/// Settings → Edit "Append a space to comment delimiter".
fn space() -> &'static str {
    if settings::get().space_after_comment {
        " "
    } else {
        ""
    }
}

fn indent_len(line: &str) -> i32 {
    line.chars().take_while(|c| *c == ' ' || *c == '\t').count() as i32
}

fn block_commented(body: &str, (s, e): &(String, String)) -> bool {
    let t = body.trim_end();
    t.starts_with(s.as_str()) && t.ends_with(e.as_str()) && t.len() >= s.len() + e.len()
}

/// Whether every non-blank touched line starts (after its indentation) with a comment delimiter.
fn lines_commented(w: &DocWindow, d: &Delimiters) -> bool {
    let b = &w.buffer;
    let (first, last) = touched_lines(b);
    let mut any = false;
    for l in first..=last {
        let t = line_text(b, l);
        let body = t.trim_start();
        if body.is_empty() {
            continue;
        }
        any = true;
        let inline = d.inline.as_ref().is_some_and(|c| body.starts_with(c.as_str()));
        if !inline && !d.block.as_ref().is_some_and(|p| block_commented(body, p)) {
            return false;
        }
    }
    any
}

/// Toggle Comment: uncomments when every selected line is commented, else comments them out.
fn toggle_comment(w: &Rc<DocWindow>) {
    let d = delimiters(w);
    if d.inline.is_none() && d.block.is_none() {
        w.win.error_bell();
        return;
    }
    if !editable(w) {
        return;
    }
    if lines_commented(w, &d) {
        uncomment_lines(w, &d);
    } else {
        comment_lines(w, &d);
    }
}

/// Comments out every non-blank touched line: the inline delimiter (else a block pair around the line) at the
/// common indentation (Settings → Edit "Insert comment delimiters after indent") or at the line start.
fn comment_lines(w: &DocWindow, d: &Delimiters) {
    let b = &w.buffer;
    let (first, last) = touched_lines(b);
    let lines: Vec<String> = (first..=last).map(|l| line_text(b, l)).collect();
    let column = if settings::get().comment_after_indent {
        lines.iter().filter(|l| !l.trim().is_empty()).map(|l| indent_len(l)).min().unwrap_or(0)
    } else {
        0
    };
    edit_keeping_selection(b, || {
        for (i, t) in lines.iter().enumerate() {
            if t.trim().is_empty() {
                continue;
            }
            let l = first + i as i32;
            match (&d.inline, &d.block) {
                (Some(c), _) => b.insert(&mut at_column(b, l, column), &format!("{c}{}", space())),
                (None, Some((s, e))) => {
                    b.insert(&mut line_end(b, l), &format!("{}{e}", space()));
                    b.insert(&mut at_column(b, l, column), &format!("{s}{}", space()));
                }
                _ => {}
            }
        }
    });
}

/// Removes the inline delimiter or a block pair (each with one adjoining space) from every touched line.
fn uncomment_lines(w: &DocWindow, d: &Delimiters) -> bool {
    let b = &w.buffer;
    let (first, last) = touched_lines(b);
    let mut changed = false;
    edit_keeping_selection(b, || {
        for l in first..=last {
            let t = line_text(b, l);
            let indent = indent_len(&t);
            let body: String = t.chars().skip(indent as usize).collect();
            if let Some((s, e)) = d.block.as_ref().filter(|p| block_commented(&body, p)) {
                let chars: Vec<char> = body.trim_end().chars().collect();
                let (sl, el) = (s.chars().count(), e.chars().count());
                let (mut from, mut len) = (chars.len() - el, el);
                if from > sl && chars[from - 1] == ' ' {
                    from -= 1;
                    len += 1;
                }
                delete_at(b, l, indent + from as i32, len as i32);
                let space = (chars.get(sl) == Some(&' ') && sl < from) as usize;
                delete_at(b, l, indent, (sl + space) as i32);
                changed = true;
            } else if let Some(c) = d.inline.as_ref().filter(|c| body.starts_with(c.as_str())) {
                let space = body[c.len()..].starts_with(' ') as i32;
                delete_at(b, l, indent, c.chars().count() as i32 + space);
                changed = true;
            }
        }
    });
    changed
}

/// Inline Comment: the inline delimiter at the start of each selected line segment.
fn inline_comment(w: &Rc<DocWindow>) {
    let Some(c) = delimiters(w).inline else {
        w.win.error_bell();
        return;
    };
    if !editable(w) {
        return;
    }
    let b = &w.buffer;
    let first_col = selection(b).0.line_offset();
    let (first, last) = touched_lines(b);
    edit_keeping_selection(b, || {
        for l in first..=last {
            b.insert(&mut at_column(b, l, if l == first { first_col } else { 0 }), &format!("{c}{}", space()));
        }
    });
}

/// Block Comment: the selection wrapped in the block delimiters.
fn block_comment(w: &Rc<DocWindow>) {
    let Some((s, e)) = delimiters(w).block else {
        w.win.error_bell();
        return;
    };
    surround(w, &format!("{s}{}", space()), &format!("{}{e}", space()));
}

/// Uncomment: a block pair that exactly fits the selection, else the inline delimiters of the touched lines.
fn uncomment(w: &Rc<DocWindow>) {
    if !editable(w) {
        return;
    }
    let d = delimiters(w);
    let b = &w.buffer;
    let (s, e) = selection(b);
    if let Some((bs, be)) = &d.block {
        let t = b.text(&s, &e, false).to_string();
        if t.starts_with(bs.as_str()) && t.ends_with(be.as_str()) && t.len() >= bs.len() + be.len() {
            let inner = &t[bs.len()..t.len() - be.len()];
            let inner = inner.strip_prefix(' ').unwrap_or(inner);
            let inner = inner.strip_suffix(' ').unwrap_or(inner);
            replace_and_select(b, s.offset(), e.offset(), inner);
            return;
        }
    }
    if !uncomment_lines(w, &d) {
        w.win.error_bell();
    }
}

// ---------------------------------------------------------------- Text ▸ Indentation

/// Shift Right adds one indent unit (a tab, or tab-width spaces with soft tabs) to every touched line;
/// Shift Left removes one leading tab or up to tab-width leading spaces.
fn shift(w: &Rc<DocWindow>, right: bool) {
    if !editable(w) {
        return;
    }
    let b = &w.buffer;
    let (first, last) = touched_lines(b);
    let (soft, width) = (w.vs.borrow().soft_tabs, w.vs.borrow().tab_width as usize);
    let unit = if soft { " ".repeat(width) } else { "\t".into() };
    if !right && !(first..=last).any(|l| line_text(b, l).starts_with([' ', '\t'])) {
        return;
    }
    edit_keeping_selection(b, || {
        for l in first..=last {
            if right {
                b.insert(&mut line_start(b, l), &unit);
            } else {
                let t = line_text(b, l);
                let n = if t.starts_with('\t') { 1 } else { t.chars().take(width).take_while(|c| *c == ' ').count() };
                delete_at(b, l, 0, n as i32);
            }
        }
    });
}

/// Leading tabs ↔ runs of tab-width spaces, in the selected lines or the whole document.
fn convert_indentation(w: &Rc<DocWindow>, to_spaces: bool) {
    let width = w.vs.borrow().tab_width as usize;
    map_lines(w, move |lines| lines.into_iter().map(|l| convert_indent(&l, to_spaces, width)).collect());
}

pub fn convert_indent(line: &str, to_spaces: bool, width: usize) -> String {
    let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
    let rest = &line[indent.len()..];
    if to_spaces {
        return indent.replace('\t', &" ".repeat(width)) + rest;
    }
    let mut out = String::new();
    let mut spaces = 0;
    for c in indent.chars() {
        if c == '\t' {
            out.push('\t');
            spaces = 0;
        } else {
            spaces += 1;
            if spaces == width {
                out.push('\t');
                spaces = 0;
            }
        }
    }
    out + &" ".repeat(spaces) + rest
}

// ---------------------------------------------------------------- Text ▸ Surround Selection With

/// Wraps the selection (or inserts the pair at the caret) and keeps the inner text selected.
fn surround(w: &Rc<DocWindow>, begin: &str, end: &str) {
    if !editable(w) {
        return;
    }
    let b = &w.buffer;
    let (s, e) = selection(b);
    let (so, eo) = (s.offset(), e.offset());
    b.begin_user_action();
    b.insert(&mut b.iter_at_offset(eo), end);
    b.insert(&mut b.iter_at_offset(so), begin);
    b.end_user_action();
    let n = begin.chars().count() as i32;
    b.select_range(&b.iter_at_offset(so + n), &b.iter_at_offset(eo + n));
}

thread_local! {
    /// The last Custom Surround pair, prefilled next time.
    static LAST_SURROUND: std::cell::RefCell<(String, String)> = const { std::cell::RefCell::new((String::new(), String::new())) };
}

fn custom_surround(w: &Rc<DocWindow>) {
    if !editable(w) {
        return;
    }
    let (last_b, last_e) = LAST_SURROUND.with(|l| l.borrow().clone());
    let begin = gtk::Entry::builder().text(&last_b).width_chars(5).activates_default(true).build();
    let end = gtk::Entry::builder().text(&last_e).width_chars(5).placeholder_text(&last_b).activates_default(true).build();
    {
        let end = end.clone();
        begin.connect_changed(move |b| end.set_placeholder_text(Some(&b.text())));
    }
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.set_halign(gtk::Align::Center);
    row.append(&gtk::Label::new(Some("Begin:")));
    row.append(&begin);
    row.append(&gtk::Label::new(Some("End:")));
    row.append(&end);
    let weak = Rc::downgrade(w);
    let (bc, ec) = (begin.clone(), end.clone());
    sheet(w, Some("Surround with:"), &row, "OK", move || {
        let (b, e) = (bc.text().to_string(), ec.text().to_string());
        if b.is_empty() {
            return false;
        }
        LAST_SURROUND.with(|l| *l.borrow_mut() = (b.clone(), e.clone()));
        if let Some(w) = weak.upgrade() {
            surround(&w, &b, if e.is_empty() { &b } else { &e });
        }
        true
    });
    begin.grab_focus();
}

// ---------------------------------------------------------------- Text ▸ Trim Trailing Whitespace

/// Removes trailing spaces and tabs at every line end; `keep_cursor_line` leaves the line being edited alone.
/// Whitespace-only lines are kept (CotEditor's default).
pub fn trim_trailing_whitespace(w: &DocWindow, keep_cursor_line: bool) {
    let b = &w.buffer;
    let cursor = b.iter_at_mark(&b.get_insert()).line();
    let lines: Vec<(i32, String)> = (0..b.line_count())
        .filter(|&l| !(keep_cursor_line && l == cursor))
        .map(|l| (l, line_text(b, l)))
        .filter(|(_, t)| !t.trim().is_empty() && t.ends_with([' ', '\t']))
        .collect();
    if lines.is_empty() {
        return;
    }
    edit_keeping_selection(b, || {
        for (l, t) in lines {
            let keep = t.trim_end_matches([' ', '\t']).chars().count() as i32;
            delete_at(b, l, keep, t.chars().count() as i32 - keep);
        }
    });
}

// ---------------------------------------------------------------- Text ▸ Lines

/// Moves the touched lines one line up or down; the selection moves along. Beeps at the document edge.
fn move_lines(w: &Rc<DocWindow>, up: bool) {
    if !editable(w) {
        return;
    }
    let b = &w.buffer;
    let (first, last) = touched_lines(b);
    // an empty line after a final line ending is not a line to swap with
    let last_line = if b.end_iter().starts_line() && b.line_count() > 1 { b.line_count() - 2 } else { b.line_count() - 1 };
    if (up && first == 0) || (!up && last >= last_line) {
        w.win.error_bell();
        return;
    }
    let (s, e) = selection(b);
    let (so, eo) = (s.offset(), e.offset());
    let (a, z) = if up { (first - 1, last) } else { (first, last + 1) };
    let mut lines: Vec<String> = (a..=z).map(|l| line_text(b, l)).collect();
    let other = if up { lines.remove(0) } else { lines.pop().unwrap_or_default() };
    let moved = (other.chars().count() + eol(w).chars().count()) as i32;
    if up {
        lines.push(other);
    } else {
        lines.insert(0, other);
    }
    let (rs, re) = (line_start(b, a).offset(), line_end(b, z).offset());
    replace_and_select(b, rs, re, &lines.join(eol(w)));
    let shift = if up { -moved } else { moved };
    b.select_range(&b.iter_at_offset(so + shift), &b.iter_at_offset(eo + shift));
    w.view().scroll_mark_onscreen(&b.get_insert());
}

/// Ascending, by the locale's collation.
fn sort_lines(lines: Vec<String>) -> Vec<String> {
    let mut keyed: Vec<glib::GString> = lines.iter().map(|l| glib::GString::from(l.as_str())).collect();
    keyed.sort_by(|a, b| a.as_gstr().collate(b.as_str()));
    keyed.into_iter().map(|g| g.to_string()).collect()
}

fn shuffle_lines(mut lines: Vec<String>) -> Vec<String> {
    for i in (1..lines.len()).rev() {
        lines.swap(i, glib::random_int_range(0, i as i32 + 1) as usize);
    }
    lines
}

/// Later duplicates removed, first occurrences kept.
pub fn delete_duplicates(lines: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    lines.into_iter().filter(|l| seen.insert(l.clone())).collect()
}

/// Copies the touched lines below themselves; the selection moves to the copy.
fn duplicate_line(w: &Rc<DocWindow>) {
    if !editable(w) {
        return;
    }
    let b = &w.buffer;
    let (first, last) = touched_lines(b);
    let (s, e) = selection(b);
    let (so, eo) = (s.offset(), e.offset());
    let copy = format!("{}{}", eol(w), b.text(&line_start(b, first), &line_end(b, last), false));
    b.begin_user_action();
    b.insert(&mut line_end(b, last), &copy);
    b.end_user_action();
    let n = copy.chars().count() as i32;
    b.select_range(&b.iter_at_offset(so + n), &b.iter_at_offset(eo + n));
    w.view().scroll_mark_onscreen(&b.get_insert());
}

/// Deletes the touched lines; the caret goes to where they were.
fn delete_line(w: &Rc<DocWindow>) {
    if !editable(w) {
        return;
    }
    let b = &w.buffer;
    let (first, last) = touched_lines(b);
    let mut s = line_start(b, first);
    let mut e = line_start(b, last);
    if !e.forward_line() {
        // the last line: take the line ending before it instead
        e = b.end_iter();
        if first > 0 {
            s = line_end(b, first - 1);
        }
    }
    b.begin_user_action();
    b.delete(&mut s, &mut e);
    b.end_user_action();
    b.place_cursor(&line_start(b, first.min(b.line_count() - 1)));
}

/// With a selection: every line break inside it, with the whitespace around it, becomes one space.
/// Without: the caret's line is joined with the next one the same way.
fn join_lines(w: &Rc<DocWindow>) {
    if !editable(w) {
        return;
    }
    let b = &w.buffer;
    if b.has_selection() {
        map_selection(w, false, join);
        return;
    }
    let l = b.iter_at_mark(&b.get_insert()).line();
    if l + 1 >= b.line_count() {
        w.win.error_bell();
        return;
    }
    let from = at_column(b, l, line_text(b, l).trim_end().chars().count() as i32).offset();
    let next = line_text(b, l + 1);
    let to = line_start(b, l + 1).offset() + (next.chars().count() - next.trim_start().chars().count()) as i32;
    replace_and_select(b, from, to, " ");
    b.place_cursor(&b.iter_at_offset(from + 1));
}

pub fn join(s: &str) -> String {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"[ \t]*(\r\n|[\n\r\u{85}\u{2028}\u{2029}])\s*").unwrap()).replace_all(s, " ").into_owned()
}

// ---------------------------------------------------------------- Text ▸ Transformations

pub fn transform(kind: &str, s: &str) -> String {
    match kind {
        "upper" => s.to_uppercase(),
        "lower" => s.to_lowercase(),
        "capitalize" => capitalize(s),
        "fullwidth" => to_fullwidth(s, false),
        "halfwidth" => to_halfwidth(s, false),
        "alnum-full" => to_fullwidth(s, true),
        "alnum-half" => to_halfwidth(s, true),
        "url-encode" => url_encode(s),
        "url-decode" => url_decode(s).unwrap_or_else(|| s.to_string()),
        _ => s.to_string(),
    }
}

/// Every word's first letter upper case, the rest lower case.
pub fn capitalize(s: &str) -> String {
    s.split_word_bounds()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) if f.is_alphanumeric() => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
                _ => w.to_string(),
            }
        })
        .collect()
}

const HALF_KANA: &str = "｡｢｣､･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝﾞﾟ";
const FULL_KANA: &str = "。「」、・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

/// ASCII → full width; unless `alnum_only`, also the space and half-width katakana (voicing marks joined).
pub fn to_fullwidth(s: &str, alnum_only: bool) -> String {
    let (half, full): (Vec<char>, Vec<char>) = (HALF_KANA.chars().collect(), FULL_KANA.chars().collect());
    let mut out = String::with_capacity(s.len() * 3);
    for c in s.chars() {
        match c {
            '!'..='~' => out.push(char::from_u32(c as u32 + 0xFEE0).unwrap_or(c)),
            _ if alnum_only => out.push(c),
            ' ' => out.push('\u{3000}'),
            'ﾞ' if !out.is_empty() => out.push('\u{3099}'),
            'ﾟ' if !out.is_empty() => out.push('\u{309A}'),
            c => out.push(half.iter().position(|h| *h == c).map_or(c, |i| full[i])),
        }
    }
    if alnum_only {
        out
    } else {
        out.nfc().collect()
    }
}

pub fn to_halfwidth(s: &str, alnum_only: bool) -> String {
    let (half, full): (Vec<char>, Vec<char>) = (HALF_KANA.chars().collect(), FULL_KANA.chars().collect());
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\u{FF01}'..='\u{FF5E}' => out.push(char::from_u32(c as u32 - 0xFEE0).unwrap_or(c)),
            _ if alnum_only => out.push(c),
            '\u{3000}' => out.push(' '),
            c if ('\u{30A0}'..='\u{30FF}').contains(&c) || "。「」、・゛゜".contains(c) => {
                for d in std::iter::once(c).nfd() {
                    out.push(match d {
                        '\u{3099}' => 'ﾞ',
                        '\u{309A}' => 'ﾟ',
                        d => full.iter().position(|f| *f == d).map_or(d, |i| half[i]),
                    });
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// Percent-encodes all but A–Z a–z 0–9 - . _ ~; valid %XX already there is kept.
pub fn url_encode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() * 3);
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'%' && bytes.get(i + 1..i + 3).is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit)) {
            out.push_str(&s[i..i + 3]);
            i += 3;
            continue;
        }
        if c.is_ascii_alphanumeric() || b"-._~".contains(&c) {
            out.push(c as char);
        } else {
            out.push_str(&format!("%{c:02X}"));
        }
        i += 1;
    }
    out
}

/// `None` when the text isn't valid percent encoding of UTF-8.
pub fn url_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            out.push(u8::from_str_radix(s.get(i + 1..i + 3)?, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

// ---------------------------------------------------------------- Text ▸ Unicode Normalizations

fn default_ignorable(c: char) -> bool {
    matches!(c as u32, 0xAD | 0x34F | 0x61C | 0x115F..=0x1160 | 0x17B4..=0x17B5 | 0x180B..=0x180F | 0x200B..=0x200F
        | 0x202A..=0x202E | 0x2060..=0x206F | 0x3164 | 0xFE00..=0xFE0F | 0xFEFF | 0xFFA0 | 0xFFF0..=0xFFF8
        | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A | 0xE0000..=0xE0FFF)
}

pub fn normalize(form: &str, s: &str) -> String {
    match form {
        "nfd" => s.nfd().collect(),
        "nfc" => s.nfc().collect(),
        "nfkd" => s.nfkd().collect(),
        "nfkc" => s.nfkc().collect(),
        // NFKC, case folding, default-ignorable code points removed
        "nfkc-casefold" => s.nfkc().flat_map(char::to_lowercase).filter(|c| !default_ignorable(*c)).nfkc().collect(),
        _ => s.to_string(),
    }
}

// ---------------------------------------------------------------- Find ▸ Go to Line…

/// CotEditor's "fuzzy range": `N` or `N:M` → the first and last line (1-based, inclusive), `(0, 0)` for the very
/// start, `None` when invalid. A negative N counts from the end; M lines are selected, a negative M ends that many
/// lines from the end.
pub fn parse_line_range(input: &str, line_count: i64) -> Option<(i64, i64)> {
    let count = line_count.max(1);
    let (n, m) = match input.trim().split_once(':') {
        Some((n, m)) => (n.trim().parse::<i64>().ok()?, Some(m.trim().parse::<i64>().ok()?)),
        None => (input.trim().parse::<i64>().ok()?, None),
    };
    let first = if n < 0 { count + n + 1 } else { n };
    if first < 0 {
        return None;
    }
    if first == 0 {
        return Some((0, 0));
    }
    let first = first.min(count);
    let last = match m {
        None | Some(0) | Some(1) => first,
        Some(m) if m > 1 => first + m - 1,
        Some(m) => count + m + 1,
    };
    (last >= first).then_some((first, last.min(count)))
}

/// Selects the line(s) including their line ending and scrolls to them.
fn go_to_line(w: &Rc<DocWindow>) {
    let entry = gtk::Entry::builder().placeholder_text("Line Number").activates_default(true).build();
    entry.add_css_class("numeric");
    let weak = Rc::downgrade(w);
    let ec = entry.clone();
    sheet(w, None, &entry, "Go", move || {
        let Some(w) = weak.upgrade() else { return true };
        let b = &w.buffer;
        let Some((first, last)) = parse_line_range(&ec.text(), crate::window::count_lines(&w.text()) as i64) else {
            return false;
        };
        if first == 0 {
            b.place_cursor(&b.start_iter());
        } else {
            let mut e = line_start(b, (last - 1) as i32);
            if !e.forward_line() {
                e = b.end_iter();
            }
            b.select_range(&line_start(b, (first - 1) as i32), &e);
        }
        w.view().scroll_to_mark(&b.get_insert(), 0.1, false, 0.0, 0.0);
        true
    });
    entry.grab_focus();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transformations() {
        assert_eq!(capitalize("hello wORLD-wide"), "Hello World-Wide");
        assert_eq!(to_fullwidth("Ab1 !", true), "Ａｂ１ ！");
        assert_eq!(to_fullwidth("ｶﾞｷﾞ A", false), "ガギ\u{3000}Ａ");
        assert_eq!(to_halfwidth("ガギ\u{3000}Ａ", false), "ｶﾞｷﾞ A");
        assert_eq!(url_encode("a b/é%20"), "a%20b%2F%C3%A9%20");
        assert_eq!(url_decode("a%20b%2F%C3%A9").as_deref(), Some("a b/é"));
        assert_eq!(url_decode("%zz"), None);
        assert_eq!(normalize("nfd", "é").chars().count(), 2);
        assert_eq!(normalize("nfkc-casefold", "Ｆｏｏ\u{200B}"), "foo");
    }

    #[test]
    fn quotes_and_lines() {
        assert_eq!(straighten_quotes("“a” ‘b’"), "\"a\" 'b'");
        assert_eq!(smart_quotes("say \"hi\" it's"), "say “hi” it’s");
        assert_eq!(join("a  \n   b\r\nc"), "a b c");
        assert_eq!(delete_duplicates(vec!["a".into(), "b".into(), "a".into()]), vec!["a", "b"]);
        assert_eq!(convert_indent("\t\tx", true, 4), "        x");
        assert_eq!(convert_indent("      x", false, 4), "\t  x");
    }

    #[test]
    fn enclosing_and_code_points() {
        let t: Vec<char> = "f(a, [b], c)".chars().collect();
        assert_eq!(enclosing_pair(&t, 6, 7), Some((5, 8)));
        assert_eq!(enclosing_pair(&t, 5, 8), Some((1, 12)));
        assert_eq!(enclosing_pair(&t, 1, 12), None);
        assert_eq!(parse_code_point("U+1F600"), Some('😀'));
        assert_eq!(parse_code_point("\\u00e9"), Some('é'));
        assert_eq!(parse_code_point("xyz"), None);
    }

    #[test]
    fn line_ranges() {
        assert_eq!(parse_line_range("3", 10), Some((3, 3)));
        assert_eq!(parse_line_range("-1", 10), Some((10, 10)));
        assert_eq!(parse_line_range("0", 10), Some((0, 0)));
        assert_eq!(parse_line_range("99", 10), Some((10, 10)));
        assert_eq!(parse_line_range("3:2", 10), Some((3, 4)));
        assert_eq!(parse_line_range("3:-1", 10), Some((3, 10)));
        assert_eq!(parse_line_range("x", 10), None);
    }
}
