// The Script menu (executables in ~/.local/share/proza/scripts, CotEditor's naming rules and
// %%%{CotEditorXInput/Output=…}%%% headers) and the Console that collects script errors.

use crate::window::DocWindow;
use adw::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub fn folder() -> PathBuf {
    glib::user_data_dir().join("proza").join("scripts")
}

const SCRIPT_EXTENSIONS: &[&str] = &["sh", "command", "zsh", "bash", "fish", "pl", "php", "rb", "py", "js", "awk", "swift", "lua", "tcl"];

/// "01)Sort Lines.@~s.py" → ("Sort Lines", Some("<Primary><Alt>s")). CotEditor's key spec: @ Command, ~ Option,
/// $ Shift, ^ Control, # keypad; mapped to Ctrl / Alt / Shift / Ctrl.
pub fn parse_name(file_name: &str, is_dir: bool) -> (String, Option<String>) {
    let mut name = file_name.to_string();
    if !is_dir {
        if let Some(dot) = name.rfind('.') {
            name.truncate(dot);
        }
    }
    // the ordering prefix "digits)"
    if let Some(p) = name.find(')') {
        if p > 0 && name[..p].chars().all(|c| c.is_ascii_digit()) {
            name = name[p + 1..].to_string();
        }
    }
    let mut key = None;
    if !is_dir {
        if let Some(dot) = name.rfind('.') {
            let spec = &name[dot + 1..];
            if let Some(accel) = key_spec(spec) {
                key = Some(accel);
                name.truncate(dot);
            }
        }
    }
    (name, key)
}

fn key_spec(spec: &str) -> Option<String> {
    let mut mods = String::new();
    let mut chars = spec.chars().peekable();
    let mut any_mod = false;
    while let Some(&c) = chars.peek() {
        let m = match c {
            '@' | '^' => "<Primary>",
            '~' => "<Alt>",
            '$' => "<Shift>",
            '#' => "",
            _ => break,
        };
        if !mods.contains(m) {
            mods.push_str(m);
        }
        any_mod |= c != '#';
        chars.next();
    }
    let key: String = chars.collect();
    if key.chars().count() != 1 {
        return None;
    }
    let k = key.chars().next()?;
    if k.is_uppercase() && !mods.contains("<Shift>") {
        mods.push_str("<Shift>");
    }
    if !any_mod && !(k.is_uppercase()) {
        return None;
    }
    Some(format!("{mods}{}", k.to_lowercase()))
}

fn is_script(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| SCRIPT_EXTENSIONS.contains(&e))
}

fn natural_sorted(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map(|r| r.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    v.sort_by(|a, b| crate::filebrowser::natural_cmp(&a.file_name().unwrap_or_default().to_string_lossy(), &b.file_name().unwrap_or_default().to_string_lossy()));
    v
}

/// Fills a Script menu: folders become submenus, "-" separators, scripts items; Open Scripts Folder last.
pub fn fill_menu(menu: &gio::Menu) {
    menu.remove_all();
    let dir = folder();
    let _ = std::fs::create_dir_all(&dir);
    let mut accels = Vec::new();
    fill_dir(menu, &dir, &mut accels);
    if let Some(app) = gio::Application::default().and_downcast::<gtk::Application>() {
        for (action, accel) in accels {
            app.set_accels_for_action(&action, &[accel.as_str()]);
        }
    }
}

fn fill_dir(menu: &gio::Menu, dir: &Path, accels: &mut Vec<(String, String)>) {
    let mut section = gio::Menu::new();
    for path in natural_sorted(dir) {
        let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if file_name.starts_with('.') || file_name.starts_with('_') {
            continue;
        }
        let is_dir = path.is_dir();
        let (name, key) = parse_name(&file_name, is_dir);
        if name == "-" {
            if section.n_items() > 0 {
                menu.append_section(None, &section);
                section = gio::Menu::new();
            }
            continue;
        }
        if is_dir {
            let sub = gio::Menu::new();
            fill_dir(&sub, &path, accels);
            section.append_submenu(Some(&name), &sub);
        } else if is_script(&path) {
            let target = path.to_string_lossy().into_owned();
            let item = gio::MenuItem::new(Some(&name), None);
            item.set_action_and_target_value(Some("app.run-script"), Some(&target.to_variant()));
            section.append_item(&item);
            if let Some(k) = key {
                accels.push((format!("app.run-script({})", target.to_variant().print(true)), k));
            }
        }
    }
    if section.n_items() > 0 {
        menu.append_section(None, &section);
    }
}

/// Rebuilds every window's Script menu when the folder changes (0.2 s debounce).
pub fn watch() {
    thread_local! {
        static MONITOR: RefCell<Option<gio::FileMonitor>> = const { RefCell::new(None) };
        static PENDING: RefCell<Option<glib::SourceId>> = const { RefCell::new(None) };
    }
    let dir = folder();
    let _ = std::fs::create_dir_all(&dir);
    let Ok(mon) = gio::File::for_path(&dir).monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE) else { return };
    mon.connect_changed(|_, _, _, _| {
        PENDING.with(|p| {
            if let Some(id) = p.borrow_mut().take() {
                id.remove();
            }
            let id = glib::timeout_add_local_once(std::time::Duration::from_millis(200), || {
                PENDING.with(|p| p.borrow_mut().take());
                for w in crate::window::all() {
                    w.refill_scripts();
                }
            });
            *p.borrow_mut() = Some(id);
        });
    });
    MONITOR.with(|m| *m.borrow_mut() = Some(mon));
}

fn header(source: &str, token: &str) -> Option<String> {
    let re = regex::Regex::new(&format!(r"%%%\{{{token}=(\w+)\}}%%%")).ok()?;
    re.captures(source).map(|c| c[1].to_string())
}

fn script_alert(heading: &str, body: &str) {
    let parent = crate::window::all().into_iter().next().map(|w| w.win.clone());
    let dlg = adw::AlertDialog::new(Some(heading), Some(body));
    dlg.add_response("ok", "OK");
    dlg.present(parent.as_ref());
}

/// Runs a script: $1 = the front document's path, stdin = the requested input, stdout applied as requested,
/// stderr reported in the Console.
pub fn run(path: &Path, target: Option<Rc<DocWindow>>) {
    let name = path.file_name().map(|n| parse_name(&n.to_string_lossy(), false).0).unwrap_or_default();
    if !path.exists() {
        script_alert(&format!("The script “{name}” does not exist."), "");
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    if std::fs::metadata(path).map(|m| m.permissions().mode() & 0o111 == 0).unwrap_or(true) {
        script_alert(&format!("The script “{name}” can’t be executed because you don’t have permission to execute it."), "Check the permission of the script file.");
        return;
    }
    let source = match std::fs::read_to_string(path) {
        Ok(s) if !s.is_empty() => s,
        _ => {
            script_alert(&format!("The script “{name}” couldn’t be read."), "Check the script file.");
            return;
        }
    };
    let input_kind = header(&source, "CotEditorXInput");
    let output_kind = header(&source, "CotEditorXOutput");
    let input = match input_kind.as_deref() {
        None => None,
        Some(kind) => {
            let Some(w) = target.as_ref() else {
                log(&name, "No document to get input from.");
                return;
            };
            let b = &w.buffer;
            let (s, e) = b.selection_bounds().unwrap_or_else(|| {
                let c = b.iter_at_mark(&b.get_insert());
                (c, c)
            });
            Some(match kind {
                "Selection" => b.text(&s, &e, false).to_string(),
                "CurrentLine" => {
                    let (a, z) = line_range(b);
                    b.text(&a, &z, false).to_string()
                }
                "AllText" => w.text(),
                _ => String::new(),
            })
        }
    };
    let mut argv: Vec<std::ffi::OsString> = vec![path.as_os_str().to_owned()];
    if let Some(p) = target.as_ref().and_then(|w| w.doc.borrow().file.as_ref().and_then(|f| f.path())) {
        argv.push(p.into_os_string());
    }
    let mut flags = gio::SubprocessFlags::STDERR_PIPE;
    flags |= if input.is_some() { gio::SubprocessFlags::STDIN_PIPE } else { gio::SubprocessFlags::STDIN_INHERIT };
    if output_kind.is_some() {
        flags |= gio::SubprocessFlags::STDOUT_PIPE;
    } else {
        flags |= gio::SubprocessFlags::STDOUT_SILENCE;
    }
    let args: Vec<&std::ffi::OsStr> = argv.iter().map(|a| a.as_os_str()).collect();
    let proc = match gio::Subprocess::newv(&args, flags) {
        Ok(p) => p,
        Err(e) => {
            log(&name, &e.to_string());
            return;
        }
    };
    let weak = target.as_ref().map(Rc::downgrade);
    proc.communicate_utf8_async(input, gio::Cancellable::NONE, move |res| {
        let (stdout, stderr) = match res {
            Ok((o, e)) => (o.map(|s| s.to_string()).unwrap_or_default(), e.map(|s| s.to_string()).unwrap_or_default()),
            Err(e) => (String::new(), e.to_string()),
        };
        if let Some(kind) = output_kind.as_deref() {
            if let Err(msg) = apply_output(kind, &stdout, weak.as_ref().and_then(|w| w.upgrade())) {
                log(&name, msg);
            }
        }
        if !stderr.is_empty() {
            log(&name, stderr.trim_end());
        }
    });
}

fn line_range(b: &sourceview5::Buffer) -> (gtk::TextIter, gtk::TextIter) {
    let (s, e) = b.selection_bounds().unwrap_or_else(|| {
        let c = b.iter_at_mark(&b.get_insert());
        (c, c)
    });
    let mut a = s;
    a.set_line_offset(0);
    let mut z = e;
    if !(z.starts_line() && z.line() > s.line()) && !z.forward_line() {
        z = b.end_iter();
    }
    (a, z)
}

fn apply_output(kind: &str, out: &str, w: Option<Rc<DocWindow>>) -> Result<(), &'static str> {
    match kind {
        "Pasteboard" => {
            if let Some(d) = gtk::gdk::Display::default() {
                d.clipboard().set_text(out);
            }
            return Ok(());
        }
        "NewDocument" => {
            if !out.is_empty() {
                if let Some(app) = gio::Application::default().and_downcast::<adw::Application>() {
                    let nw = DocWindow::new(&app);
                    nw.set_untitled_text(out);
                    nw.win.present();
                }
            }
            return Ok(());
        }
        _ => {}
    }
    let w = w.ok_or("No document to put output into.")?;
    if !w.view().is_editable() {
        return Err("The document is not editable.");
    }
    let b = &w.buffer;
    let text = crate::document::convert_line_endings(out, w.doc.borrow().ending);
    let (s, e) = b.selection_bounds().unwrap_or_else(|| {
        let c = b.iter_at_mark(&b.get_insert());
        (c, c)
    });
    let (from, to) = match kind {
        "ReplaceSelection" => (s.offset(), e.offset()),
        "ReplaceCurrentLine" => {
            let (a, z) = line_range(b);
            (a.offset(), z.offset())
        }
        "ReplaceAllText" => (0, b.char_count()),
        // CotEditor inserts at the selection's start despite the name; Proza inserts after the selection
        "InsertAfterSelection" => (e.offset(), e.offset()),
        "AppendToAllText" => (b.char_count(), b.char_count()),
        _ => return Ok(()),
    };
    b.begin_user_action();
    let (mut a, mut z) = (b.iter_at_offset(from), b.iter_at_offset(to));
    b.delete(&mut a, &mut z);
    b.insert(&mut a, &text);
    b.end_user_action();
    b.select_range(&b.iter_at_offset(from), &b.iter_at_offset(from + text.chars().count() as i32));
    Ok(())
}

// ---------------------------------------------------------------- Console

struct Console {
    win: adw::Window,
    view: gtk::TextView,
}

thread_local! {
    static CONSOLE: RefCell<Option<Console>> = const { RefCell::new(None) };
}

fn console() -> (adw::Window, gtk::TextView) {
    if let Some(c) = CONSOLE.with(|c| c.borrow().as_ref().map(|c| (c.win.clone(), c.view.clone()))) {
        return c;
    }
    let win = adw::Window::builder().title("Console").default_width(360).default_height(200).hide_on_close(true).build();
    if let Some(app) = gio::Application::default().and_downcast::<gtk::Application>() {
        win.set_application(Some(&app));
    }
    let view = gtk::TextView::builder().editable(false).cursor_visible(false).wrap_mode(gtk::WrapMode::WordChar).top_margin(4).bottom_margin(4).left_margin(6).right_margin(6).build();
    view.add_css_class("console");
    let b = view.buffer();
    let tags = b.tag_table();
    let t = gtk::TextTag::new(Some("title"));
    t.set_weight(600);
    tags.add(&t);
    let m = gtk::TextTag::new(Some("message"));
    m.set_family(Some("monospace"));
    m.set_left_margin(16);
    tags.add(&m);
    let d = gtk::TextTag::new(Some("date"));
    d.set_foreground(Some("gray"));
    tags.add(&d);
    let clear = gtk::Button::builder().icon_name("user-trash-symbolic").tooltip_text("Clear Log").build();
    {
        let b = b.clone();
        clear.connect_clicked(move |_| b.set_text(""));
    }
    let header = adw::HeaderBar::new();
    header.pack_start(&clear);
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&header);
    tv.set_content(Some(&gtk::ScrolledWindow::builder().child(&view).vexpand(true).build()));
    win.set_content(Some(&tv));
    CONSOLE.with(|c| *c.borrow_mut() = Some(Console { win: win.clone(), view: view.clone() }));
    (win, view)
}

pub fn show_console() {
    console().0.present();
}

/// Adds an entry ("[date] title" then the message, monospaced and indented) and shows the Console.
pub fn log(title: &str, message: &str) {
    let (win, view) = console();
    let b = view.buffer();
    let mut end = b.end_iter();
    let date = glib::DateTime::now_local().ok().and_then(|d| d.format("[%Y-%m-%d %H:%M:%S]").ok()).map(|s| s.to_string()).unwrap_or_default();
    b.insert_with_tags_by_name(&mut end, &format!("{date} "), &["date"]);
    b.insert_with_tags_by_name(&mut end, &format!("{title}\n"), &["title"]);
    b.insert_with_tags_by_name(&mut end, &format!("{message}\n"), &["message"]);
    let mark = b.create_mark(None, &b.end_iter(), false);
    view.scroll_mark_onscreen(&mark);
    win.present();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_keys() {
        assert_eq!(parse_name("01)Format JSON.py", false), ("Format JSON".into(), None));
        assert_eq!(parse_name("Sort Lines.@~s.py", false), ("Sort Lines".into(), Some("<Primary><Alt>s".into())));
        assert_eq!(parse_name("Up.@S.sh", false), ("Up".into(), Some("<Primary><Shift>s".into())));
        assert_eq!(parse_name("05)-", false), ("-".into(), None));
        assert_eq!(parse_name("02)Markdown", true), ("Markdown".into(), None));
        assert_eq!(header("# %%%{CotEditorXInput=Selection}%%%", "CotEditorXInput").as_deref(), Some("Selection"));
    }
}
