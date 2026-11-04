// Snippets (Text ▸ Insert Snippet, with <<<SELECTION>>> and <<<CURSOR>>>) and file-drop rules (what dropping files
// on the editor inserts).

use crate::settings;
use crate::window::DocWindow;
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use sourceview5::prelude::*;
use std::path::Path;
use std::rc::Rc;

/// The syntax id of a window's document ("" for plain text).
fn syntax(w: &DocWindow) -> String {
    w.buffer.language().map(|l| l.id().to_string()).unwrap_or_default()
}

fn syntax_name(id: &str) -> String {
    sourceview5::LanguageManager::default().language(id).map(|l| l.name().to_string()).unwrap_or_else(|| id.to_string())
}

/// General snippets, then a section with the current syntax's, then Edit Snippets…. Shortcuts: a scoped snippet
/// wins over a general one with the same keys.
pub fn fill_menu(w: &DocWindow) {
    let menu = &w.dynm.snippets;
    menu.remove_all();
    let s = settings::get();
    let current = syntax(w);
    let general = gio::Menu::new();
    let scoped = gio::Menu::new();
    let scoped_keys: Vec<&str> = s.snippets.iter().filter(|x| !x.scope.is_empty() && x.scope == current && !x.key.is_empty()).map(|x| x.key.as_str()).collect();
    let app = w.win.application();
    for (i, snip) in s.snippets.iter().enumerate() {
        let target = if snip.scope.is_empty() {
            &general
        } else if snip.scope == current {
            &scoped
        } else {
            continue;
        };
        let item = gio::MenuItem::new(Some(&snip.name), None);
        item.set_action_and_target_value(Some("win.insert-snippet"), Some(&(i as u32).to_variant()));
        target.append_item(&item);
        if let Some(app) = &app {
            let action = format!("win.insert-snippet(uint32 {i})");
            let shadowed = snip.scope.is_empty() && scoped_keys.contains(&snip.key.as_str());
            if snip.key.is_empty() || shadowed {
                app.set_accels_for_action(&action, &[]);
            } else {
                app.set_accels_for_action(&action, &[snip.key.as_str()]);
            }
        }
    }
    menu.append_section(None, &general);
    if scoped.n_items() > 0 {
        menu.append_section(Some(&syntax_name(&current)), &scoped);
    }
    let edit = gio::Menu::new();
    edit.append(Some("Edit Snippets…"), Some("app.edit-snippets"));
    menu.append_section(None, &edit);
}

pub fn install(w: &Rc<DocWindow>) {
    let a = gio::SimpleAction::new("insert-snippet", Some(glib::VariantTy::UINT32));
    let weak = Rc::downgrade(w);
    a.connect_activate(move |_, v| {
        let (Some(w), Some(i)) = (weak.upgrade(), v.and_then(|v| v.get::<u32>())) else { return };
        if let Some(snip) = settings::get().snippets.get(i as usize) {
            insert(&w, &snip.format);
        }
    });
    w.actions.add_action(&a);
}

/// Expands a snippet format for one selection: the selection substituted, each newline followed by `indent`;
/// returns the text and the caret offset (chars) if the format has a <<<CURSOR>>>.
pub fn expand(format: &str, selection: &str, indent: &str, eol: &str) -> (String, Option<usize>) {
    let fmt = format.replace("\r\n", "\n").replace('\n', &format!("{eol}{indent}"));
    let (before, after) = match fmt.split_once("<<<CURSOR>>>") {
        Some((b, a)) => (b.to_string(), Some(a.replace("<<<CURSOR>>>", ""))),
        None => (fmt.clone(), None),
    };
    let before = before.replace("<<<SELECTION>>>", selection);
    match after {
        Some(a) => {
            let caret = before.chars().count();
            (before + &a.replace("<<<SELECTION>>>", selection), Some(caret))
        }
        None => (before, None),
    }
}

fn insert(w: &DocWindow, format: &str) {
    if !w.view().is_editable() {
        w.win.error_bell();
        return;
    }
    let b = &w.buffer;
    let (s, e) = b.selection_bounds().unwrap_or_else(|| {
        let c = b.iter_at_mark(&b.get_insert());
        (c, c)
    });
    let selection = b.text(&s, &e, false).to_string();
    let mut ls = s;
    ls.set_line_offset(0);
    let line = b.text(&ls, &s, false).to_string();
    let indent: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let (text, caret) = expand(format, &selection, &indent, w.doc.borrow().ending.as_str());
    let start = s.offset();
    b.begin_user_action();
    let (mut a, mut z) = (s, e);
    b.delete(&mut a, &mut z);
    b.insert(&mut a, &text);
    b.end_user_action();
    let at = start + caret.unwrap_or(text.chars().count()) as i32;
    b.place_cursor(&b.iter_at_offset(at));
    w.view().scroll_mark_onscreen(&b.get_insert());
}

// ---------------------------------------------------------------- file drop

/// What dropping `file` inserts: the first rule whose scope and extensions match, else its absolute path.
pub fn drop_text(file: &Path, document: Option<&Path>, syntax: &str) -> String {
    let ext = file.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
    let rules = settings::get().file_drops;
    let rule = rules.iter().find(|r| {
        (r.scope.is_empty() || r.scope == syntax)
            && (r.extensions.trim().is_empty() || r.extensions.split(',').any(|x| x.trim().eq_ignore_ascii_case(&ext)))
    });
    let abs = file.display().to_string();
    let Some(rule) = rule.filter(|r| !r.format.is_empty()) else { return abs };
    let rel = document.and_then(|d| d.parent()).map(|dir| relative(file, dir)).unwrap_or_else(|| abs.clone());
    let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let stem = file.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let dir = file.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut out = rule
        .format
        .replace("<<<ABSOLUTE-PATH>>>", &abs)
        .replace("<<<RELATIVE-PATH>>>", &rel)
        .replace("<<<FILENAME-NOSUFFIX>>>", &stem)
        .replace("<<<FILENAME>>>", &name)
        .replace("<<<FILEEXTENSION-LOWER>>>", &ext.to_lowercase())
        .replace("<<<FILEEXTENSION-UPPER>>>", &ext.to_uppercase())
        .replace("<<<FILEEXTENSION>>>", &ext)
        .replace("<<<DIRECTORY>>>", &dir);
    if out.contains("<<<FILECONTENT>>>") {
        out = out.replace("<<<FILECONTENT>>>", &std::fs::read_to_string(file).unwrap_or_default());
    }
    if out.contains("<<<IMAGEWIDTH>>>") || out.contains("<<<IMAGEHEIGHT>>>") {
        let (wd, ht) = gtk::gdk_pixbuf::Pixbuf::file_info(file).map(|(_, w, h)| (w.to_string(), h.to_string())).unwrap_or_default();
        out = out.replace("<<<IMAGEWIDTH>>>", &wd).replace("<<<IMAGEHEIGHT>>>", &ht);
    }
    out
}

/// `file` relative to `dir` ("../img/a.png").
pub fn relative(file: &Path, dir: &Path) -> String {
    let f: Vec<_> = file.components().collect();
    let d: Vec<_> = dir.components().collect();
    let common = f.iter().zip(&d).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = std::iter::repeat_n("..".to_string(), d.len() - common).collect();
    parts.extend(f[common..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
    parts.join("/")
}

/// Dropping files on an editor inserts the file-drop text at the drop point (several files: one per line).
pub fn attach_drop(w: &Rc<DocWindow>, view: &sourceview5::View) {
    let target = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    let weak = Rc::downgrade(w);
    let v = view.clone();
    target.connect_drop(move |_, value, x, y| {
        let (Some(w), Ok(list)) = (weak.upgrade(), value.get::<gdk::FileList>()) else { return false };
        if !v.is_editable() {
            return false;
        }
        let doc = w.doc.borrow().file.as_ref().and_then(|f| f.path());
        let lang = syntax(&w);
        let texts: Vec<String> = list.files().iter().filter_map(|f| f.path()).map(|p| drop_text(&p, doc.as_deref(), &lang)).collect();
        let text = texts.join(w.doc.borrow().ending.as_str());
        let (bx, by) = v.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
        let Some(mut at) = v.iter_at_location(bx, by) else { return false };
        let b = &w.buffer;
        b.begin_user_action();
        b.insert(&mut at, &text);
        b.end_user_action();
        true
    });
    view.add_controller(target);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_and_drop() {
        assert_eq!(expand("<b><<<SELECTION>>><<<CURSOR>>></b>", "hi", "", "\n"), ("<b>hi</b>".into(), Some(5)));
        assert_eq!(expand("if {\n\t<<<CURSOR>>>\n}", "", "  ", "\n"), ("if {\n  \t\n  }".into(), Some(8)));
        assert_eq!(relative(Path::new("/a/img/x.png"), Path::new("/a/doc")), "../img/x.png");
        assert_eq!(drop_text(Path::new("/a/img/x.png"), Some(Path::new("/a/notes.md")), "markdown"), "![x](img/x.png)");
        assert_eq!(drop_text(Path::new("/a/f.zip"), None, ""), "/a/f.zip");
    }
}
