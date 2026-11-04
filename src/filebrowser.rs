// Folder windows: the File Browser sidebar (live tree of the folder, folders first in natural order, hidden files
// on request), its footer (+ New File / New Folder, filter) and context menu, in-place renaming, and the preview
// page for files that aren't text.

use crate::settings;
use crate::window::DocWindow;
use adw::prelude::*;
use gtk::{gdk, gio, glib, pango};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const ATTRIBUTES: &str = "standard::name,standard::display-name,standard::type,standard::is-hidden,standard::is-backup,\
standard::symbolic-icon,standard::content-type,standard::is-symlink,standard::symlink-target,access::can-write";

pub struct Browser {
    root_dir: gio::File,
    selection: gtk::SingleSelection,
    list: gtk::ListView,
    filter_entry: gtk::SearchEntry,
    filter_results: gtk::ListBox,
    filter_paths: RefCell<Vec<PathBuf>>,
    stack: gtk::Stack,
    /// A file created by New File / New Folder: its row starts renaming when it appears.
    pending_rename: RefCell<Option<PathBuf>>,
    filter_id: RefCell<Option<glib::SourceId>>,
    /// Hidden-file filters of all directory levels, re-evaluated when the setting changes.
    filters: RefCell<Vec<gtk::CustomFilter>>,
    opening: Cell<bool>,
}

thread_local! {
    static BROWSERS: RefCell<Vec<(glib::WeakRef<adw::ApplicationWindow>, Rc<Browser>)>> = const { RefCell::new(Vec::new()) };
}

fn browser(w: &DocWindow) -> Option<Rc<Browser>> {
    BROWSERS.with(|b| b.borrow().iter().find(|(win, _)| win.upgrade().as_ref() == Some(&w.win)).map(|(_, b)| b.clone()))
}

/// "file 2" before "file 10", case-insensitive.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let mut n = String::new();
                while let Some(c) = x.peek().copied().filter(char::is_ascii_digit) {
                    n.push(c);
                    x.next();
                }
                let mut m = String::new();
                while let Some(d) = y.peek().copied().filter(char::is_ascii_digit) {
                    m.push(d);
                    y.next();
                }
                let (n, m) = (n.trim_start_matches('0'), m.trim_start_matches('0'));
                match n.len().cmp(&m.len()).then(n.cmp(m)) {
                    Ordering::Equal => {}
                    o => return o,
                }
            }
            (Some(c), Some(d)) => {
                x.next();
                y.next();
                match c.cmp(&d) {
                    Ordering::Equal => {}
                    o => return o,
                }
            }
        }
    }
}

fn info_of(item: &glib::Object) -> Option<gio::FileInfo> {
    item.downcast_ref::<gtk::TreeListRow>().and_then(|r| r.item()).and_then(|i| i.downcast::<gio::FileInfo>().ok())
}

fn file_of(info: &gio::FileInfo) -> Option<gio::File> {
    info.attribute_object("standard::file").and_then(|o| o.downcast::<gio::File>().ok())
}

fn is_hidden(info: &gio::FileInfo) -> bool {
    info.is_hidden() || info.is_backup() || info.name().to_string_lossy().starts_with('.')
}

/// Text-like files open as text; others as a preview. Files with no extension, or that a syntax claims, are text.
pub fn is_text(file: &gio::File, content_type: Option<&str>) -> bool {
    let name = file.basename().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    if !name.contains('.') || name.starts_with('.') && name.matches('.').count() == 1 {
        return true;
    }
    let lm = sourceview5::LanguageManager::default();
    if lm.guess_language(Some(&name), None::<&str>).is_some() {
        return true;
    }
    match content_type {
        Some(ct) => gio::content_type_is_a(ct, "text/plain") || ct == "application/x-zerosize" || ct.starts_with("text/"),
        None => true,
    }
}

fn directory_model(dir: &gio::File, filters: &RefCell<Vec<gtk::CustomFilter>>) -> gio::ListModel {
    let list = gtk::DirectoryList::new(Some(ATTRIBUTES), Some(dir));
    list.set_monitored(true);
    let filter = gtk::CustomFilter::new(|o| {
        let Some(info) = o.downcast_ref::<gio::FileInfo>() else { return false };
        let name = info.name();
        let name = name.to_string_lossy();
        if name == ".git" || name == ".DS_Store" {
            return false;
        }
        settings::get().file_browser_hidden || !is_hidden(info)
    });
    filters.borrow_mut().push(filter.clone());
    let filtered = gtk::FilterListModel::new(Some(list), Some(filter));
    let sorter = gtk::CustomSorter::new(|a, b| {
        let (Some(a), Some(b)) = (a.downcast_ref::<gio::FileInfo>(), b.downcast_ref::<gio::FileInfo>()) else { return gtk::Ordering::Equal };
        let dir = |i: &gio::FileInfo| i.file_type() == gio::FileType::Directory;
        let o = dir(b).cmp(&dir(a)).then_with(|| natural_cmp(&a.display_name(), &b.display_name()));
        o.into()
    });
    gtk::SortListModel::new(Some(filtered), Some(sorter)).upcast()
}

/// View ▸ Sidebar: only enabled in folder windows.
pub fn install(w: &Rc<DocWindow>) {
    w.add_toggle("sidebar", false, |w, on| w.outer.set_show_sidebar(on));
    if let Some(a) = w.actions.lookup_action("sidebar").and_downcast::<gio::SimpleAction>() {
        a.set_enabled(false);
    }
}

/// Opens `folder` in a folder window (the window already showing it, else the blank window, else a new one).
pub fn open_folder(app: &adw::Application, folder: &gio::File) {
    if let Some(w) = crate::window::all().into_iter().find(|w| w.folder.borrow().as_ref().is_some_and(|f| f.equal(folder))) {
        w.win.present();
        return;
    }
    let w = crate::window::all().into_iter().find(|w| w.is_blank() && w.folder.borrow().is_none()).unwrap_or_else(|| DocWindow::new(app));
    attach(&w, folder);
    w.win.present();
}

fn attach(w: &Rc<DocWindow>, folder: &gio::File) {
    w.folder.replace(Some(folder.clone()));
    let filters = RefCell::new(Vec::new());
    let root_model = directory_model(folder, &filters);
    let tree = gtk::TreeListModel::new(root_model, false, false, {
        // child models are created lazily; their filters are registered through the browser once it exists
        let w = Rc::downgrade(w);
        move |item| {
            let info = item.downcast_ref::<gio::FileInfo>()?;
            if info.file_type() != gio::FileType::Directory {
                return None;
            }
            let dir = file_of(info)?;
            let b = browser(&*w.upgrade()?)?;
            Some(directory_model(&dir, &b.filters))
        }
    });
    let selection = gtk::SingleSelection::builder().model(&tree).autoselect(false).can_unselect(true).build();
    selection.set_selected(gtk::INVALID_LIST_POSITION);
    let factory = gtk::SignalListItemFactory::new();
    let list = gtk::ListView::new(Some(selection.clone()), Some(factory.clone()));
    list.add_css_class("navigation-sidebar");
    list.add_css_class("file-browser");

    let scroller = gtk::ScrolledWindow::builder().child(&list).vexpand(true).hscrollbar_policy(gtk::PolicyType::Never).build();
    let filter_results = gtk::ListBox::new();
    filter_results.add_css_class("navigation-sidebar");
    let none = gtk::Label::new(Some("No Filter Results"));
    none.add_css_class("dim-label");
    filter_results.set_placeholder(Some(&none));
    let results_scroller = gtk::ScrolledWindow::builder().child(&filter_results).vexpand(true).hscrollbar_policy(gtk::PolicyType::Never).build();
    let stack = gtk::Stack::new();
    stack.add_named(&scroller, Some("tree"));
    stack.add_named(&results_scroller, Some("filter"));

    // footer: + menu and filter field
    let add_menu = gio::Menu::new();
    add_menu.append(Some("New File"), Some("browser.new-file"));
    add_menu.append(Some("New Folder"), Some("browser.new-folder"));
    let add = gtk::MenuButton::builder().icon_name("list-add-symbolic").menu_model(&add_menu).direction(gtk::ArrowType::Up).tooltip_text("Add").build();
    add.add_css_class("circular");
    add.add_css_class("flat");
    let filter_entry = gtk::SearchEntry::builder().placeholder_text("Filter").hexpand(true).build();
    filter_entry.update_property(&[gtk::accessible::Property::Label("File browser filter")]);
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    footer.set_margin_start(8);
    footer.set_margin_end(8);
    footer.set_margin_top(6);
    footer.set_margin_bottom(6);
    footer.append(&add);
    footer.append(&filter_entry);
    let side = gtk::Box::new(gtk::Orientation::Vertical, 0);
    side.add_css_class("file-browser-side");
    side.append(&stack);
    side.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    side.append(&footer);

    let b = Rc::new(Browser {
        root_dir: folder.clone(),
        selection: selection.clone(),
        list: list.clone(),
        filter_entry: filter_entry.clone(),
        filter_results: filter_results.clone(),
        filter_paths: RefCell::new(Vec::new()),
        stack,
        pending_rename: RefCell::new(None),
        filter_id: RefCell::new(None),
        filters,
        opening: Cell::new(false),
    });
    BROWSERS.with(|all| all.borrow_mut().push((w.win.downgrade(), b.clone())));
    {
        let win = w.win.downgrade();
        w.win.connect_destroy(move |_| BROWSERS.with(|all| all.borrow_mut().retain(|(x, _)| x.upgrade().is_some() && x != &win)));
    }

    setup_factory(w, &b, &factory);
    install_actions(w, &b, &side);

    // a single click on a file shows it; a double click on a folder expands it
    {
        let weak = Rc::downgrade(w);
        selection.connect_selected_item_notify(move |sel| {
            let (Some(w), Some(item)) = (weak.upgrade(), sel.selected_item()) else { return };
            let Some(info) = info_of(&item) else { return };
            if info.file_type() == gio::FileType::Directory {
                return;
            }
            if let Some(file) = file_of(&info) {
                show_file(&w, &file, info.content_type().as_deref(), false);
            }
        });
        list.connect_activate(move |list, pos| {
            if let Some(row) = list.model().and_then(|m| m.item(pos)).and_downcast::<gtk::TreeListRow>() {
                if row.is_expandable() {
                    row.set_expanded(!row.is_expanded());
                }
            }
        });
    }
    // filter: 0.5 s after typing, a recursive case-insensitive substring search on names
    {
        let weak = Rc::downgrade(w);
        filter_entry.connect_search_changed(move |_| {
            let Some(w) = weak.upgrade() else { return };
            let Some(b) = browser(&w) else { return };
            if let Some(id) = b.filter_id.borrow_mut().take() {
                id.remove();
            }
            let weak = Rc::downgrade(&w);
            let id = glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
                if let Some(w) = weak.upgrade() {
                    if let Some(b) = browser(&w) {
                        b.filter_id.borrow_mut().take();
                        run_filter(&b);
                    }
                }
            });
            b.filter_id.replace(Some(id));
        });
        let weak = Rc::downgrade(w);
        filter_results.connect_row_activated(move |_, row| {
            let Some(w) = weak.upgrade() else { return };
            let Some(b) = browser(&w) else { return };
            let Some(path) = b.filter_paths.borrow().get(row.index() as usize).cloned() else { return };
            let file = gio::File::for_path(&path);
            if path.is_file() {
                let ct = gio::content_type_guess(Some(&path), None).0;
                show_file(&w, &file, Some(ct.as_str()), false);
            }
        });
    }

    w.outer.set_sidebar(Some(&side));
    w.outer.set_show_sidebar(true);
    w.outer.set_sidebar_width_fraction(0.22);
    if let Some(a) = w.actions.lookup_action("sidebar").and_downcast::<gio::SimpleAction>() {
        a.set_enabled(true);
        a.set_state(&true.to_variant());
    }
    w.update_title();
}

fn setup_factory(w: &Rc<DocWindow>, b: &Rc<Browser>, factory: &gtk::SignalListItemFactory) {
    let weak_w = Rc::downgrade(w);
    factory.connect_setup(move |_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().expect("list item");
        let icon = gtk::Image::new();
        // the name, swapped for an entry while renaming (an editable label would take every click)
        let label = gtk::Label::builder().xalign(0.0).ellipsize(pango::EllipsizeMode::Middle).build();
        let entry = gtk::Entry::new();
        let name = gtk::Stack::new();
        name.add_named(&label, Some("label"));
        name.add_named(&entry, Some("entry"));
        name.set_hexpand(true);
        let link = gtk::Image::from_icon_name("emblem-symbolic-link");
        link.set_pixel_size(10);
        link.add_css_class("dim-label");
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        row.append(&icon);
        row.append(&name);
        row.append(&link);
        let expander = gtk::TreeExpander::new();
        expander.set_child(Some(&row));
        item.set_child(Some(&expander));
        // right click: select the row, then the context menu at the pointer
        let click = gtk::GestureClick::builder().button(gdk::BUTTON_SECONDARY).build();
        let weak = weak_w.clone();
        let li = item.clone();
        click.connect_pressed(move |g, _, x, y| {
            let Some(w) = weak.upgrade() else { return };
            let Some(b) = browser(&w) else { return };
            b.selection.set_selected(li.position());
            if let Some(widget) = g.widget() {
                if let Some(p) = widget.compute_point(&b.list, &gtk::graphene::Point::new(x as f32, y as f32)) {
                    popup_context_menu(&b, p.x() as f64, p.y() as f64);
                }
            }
        });
        expander.add_controller(click);
        // Return or leaving the field renames; Escape cancels
        let finish = {
            let weak = weak_w.clone();
            let li = item.clone();
            let (name, label) = (name.clone(), label.clone());
            move |entry: &gtk::Entry, commit: bool| {
                if name.visible_child_name().as_deref() != Some("entry") {
                    return;
                }
                name.set_visible_child_name("label");
                let (Some(w), Some(info)) = (weak.upgrade(), li.item().and_then(|i| info_of(&i))) else { return };
                let new = entry.text().to_string();
                if commit && new != info.display_name().as_str() {
                    rename(&w, &info, &new, &label);
                }
            }
        };
        {
            let finish = finish.clone();
            entry.connect_activate(move |e| finish(e, true));
        }
        {
            let finish = finish.clone();
            let focus = gtk::EventControllerFocus::new();
            let e = entry.clone();
            focus.connect_leave(move |_| finish(&e, true));
            entry.add_controller(focus);
        }
        {
            let keys = gtk::EventControllerKey::new();
            let e = entry.clone();
            keys.connect_key_pressed(move |_, key, _, _| {
                if key == gdk::Key::Escape {
                    finish(&e, false);
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
            entry.add_controller(keys);
        }
    });
    let weak_b = Rc::downgrade(b);
    factory.connect_bind(move |_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().expect("list item");
        let Some(row) = item.item().and_downcast::<gtk::TreeListRow>() else { return };
        let Some(expander) = item.child().and_downcast::<gtk::TreeExpander>() else { return };
        expander.set_list_row(Some(&row));
        let Some(info) = row.item().and_downcast::<gio::FileInfo>() else { return };
        let Some(bx) = expander.child().and_downcast::<gtk::Box>() else { return };
        let icon = bx.first_child().and_downcast::<gtk::Image>().expect("icon");
        let name = icon.next_sibling().and_downcast::<gtk::Stack>().expect("name");
        let label = name.child_by_name("label").and_downcast::<gtk::Label>().expect("label");
        let link = name.next_sibling().expect("link");
        if let Some(gicon) = info.symbolic_icon() {
            icon.set_from_gicon(&gicon);
        }
        name.set_visible_child_name("label");
        label.set_label(&info.display_name());
        link.set_visible(info.is_symlink());
        let hidden = is_hidden(&info);
        icon.set_opacity(if hidden { 0.5 } else { 1.0 });
        if hidden {
            label.add_css_class("dim-label");
        } else {
            label.remove_css_class("dim-label");
        }
        // a file just made by New File / New Folder starts renaming
        let Some(b) = weak_b.upgrade() else { return };
        let path = file_of(&info).and_then(|f| f.path());
        if path.is_some() && *b.pending_rename.borrow() == path {
            b.pending_rename.replace(None);
            b.selection.set_selected(item.position());
            let name = name.clone();
            glib::idle_add_local_once(move || start_rename(&name));
        }
    });
}

/// Shows the entry in a name cell, filled with the name and the part before the extension selected.
fn start_rename(name: &gtk::Stack) {
    let (Some(label), Some(entry)) = (name.child_by_name("label").and_downcast::<gtk::Label>(), name.child_by_name("entry").and_downcast::<gtk::Entry>()) else {
        return;
    };
    let text = label.label().to_string();
    entry.set_text(&text);
    name.set_visible_child_name("entry");
    entry.grab_focus();
    let stem = text.rfind('.').filter(|&i| i > 0).map_or(text.chars().count(), |i| text[..i].chars().count());
    entry.select_region(0, stem as i32);
}

fn rename(w: &Rc<DocWindow>, info: &gio::FileInfo, new: &str, label: &gtk::Label) {
    let Some(file) = file_of(info) else { return };
    let error = if new.is_empty() {
        Some("The name can’t be empty.")
    } else if new.contains('/') || new.contains(':') {
        Some("The name can’t contain “/” or “:”.")
    } else if new.contains('\n') {
        Some("The name can’t contain a line break.")
    } else if new.len() > 255 {
        Some("The name is too long.")
    } else if file.parent().is_some_and(|p| p.child(new).query_exists(gio::Cancellable::NONE)) {
        Some("The name is already taken.")
    } else {
        None
    };
    let result = match error {
        Some(e) => Err(e.to_string()),
        None => file.set_display_name(new, gio::Cancellable::NONE).map_err(|e| e.to_string()),
    };
    match result {
        Ok(renamed) => {
            let current = w.doc.borrow().file.as_ref().is_some_and(|f| f.equal(&file));
            if current {
                w.doc.borrow_mut().file = Some(renamed.clone());
                w.update_title();
                let lang = w.guess_syntax_for(&renamed);
                w.set_syntax(&lang);
            }
        }
        Err(e) => {
            label.set_label(&info.display_name());
            alert(w, &format!("“{new}” can’t be used."), &e);
        }
    }
}

fn alert(w: &DocWindow, heading: &str, body: &str) {
    let dlg = adw::AlertDialog::new(Some(heading), Some(body));
    dlg.add_response("ok", "OK");
    dlg.present(Some(&w.win));
}

fn selected_file(b: &Browser) -> Option<(gio::File, gio::FileInfo)> {
    let info = b.selection.selected_item().and_then(|i| info_of(&i))?;
    Some((file_of(&info)?, info))
}

/// New File / New Folder target: the selected folder, the selected file's folder, or the root.
fn target_dir(b: &Browser) -> gio::File {
    match selected_file(b) {
        Some((f, info)) if info.file_type() == gio::FileType::Directory => f,
        Some((f, _)) => f.parent().unwrap_or_else(|| b.root_dir.clone()),
        None => b.root_dir.clone(),
    }
}

/// "Untitled.txt", "Untitled 2.txt"…, or "name copy.txt", "name copy 2.txt"… for duplicates.
fn unique(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let dot = if ext.is_empty() { String::new() } else { format!(".{ext}") };
    let mut path = dir.join(format!("{stem}{dot}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem} {n}{dot}"));
        n += 1;
    }
    path
}

fn install_actions(w: &Rc<DocWindow>, b: &Rc<Browser>, side: &gtk::Box) {
    let group = gio::SimpleActionGroup::new();
    let add = |name: &str, f: fn(&Rc<DocWindow>, &Rc<Browser>)| {
        let a = gio::SimpleAction::new(name, None);
        let (ww, wb) = (Rc::downgrade(w), Rc::downgrade(b));
        a.connect_activate(move |_, _| {
            if let (Some(w), Some(b)) = (ww.upgrade(), wb.upgrade()) {
                f(&w, &b)
            }
        });
        group.add_action(&a);
    };
    add("new-file", |w, b| {
        let Some(dir) = target_dir(b).path() else { return };
        let path = unique(&dir, "Untitled", "txt");
        if let Err(e) = std::fs::write(&path, "") {
            alert(w, "The file couldn’t be created.", &e.to_string());
            return;
        }
        b.filter_entry.set_text("");
        expand_to(b, &dir);
        b.pending_rename.replace(Some(path));
    });
    add("new-folder", |w, b| {
        let Some(dir) = target_dir(b).path() else { return };
        let path = unique(&dir, "untitled folder", "");
        if let Err(e) = std::fs::create_dir(&path) {
            alert(w, "The folder couldn’t be created.", &e.to_string());
            return;
        }
        b.filter_entry.set_text("");
        expand_to(b, &dir);
        b.pending_rename.replace(Some(path));
    });
    add("show-in-files", |_, b| {
        if let Some((f, _)) = selected_file(b) {
            crate::app::show_in_files(&f);
        }
    });
    add("open-new-window", |w, b| {
        let Some((f, info)) = selected_file(b) else { return };
        let Some(app) = w.win.application().and_downcast::<adw::Application>() else { return };
        if info.file_type() == gio::FileType::Directory {
            let nw = DocWindow::new(&app);
            attach(&nw, &f);
            nw.win.present();
        } else {
            if w.doc.borrow().file.as_ref().is_some_and(|c| c.equal(&f)) {
                w.save_if_auto();
                w.close_document();
            }
            let nw = DocWindow::new(&app);
            nw.load_file(&f, None);
            nw.win.present();
        }
    });
    add("open-external", |w, b| {
        if let Some((f, _)) = selected_file(b) {
            gtk::FileLauncher::new(Some(&f)).launch(Some(&w.win), gio::Cancellable::NONE, |_| {});
        }
    });
    add("rename", |_, b| {
        let pos = b.selection.selected();
        if pos == gtk::INVALID_LIST_POSITION {
            return;
        }
        // the row widget of the selection: walk the list's children to the bound one
        let mut child = b.list.first_child();
        while let Some(c) = child {
            if let Some(exp) = c.first_child().and_downcast::<gtk::TreeExpander>() {
                if exp.list_row().is_some_and(|r| r.position() == pos) {
                    if let Some(name) = exp.child().and_then(|bx| bx.first_child()).and_then(|i| i.next_sibling()).and_downcast::<gtk::Stack>() {
                        start_rename(&name);
                    }
                    return;
                }
            }
            child = c.next_sibling();
        }
    });
    add("duplicate", |w, b| {
        let Some((f, _)) = selected_file(b) else { return };
        let Some(path) = f.path() else { return };
        let dir = path.parent().unwrap_or(Path::new("/")).to_path_buf();
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let ext = if path.is_dir() { String::new() } else { path.extension().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default() };
        let target = unique(&dir, &format!("{stem} copy"), &ext);
        let result = if path.is_dir() {
            std::process::Command::new("cp").arg("-a").arg(&path).arg(&target).status().map(|_| ())
        } else {
            std::fs::copy(&path, &target).map(|_| ())
        };
        if let Err(e) = result {
            alert(w, "The item couldn’t be duplicated.", &e.to_string());
        }
    });
    add("trash", |w, b| {
        let Some((f, info)) = selected_file(b) else { return };
        let dlg = adw::AlertDialog::new(Some(&format!("Do you want to move “{}” to the Trash?", info.display_name())), Some("This operation cannot be undone."));
        dlg.add_responses(&[("trash", "Move to Trash"), ("cancel", "Cancel")]);
        dlg.set_response_appearance("trash", adw::ResponseAppearance::Destructive);
        dlg.set_close_response("cancel");
        let weak = Rc::downgrade(w);
        dlg.connect_response(Some("trash"), move |_, _| {
            let Some(w) = weak.upgrade() else { return };
            let shown = w.doc.borrow().file.clone();
            if shown.as_ref().is_some_and(|s| s.equal(&f) || s.has_prefix(&f)) {
                w.close_document();
            }
            if let Err(e) = f.trash(gio::Cancellable::NONE) {
                alert(&w, "The item couldn’t be moved to the Trash.", &e.to_string());
            }
        });
        dlg.present(Some(&w.win));
    });
    let hidden = gio::SimpleAction::new_stateful("show-hidden", None, &settings::get().file_browser_hidden.to_variant());
    let wb = Rc::downgrade(b);
    hidden.connect_activate(move |a, _| {
        let on = !a.state().and_then(|v| v.get::<bool>()).unwrap_or(false);
        a.set_state(&on.to_variant());
        settings::update(|s| s.file_browser_hidden = on);
        if let Some(b) = wb.upgrade() {
            for f in b.filters.borrow().iter() {
                f.changed(gtk::FilterChange::Different);
            }
        }
    });
    group.add_action(&hidden);
    side.insert_action_group("browser", Some(&group));
}

/// Expands the tree rows down to `dir` (so a new item there shows up).
fn expand_to(b: &Browser, dir: &Path) {
    let Some(model) = b.selection.model() else { return };
    for i in 0..model.n_items() {
        let Some(row) = model.item(i).and_downcast::<gtk::TreeListRow>() else { continue };
        let Some(path) = row.item().and_downcast::<gio::FileInfo>().and_then(|i| file_of(&i)).and_then(|f| f.path()) else { continue };
        if dir.starts_with(&path) && !row.is_expanded() {
            row.set_expanded(true);
        }
    }
}

fn popup_context_menu(b: &Browser, x: f64, y: f64) {
    let target = selected_file(b);
    let menu = gio::Menu::new();
    if let Some((_, info)) = &target {
        let s1 = gio::Menu::new();
        s1.append(Some("Show in Files"), Some("browser.show-in-files"));
        menu.append_section(None, &s1);
        let s2 = gio::Menu::new();
        s2.append(Some("Open in New Window"), Some("browser.open-new-window"));
        if info.file_type() != gio::FileType::Directory {
            s2.append(Some("Open with External Editor"), Some("browser.open-external"));
        }
        menu.append_section(None, &s2);
        let s3 = gio::Menu::new();
        s3.append(Some("Rename"), Some("browser.rename"));
        s3.append(Some("Move to Trash"), Some("browser.trash"));
        s3.append(Some("Duplicate"), Some("browser.duplicate"));
        menu.append_section(None, &s3);
    }
    let s4 = gio::Menu::new();
    s4.append(Some("New File"), Some("browser.new-file"));
    s4.append(Some("New Folder"), Some("browser.new-folder"));
    menu.append_section(None, &s4);
    let s5 = gio::Menu::new();
    s5.append(Some("Show Hidden Files"), Some("browser.show-hidden"));
    menu.append_section(None, &s5);
    let pop = gtk::PopoverMenu::from_model(Some(&menu));
    pop.set_parent(&b.list);
    pop.set_has_arrow(false);
    pop.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
    pop.connect_closed(|p| {
        let p = p.clone();
        glib::idle_add_local_once(move || p.unparent());
    });
    pop.popup();
}

fn run_filter(b: &Browser) {
    let query = b.filter_entry.text().to_lowercase();
    while let Some(r) = b.filter_results.row_at_index(0) {
        b.filter_results.remove(&r);
    }
    b.filter_paths.borrow_mut().clear();
    if query.is_empty() {
        b.stack.set_visible_child_name("tree");
        return;
    }
    b.stack.set_visible_child_name("filter");
    let Some(root) = b.root_dir.path() else { return };
    let hidden = settings::get().file_browser_hidden;
    let mut found = Vec::new();
    let mut dirs = vec![root.clone()];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by(|a, b| natural_cmp(&a.file_name().to_string_lossy(), &b.file_name().to_string_lossy()));
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            if name == ".git" || name == ".DS_Store" || (!hidden && name.starts_with('.')) {
                continue;
            }
            let path = e.path();
            if name.to_lowercase().contains(&query) {
                found.push(path.clone());
            }
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                dirs.push(path);
            }
            if found.len() >= 2000 {
                break;
            }
        }
    }
    found.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
    for path in &found {
        let rel = path.strip_prefix(&root).unwrap_or(path);
        let name = rel.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let parent = rel.parent().map(|p| p.display().to_string()).unwrap_or_default();
        // the matched part in the primary colour, the rest dimmed
        let lower = name.to_lowercase();
        let markup = match lower.find(&query) {
            Some(at) if name.is_char_boundary(at) && name.is_char_boundary(at + query.len()) => format!(
                "<span alpha='60%'>{}</span>{}<span alpha='60%'>{}</span>",
                glib::markup_escape_text(&name[..at]),
                glib::markup_escape_text(&name[at..at + query.len()]),
                glib::markup_escape_text(&name[at + query.len()..])
            ),
            _ => glib::markup_escape_text(&name).to_string(),
        };
        let icon = gtk::Image::from_icon_name(if path.is_dir() { "folder-symbolic" } else { "text-x-generic-symbolic" });
        let label = gtk::Label::builder().xalign(0.0).ellipsize(pango::EllipsizeMode::Middle).build();
        label.set_markup(&markup);
        let sub = gtk::Label::builder().label(parent).xalign(0.0).ellipsize(pango::EllipsizeMode::Start).build();
        sub.add_css_class("caption");
        sub.add_css_class("dim-label");
        let texts = gtk::Box::new(gtk::Orientation::Vertical, 0);
        texts.append(&label);
        if !sub.label().is_empty() {
            texts.append(&sub);
        }
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        row.append(&icon);
        row.append(&texts);
        b.filter_results.append(&row);
    }
    *b.filter_paths.borrow_mut() = found;
}

// ---------------------------------------------------------------- showing a file in the folder window

/// Shows `file` in the window: as text when it is text-like (or `as_text`), else as a preview.
pub fn show_file(w: &Rc<DocWindow>, file: &gio::File, content_type: Option<&str>, as_text: bool) {
    if w.doc.borrow().file.as_ref().is_some_and(|f| f.equal(file)) && w.content_stack.visible_child_name().as_deref() == Some("text") {
        return;
    }
    if let Some(other) = crate::window::all().into_iter().find(|o| !Rc::ptr_eq(o, w) && o.doc.borrow().file.as_ref().is_some_and(|f| f.equal(file))) {
        let name = file.basename().map(|p| p.display().to_string()).unwrap_or_default();
        alert(w, &format!("The file “{name}” is already open in another window."), "To open it in this window, close the existing window first.");
        other.win.present();
        return;
    }
    // the shown document is saved first (auto-save); unsaved untitled text is never thrown away
    if w.buffer.is_modified() {
        if w.doc.borrow().file.is_some() {
            w.save_if_auto();
        } else if let Some(app) = w.win.application().and_downcast::<adw::Application>() {
            crate::app::open_file(&app, file);
            return;
        }
    }
    if as_text || is_text(file, content_type) {
        if w.load_file(file, None) {
            w.content_stack.set_visible_child_name("text");
            return;
        }
    }
    show_preview(w, file, content_type);
}

fn show_preview(w: &Rc<DocWindow>, file: &gio::File, content_type: Option<&str>) {
    if let Some(old) = w.content_stack.child_by_name("preview") {
        w.content_stack.remove(&old);
    }
    let name = file.basename().map(|p| p.display().to_string()).unwrap_or_default();
    let ct = content_type.map(str::to_string).unwrap_or_else(|| gio::content_type_guess(file.path().as_deref(), None).0.to_string());
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.set_valign(gtk::Align::Center);
    page.set_halign(gtk::Align::Center);
    page.add_css_class("file-preview");
    let picture: gtk::Widget = if ct.starts_with("image/") {
        let p = gtk::Picture::for_file(file);
        p.set_can_shrink(true);
        p.set_size_request(256, 256);
        p.upcast()
    } else {
        let i = gtk::Image::from_gicon(&gio::content_type_get_icon(&ct));
        i.set_pixel_size(128);
        i.upcast()
    };
    page.append(&picture);
    let title = gtk::Label::new(Some(&name));
    title.add_css_class("heading");
    title.add_css_class("dim-label");
    page.append(&title);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::Center);
    let external = gtk::Button::with_label("Open with External Editor");
    let plain = gtk::Button::with_label("Open as Plain Text");
    buttons.append(&external);
    buttons.append(&plain);
    page.append(&buttons);
    let grid = gtk::Grid::builder().column_spacing(8).row_spacing(4).halign(gtk::Align::Center).build();
    let mut rows = vec![("Kind", gio::content_type_get_description(&ct).to_string())];
    if let Some(target) = file.query_info("standard::symlink-target", gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS, gio::Cancellable::NONE).ok().and_then(|i| i.symlink_target()) {
        rows.push(("Original", target.display().to_string()));
    }
    if ct.starts_with("image/") {
        if let Some((_, wd, ht)) = file.path().and_then(|p| gtk::gdk_pixbuf::Pixbuf::file_info(p)) {
            rows.push(("Dimensions", format!("{wd}\u{200A}×\u{200A}{ht}")));
        }
    }
    for (n, (k, v)) in rows.into_iter().enumerate() {
        let kl = gtk::Label::builder().label(format!("{k}:")).xalign(1.0).build();
        kl.add_css_class("dim-label");
        grid.attach(&kl, 0, n as i32, 1, 1);
        grid.attach(&gtk::Label::builder().label(v).xalign(0.0).selectable(true).build(), 1, n as i32, 1, 1);
    }
    page.append(&grid);
    {
        let f = file.clone();
        let win = w.win.clone();
        external.connect_clicked(move |_| {
            gtk::FileLauncher::new(Some(&f)).launch(Some(&win), gio::Cancellable::NONE, |_| {});
        });
        let f = file.clone();
        let weak = Rc::downgrade(w);
        let ct2 = ct.clone();
        plain.connect_clicked(move |_| {
            if let Some(w) = weak.upgrade() {
                show_file(&w, &f, Some(&ct2), true);
            }
        });
    }
    w.content_stack.add_named(&page, Some("preview"));
    w.content_stack.set_visible_child_name("preview");
    w.close_document();
    w.show_title(&name);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order() {
        let mut v = vec!["file10", "File2", "file1", "a"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["a", "file1", "File2", "file10"]);
    }
}
