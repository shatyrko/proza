// Text commands (Text and Format menus): comments, indentation, lines, transformations, normalization.

use crate::window::DocWindow;
use gtk::prelude::*;
use std::rc::Rc;

pub fn install(_w: &Rc<DocWindow>) {}

/// Removes trailing spaces and tabs at every line end; `keep_cursor_line` leaves the line being edited alone.
pub fn trim_trailing_whitespace(w: &DocWindow, keep_cursor_line: bool) {
    let b = &w.buffer;
    let cursor = b.iter_at_mark(&b.get_insert()).line();
    let text = w.text();
    let mut changed = false;
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split_inclusive('\n').enumerate() {
        let (body, end) = match line.strip_suffix('\n') {
            Some(b) => (b, "\n"),
            None => (line, ""),
        };
        let (body, cr) = match body.strip_suffix('\r') {
            Some(b) => (b, "\r"),
            None => (body, ""),
        };
        let trimmed = body.trim_end_matches([' ', '\t']);
        if trimmed.len() != body.len() && !(keep_cursor_line && i as i32 == cursor) && !trimmed.is_empty() {
            changed = true;
            out.push_str(trimmed);
        } else {
            out.push_str(body);
        }
        out.push_str(cr);
        out.push_str(end);
    }
    if changed {
        w.replace_all_text(&out);
    }
}
