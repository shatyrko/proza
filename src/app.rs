// Application level: start-up, app actions (New, Open, Settings, About, Quit), recent files, crash backups.

use crate::window::{self, DocWindow};
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use std::path::PathBuf;
use std::rc::Rc;

const CSS: &str = "
.proza-window .navigation-bar { min-height: 24px; padding: 0 4px; }
.proza-window .navigation-bar button { min-height: 20px; min-width: 20px; padding: 0 2px; }
.proza-window .navigation-bar .outline-popup { font-weight: 600; }
.proza-window .document-title { font-weight: 700; font-size: 1.1em; margin: 0 6px; }
.proza-window .proza-editor gutter { padding-left: 10px; }
.proza-window .status-bar { min-height: 28px; padding: 0 6px 0 14px; font-size: 0.92em;
    border-top: 1px solid alpha(currentColor, 0.12); }
.proza-window .status-bar separator { margin: 5px 4px; }
.proza-window .status-bar button { min-height: 20px; padding: 0 6px; font-weight: normal; }
.proza-window .syntax-popup > button { border-radius: 999px; padding: 2px 12px; min-height: 30px; }
.proza-window headerbar .circular { min-width: 34px; min-height: 34px; }
.proza-window .navigation-bar .rotated image { -gtk-icon-transform: rotate(-90deg); }
.outline-badge { min-width: 15px; min-height: 15px; border-radius: 4px; font-size: 0.72em; font-weight: 800;
    color: white; padding: 0 1px; }
.outline-badge.container { background: #3478f6; }
.outline-badge.value { background: #30b04f; }
.outline-badge.function { background: #f28c1b; }
.outline-badge.title { background: #2aa5a5; }
.outline-badge.heading { background: #5856d6; }
.outline-badge.mark { background: transparent; color: #ff453a; font-size: 1em; }
.outline-menu { background: transparent; }
.file-preview { padding: 24px; }
.opaque-bars .status-bar, .opaque-bars .navigation-bar { background: @headerbar_bg_color; }
.settings-pane { margin: 20px 24px 16px 24px; }
.settings-pane .row-label { color: alpha(currentColor, 0.85); }
.settings-pane .subtitle { font-size: 0.85em; color: alpha(currentColor, 0.6); }
.settings-tabs button { padding: 4px 8px; min-width: 64px; }
.settings-tabs button label { font-size: 0.85em; }
.settings-list { border: 1px solid alpha(currentColor, 0.15); border-radius: 7px; }
.settings-list-bar { border-top: 1px solid alpha(currentColor, 0.15); padding: 2px; }
.shortcut-recorder { font-family: monospace; min-width: 110px; }
.file-browser-side { background: alpha(currentColor, 0.03); }
.character-glyph { font-family: serif; font-size: 48pt; }
.code-point-chip { border: 1px solid alpha(currentColor, 0.3); border-radius: 4px; padding: 0 4px; }
.inspector expander title { padding: 2px 0; }
.outline-menu row { padding: 3px 6px; border-radius: 6px; }
.find-panel .find-field-frame { border: 1px solid alpha(currentColor, 0.15); border-radius: 7px; }
.find-panel .find-field-frame:focus-within { outline: 2px solid alpha(@accent_color, 0.5); outline-offset: 1px; }
.find-panel .find-field, .find-panel .find-field text { background: transparent; }
.find-panel .history-button { min-height: 24px; min-width: 24px; padding: 2px; margin: 4px; }
.find-panel .help-button { min-width: 22px; min-height: 22px; padding: 0; }
";

/// Puts the bundled themes and icons in place and loads the stylesheet. Runs once at start-up.
pub fn startup() {
    sourceview5::init();
    crate::theme::install();
    crate::scripts::watch();
    adw::StyleManager::default().set_color_scheme(match crate::settings::get().appearance.as_str() {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    });
    install_icons();
    install_languages();
    let css = gtk::CssProvider::new();
    css.load_from_string(CSS);
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("display"),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_str!(concat!("../data/icons/", $name, ".svg")))),*]
    };
}
const ICONS: &[(&str, &str)] = icons![
    "proza-updown-symbolic", "proza-info-symbolic", "proza-split-add-symbolic", "proza-not-editable-symbolic",
    "proza-sidebar-symbolic", "proza-clear-symbolic",
];

fn install_icons() {
    let dir = glib::user_data_dir().join("proza").join("icons");
    let actions = dir.join("hicolor/scalable/actions");
    let _ = std::fs::create_dir_all(&actions);
    for (name, svg) in ICONS {
        let path = actions.join(format!("{name}.svg"));
        if std::fs::read_to_string(&path).ok().as_deref() != Some(svg) {
            let _ = std::fs::write(&path, svg);
        }
    }
    gtk::IconTheme::for_display(&gdk::Display::default().expect("display")).add_search_path(&dir);
}

/// Proza's own syntax definitions (for syntaxes CotEditor has and GtkSourceView doesn't), in id order.
pub const LANGUAGES: &[(&str, &str)] = &[("svg", include_str!("../data/language-specs/svg.lang"))];

fn install_languages() {
    let dir = glib::user_data_dir().join("proza").join("language-specs");
    let _ = std::fs::create_dir_all(&dir);
    for (id, xml) in LANGUAGES {
        let path = dir.join(format!("{id}.lang"));
        if std::fs::read_to_string(&path).ok().as_deref() != Some(xml) {
            let _ = std::fs::write(&path, xml);
        }
    }
    let lm = sourceview5::LanguageManager::default();
    let mut paths: Vec<String> = vec![dir.to_string_lossy().into_owned()];
    paths.extend(lm.search_path().iter().map(|p| p.to_string()));
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    lm.set_search_path(&refs);
}

/// The window a newly opened file goes to: the untouched blank window if there is one, else a new one.
fn target_window(app: &adw::Application) -> Rc<DocWindow> {
    window::all().into_iter().find(|w| w.is_blank()).unwrap_or_else(|| DocWindow::new(app))
}

pub fn open_file(app: &adw::Application, file: &gio::File) {
    if file.query_file_type(gio::FileQueryInfoFlags::NONE, gio::Cancellable::NONE) == gio::FileType::Directory {
        crate::filebrowser::open_folder(app, file);
        return;
    }
    // already open: bring that window forward
    if let Some(w) = window::all().into_iter().find(|w| w.doc.borrow().file.as_ref().is_some_and(|f| f.equal(file))) {
        w.win.present();
        return;
    }
    let w = target_window(app);
    if w.load_file(file, None) {
        w.win.present();
    } else {
        let name = file.basename().map(|p| p.display().to_string()).unwrap_or_default();
        let dlg = adw::AlertDialog::new(
            Some(&format!("The file “{name}” couldn’t be opened.")),
            Some("It may not be a text file, or its text encoding isn’t in the encoding list."),
        );
        dlg.add_response("ok", "OK");
        w.win.present();
        dlg.present(Some(&w.win));
    }
}

pub fn new_document(app: &adw::Application) {
    DocWindow::new(app).win.present();
}

pub fn install_actions(app: &adw::Application) {
    let add = |name: &str, f: fn(&adw::Application)| {
        let a = gio::SimpleAction::new(name, None);
        let app2 = app.clone();
        a.connect_activate(move |_, _| f(&app2));
        app.add_action(&a);
    };
    add("new", new_document);
    add("new-window", new_document);
    add("open", |app| {
        let dialog = gtk::FileDialog::builder().title("Open").modal(true).build();
        let app2 = app.clone();
        dialog.open_multiple(app.active_window().as_ref(), gio::Cancellable::NONE, move |res| {
            if let Ok(files) = res {
                for f in files.iter::<gio::File>().flatten() {
                    open_file(&app2, &f);
                }
            }
        });
    });
    add("open-folder", |app| {
        let dialog = gtk::FileDialog::builder().title("Open Folder").modal(true).build();
        let app2 = app.clone();
        dialog.select_folder(app.active_window().as_ref(), gio::Cancellable::NONE, move |res| {
            if let Ok(folder) = res {
                crate::filebrowser::open_folder(&app2, &folder);
            }
        });
    });
    add("open-scripts-folder", |_| {
        let dir = crate::scripts::folder();
        let _ = std::fs::create_dir_all(&dir);
        gtk::FileLauncher::new(Some(&gio::File::for_path(&dir))).launch(None::<&gtk::Window>, gio::Cancellable::NONE, |_| {});
    });
    add("settings", |_| crate::settingswin::show(None));
    add("edit-snippets", |_| crate::settingswin::show(Some(6)));
    add("quit", |app| {
        for w in window::all() {
            w.win.close();
        }
        if window::all().is_empty() {
            app.quit();
        }
    });
    add("about", |app| {
        let about = adw::AboutDialog::builder()
            .application_name("Proza")
            .application_icon("text-x-generic")
            .version(env!("CARGO_PKG_VERSION"))
            .comments("A lightweight plain-text editor, modelled on CotEditor.")
            .website("https://github.com/shatyrko/proza")
            .license_type(gtk::License::Gpl30)
            .developer_name("Oleksii Shatyrko")
            .build();
        about.add_legal_section(
            "CotEditor themes",
            Some("© 2014-2026 1024jp and contributors"),
            gtk::License::Apache20,
            None,
        );
        about.present(app.active_window().as_ref());
    });
    add("website", |_| {
        gtk::UriLauncher::new("https://github.com/shatyrko/proza").launch(None::<&gtk::Window>, gio::Cancellable::NONE, |_| {});
    });
    add("report-issue", |_| {
        gtk::UriLauncher::new("https://github.com/shatyrko/proza/issues").launch(None::<&gtk::Window>, gio::Cancellable::NONE, |_| {});
    });
    let run = gio::SimpleAction::new("run-script", Some(glib::VariantTy::STRING));
    let app3 = app.clone();
    run.connect_activate(move |_, v| {
        if let Some(path) = v.and_then(|v| v.get::<String>()) {
            // the target is the front document window
            let target = app3.active_window().and_then(|aw| window::all().into_iter().find(|w| w.win.upcast_ref::<gtk::Window>() == &aw)).or_else(|| window::all().into_iter().next());
            crate::scripts::run(std::path::Path::new(&path), target);
        }
    });
    app.add_action(&run);
    let recent = gio::SimpleAction::new("open-recent", Some(glib::VariantTy::STRING));
    let app2 = app.clone();
    recent.connect_activate(move |_, v| {
        if let Some(uri) = v.and_then(|v| v.get::<String>()) {
            open_file(&app2, &gio::File::for_uri(&uri));
        }
    });
    app.add_action(&recent);
    let clear = gio::SimpleAction::new("clear-recent", None);
    clear.connect_activate(|_, _| {
        let rm = gtk::RecentManager::default();
        for item in rm.items() {
            if item.has_application("Proza") {
                let _ = rm.remove_item(&item.uri());
            }
        }
    });
    app.add_action(&clear);

    apply_shortcuts();
}

/// The shortcuts of a command: the user's (Settings ▸ Shortcuts) or the default.
pub fn accels_for(action: &str) -> Vec<String> {
    if let Some(custom) = crate::settings::get().shortcuts.get(action) {
        return custom.clone();
    }
    crate::menu::accels().into_iter().find(|(a, _)| *a == action).map(|(_, k)| k.iter().map(|s| s.to_string()).collect()).unwrap_or_default()
}

/// Default shortcuts with the user's changes on top.
pub fn apply_shortcuts() {
    let Some(app) = gio::Application::default().and_downcast::<gtk::Application>() else { return };
    for (action, accels) in crate::menu::accels() {
        app.set_accels_for_action(action, accels);
    }
    for (action, keys) in crate::settings::get().shortcuts {
        let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
        app.set_accels_for_action(&action, &refs);
    }
}

// ---------------------------------------------------------------- recent files

pub fn add_recent(file: &gio::File) {
    let data = gtk::RecentData::new(None, None, "text/plain", "Proza", "proza %u", &[], false);
    gtk::RecentManager::default().add_full(&file.uri(), &data);
}

/// Open Recent: Proza's recent files (newest first), then Clear Menu.
pub fn fill_recent(menu: &gio::Menu) {
    menu.remove_all();
    let mut items: Vec<gtk::RecentInfo> =
        gtk::RecentManager::default().items().into_iter().filter(|i| i.has_application("Proza") && i.exists()).collect();
    items.sort_by_key(|i| std::cmp::Reverse(i.modified().to_unix()));
    let files = gio::Menu::new();
    for i in items.iter().take(10) {
        let it = gio::MenuItem::new(Some(&i.display_name()), None);
        it.set_action_and_target_value(Some("app.open-recent"), Some(&i.uri().to_variant()));
        files.append_item(&it);
    }
    menu.append_section(None, &files);
    let clear = gio::Menu::new();
    clear.append(Some("Clear Menu"), Some("app.clear-recent"));
    menu.append_section(None, &clear);
}

pub fn show_in_files(file: &gio::File) {
    let uri = file.uri().to_string();
    gio::bus_get(gio::BusType::Session, gio::Cancellable::NONE, move |bus| {
        if let Ok(bus) = bus {
            bus.call(
                Some("org.freedesktop.FileManager1"),
                "/org/freedesktop/FileManager1",
                "org.freedesktop.FileManager1",
                "ShowItems",
                Some(&(vec![uri], "").to_variant()),
                None,
                gio::DBusCallFlags::NONE,
                -1,
                gio::Cancellable::NONE,
                |_| {},
            );
        }
    });
}

// ---------------------------------------------------------------- crash backups (unsaved, untitled text)

fn backup_dir() -> PathBuf {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| glib::home_dir().join(".local/state"));
    state.join("proza").join("backups")
}

pub fn write_backup(w: &DocWindow) {
    if w.doc.borrow().file.is_some() || !w.buffer.is_modified() {
        return;
    }
    let dir = backup_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{}.txt", w.doc.borrow().backup_id));
    let text = w.text();
    if text.is_empty() {
        let _ = std::fs::remove_file(path);
    } else {
        let _ = std::fs::write(path, text);
    }
}

pub fn remove_backup(id: &str) {
    let _ = std::fs::remove_file(backup_dir().join(format!("{id}.txt")));
}

/// Untitled documents that were not saved when Proza last quit or crashed: reopened as they were.
/// Returns how many windows were restored.
pub fn restore_backups(app: &adw::Application) -> usize {
    let Ok(entries) = std::fs::read_dir(backup_dir()) else { return 0 };
    let mut n = 0;
    for e in entries.flatten() {
        let path = e.path();
        if let Ok(text) = std::fs::read_to_string(&path) {
            let w = DocWindow::new(app);
            w.set_untitled_text(&text);
            if let Some(stem) = path.file_stem() {
                w.doc.borrow_mut().backup_id = stem.to_string_lossy().into_owned();
            }
            w.win.present();
            n += 1;
        }
    }
    n
}
