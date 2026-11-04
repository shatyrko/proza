// CotEditor's typing aids: automatic symbol balancing (brackets and quotes typed in pairs) and the selection
// instance highlight (other occurrences of a selected word, after 0.5 s).

use crate::settings;
use crate::window::{DocWindow, GENERAL_SYNTAXES};
use adw::prelude::*;
use gtk::{gdk, glib};
use sourceview5::prelude::*;
use std::rc::Rc;

const INSTANCE_TAG: &str = "proza-instance";

/// Brackets always pair; quotes only in syntaxes that have strings (not in plain text or prose syntaxes).
fn closer_for(w: &DocWindow, c: char) -> Option<char> {
    match c {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' | '\'' if quotes_pair(w) => Some(c),
        _ => None,
    }
}

fn quotes_pair(w: &DocWindow) -> bool {
    w.buffer.language().is_some_and(|l| !GENERAL_SYNTAXES.contains(&l.id().as_str()))
}

/// Typing aids for one editor view.
pub fn attach(w: &Rc<DocWindow>, view: &sourceview5::View) {
    view.set_smart_backspace(true);
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak = Rc::downgrade(w);
    keys.connect_key_pressed(move |_, key, _, state| {
        let Some(w) = weak.upgrade() else { return glib::Propagation::Proceed };
        if state.intersects(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK | gdk::ModifierType::SUPER_MASK)
            || !settings::get().balance_symbols
            || !w.view().is_editable()
        {
            return glib::Propagation::Proceed;
        }
        let handled = if key == gdk::Key::BackSpace { delete_pair(&w) } else { key.to_unicode().is_some_and(|c| typed(&w, c)) };
        if handled {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    view.add_controller(keys);
    // Edit ▸ Complete (F5): words of the document, only on request (CotEditor's automatic completion is off)
    let words = sourceview5::CompletionWords::new(Some("Document Words"));
    words.register(&w.buffer);
    let completion = view.completion();
    completion.add_provider(&words);
    completion.block_interactive();
}

/// The caret sits right before the innermost closer Proza inserted itself.
fn at_auto_closer(w: &DocWindow) -> Option<gtk::TextIter> {
    let b = &w.buffer;
    let at = b.iter_at_mark(w.auto_closers.borrow().last()?);
    (at == b.iter_at_mark(&b.get_insert()) && !b.has_selection()).then_some(at)
}

fn pop_auto_closer(w: &DocWindow) {
    if let Some(m) = w.auto_closers.borrow_mut().pop() {
        w.buffer.delete_mark(&m);
    }
}

fn typed(w: &DocWindow, c: char) -> bool {
    let b = &w.buffer;
    // a closer typed right before the closer we inserted steps over it
    if let Some(at) = at_auto_closer(w) {
        if at.char() == c && matches!(c, ')' | ']' | '}' | '"' | '\'') {
            let mut next = at;
            next.forward_char();
            pop_auto_closer(w);
            b.place_cursor(&next);
            return true;
        }
    }
    let Some(closer) = closer_for(w, c) else { return false };
    if let Some((s, e)) = b.selection_bounds() {
        // an opener typed over a selection wraps it
        let (so, eo) = (s.offset(), e.offset());
        b.begin_user_action();
        b.insert(&mut b.iter_at_offset(eo), &closer.to_string());
        b.insert(&mut b.iter_at_offset(so), &c.to_string());
        b.end_user_action();
        b.select_range(&b.iter_at_offset(so + 1), &b.iter_at_offset(eo + 1));
        return true;
    }
    let cursor = b.iter_at_mark(&b.get_insert());
    if cursor.char().is_alphanumeric() {
        return false;
    }
    if c == closer {
        let mut before = cursor;
        if before.backward_char() && (before.char().is_alphanumeric() || before.char() == c) {
            return false;
        }
    }
    b.begin_user_action();
    b.insert_at_cursor(&format!("{c}{closer}"));
    b.end_user_action();
    let mut between = b.iter_at_mark(&b.get_insert());
    between.backward_char();
    // pushed before the caret moves there, so the move doesn't count as leaving the pair
    w.auto_closers.borrow_mut().push(b.create_mark(None, &between, false));
    b.place_cursor(&between);
    true
}

/// Backspace between an empty pair Proza inserted deletes both.
fn delete_pair(w: &DocWindow) -> bool {
    let b = &w.buffer;
    let Some(at) = at_auto_closer(w) else { return false };
    let mut open = at;
    if !open.backward_char() || closer_for(w, open.char()) != Some(at.char()) {
        return false;
    }
    let mut end = at;
    end.forward_char();
    b.begin_user_action();
    pop_auto_closer(w);
    b.delete(&mut open, &mut end);
    b.end_user_action();
    true
}

/// The caret left the innermost pair (other than by stepping over its closer): the inserted closers become
/// ordinary text.
pub fn cursor_moved(w: &DocWindow) {
    let b = &w.buffer;
    let cursor = b.iter_at_mark(&b.get_insert());
    let left = w.auto_closers.borrow().last().is_some_and(|m| b.iter_at_mark(m) != cursor);
    if left {
        for m in w.auto_closers.borrow_mut().drain(..) {
            b.delete_mark(&m);
        }
    }
}

// ---------------------------------------------------------------- selection instance highlight

pub fn instance_tag(b: &sourceview5::Buffer) -> gtk::TextTag {
    let table = b.tag_table();
    table.lookup(INSTANCE_TAG).unwrap_or_else(|| {
        let tag = gtk::TextTag::new(Some(INSTANCE_TAG));
        table.add(&tag);
        tag
    })
}

/// Called on every selection change or edit: clears the highlight and, if the setting is on, schedules a new one.
pub fn selection_changed(w: &Rc<DocWindow>) {
    let b = &w.buffer;
    if let Some(id) = w.instance_id.borrow_mut().take() {
        id.remove();
    }
    if b.tag_table().lookup(INSTANCE_TAG).is_some() {
        b.remove_tag(&instance_tag(b), &b.start_iter(), &b.end_iter());
    }
    if !settings::get().highlight_selection_instance || !b.has_selection() {
        return;
    }
    let weak = Rc::downgrade(w);
    let id = glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
        if let Some(w) = weak.upgrade() {
            w.instance_id.borrow_mut().take();
            highlight_instances(&w);
        }
    });
    w.instance_id.replace(Some(id));
}

/// The selection is one whole word of two or more characters: all its other whole-word occurrences get the tag.
fn highlight_instances(w: &DocWindow) {
    let b = &w.buffer;
    let Some((s, e)) = b.selection_bounds() else { return };
    let word = b.text(&s, &e, false).to_string();
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let chars: Vec<char> = word.chars().collect();
    if chars.len() < 2 || !is_word(chars[0]) || !is_word(chars[chars.len() - 1]) || word.contains('\n') {
        return;
    }
    let mut before = s;
    let mut after = e;
    if (before.backward_char() && is_word(before.char())) || (!after.is_end() && is_word(after.char())) {
        return;
    }
    let Ok(re) = regex::Regex::new(&format!(r"\b{}\b", regex::escape(&word))) else { return };
    let text = w.text();
    let tag = instance_tag(b);
    let own = s.offset();
    let mut char_pos = 0;
    let mut byte_pos = 0;
    for m in re.find_iter(&text).take(10_000) {
        char_pos += text[byte_pos..m.start()].chars().count();
        byte_pos = m.start();
        if char_pos as i32 != own {
            let len = word.chars().count() as i32;
            b.apply_tag(&tag, &b.iter_at_offset(char_pos as i32), &b.iter_at_offset(char_pos as i32 + len));
        }
    }
}
