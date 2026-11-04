// The Settings window: CotEditor's nine panes (General, Appearance, Window, Edit, Mode, Format, Snippets,
// Shortcuts, Integration) as icon + label tabs in the title bar. Every control applies at once.

use crate::settings::{self, Mode, Settings};
use crate::theme;
use adw::prelude::*;
use gtk::{gdk, gio, glib, pango};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;

const PANES: [(&str, &str); 9] = [
    ("General", "emblem-system-symbolic"),
    ("Appearance", "preferences-desktop-appearance-symbolic"),
    ("Window", "view-dual-symbolic"),
    ("Edit", "document-edit-symbolic"),
    ("Mode", "toggles-symbolic"),
    ("Format", "text-x-generic-symbolic"),
    ("Snippets", "insert-text-symbolic"),
    ("Shortcuts", "input-keyboard-symbolic"),
    ("Integration", "utilities-terminal-symbolic"),
];

thread_local! {
    static WINDOW: RefCell<Option<(adw::Window, Vec<gtk::ToggleButton>)>> = const { RefCell::new(None) };
}

/// Changes a setting and applies it to every window.
fn set(f: impl FnOnce(&mut Settings)) {
    settings::update(f);
    crate::window::settings_changed();
}

/// Settings… (or a given pane): the window opens on the last pane used.
pub fn show(pane: Option<usize>) {
    let (win, buttons) = WINDOW.with(|w| w.borrow().clone()).unwrap_or_else(build);
    let n = pane.unwrap_or(settings::get().settings_pane as usize).min(PANES.len() - 1);
    buttons[n].set_active(true);
    win.present();
}

fn build() -> (adw::Window, Vec<gtk::ToggleButton>) {
    let win = adw::Window::builder().title("General").resizable(false).hide_on_close(true).build();
    if let Some(app) = gio::Application::default().and_downcast::<gtk::Application>() {
        win.set_application(Some(&app));
    }
    let stack = gtk::Stack::builder().vhomogeneous(false).hhomogeneous(true).build();
    let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    tabs.add_css_class("settings-tabs");
    let mut buttons: Vec<gtk::ToggleButton> = Vec::new();
    let pages: [fn(&adw::Window) -> gtk::Widget; 9] = [general, appearance, window_pane, edit, mode, format, snippets, shortcuts, integration];
    for (i, ((label, icon), page)) in PANES.iter().zip(pages).enumerate() {
        let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
        content.append(&gtk::Image::from_icon_name(icon));
        content.append(&gtk::Label::new(Some(label)));
        let b = gtk::ToggleButton::builder().child(&content).build();
        b.add_css_class("flat");
        if let Some(first) = buttons.first() {
            b.set_group(Some(first));
        }
        tabs.append(&b);
        let pane = gtk::Box::new(gtk::Orientation::Vertical, 12);
        pane.add_css_class("settings-pane");
        pane.set_width_request(580);
        pane.append(&page(&win));
        let help = gtk::Button::builder().label("?").halign(gtk::Align::End).tooltip_text("Help").build();
        help.add_css_class("circular");
        let anchor = format!("settings_{}", label.to_lowercase());
        help.connect_clicked(move |_| {
            gtk::UriLauncher::new(&format!("https://coteditor.com/manual/{anchor}")).launch(None::<&gtk::Window>, gio::Cancellable::NONE, |_| {});
        });
        pane.append(&help);
        stack.add_named(&pane, Some(label));
        let (st, w) = (stack.clone(), win.clone());
        b.connect_toggled(move |b| {
            if b.is_active() {
                st.set_visible_child_name(label);
                w.set_title(Some(label));
                settings::update(|s| s.settings_pane = i as u32);
            }
        });
        buttons.push(b);
    }
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&tabs));
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&header);
    // tall panes (Appearance) scroll rather than outgrow the screen
    let scroller = gtk::ScrolledWindow::builder().child(&stack).hscrollbar_policy(gtk::PolicyType::Never).propagate_natural_height(true).max_content_height(880).build();
    tv.set_content(Some(&scroller));
    win.set_content(Some(&tv));
    let keys = gtk::EventControllerKey::new();
    let w = win.clone();
    keys.connect_key_pressed(move |_, k, _, _| {
        if k == gdk::Key::Escape {
            w.close();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    win.add_controller(keys);
    WINDOW.with(|cell| *cell.borrow_mut() = Some((win.clone(), buttons.clone())));
    (win, buttons)
}

// ---------------------------------------------------------------- building blocks

fn form() -> gtk::Grid {
    gtk::Grid::builder().column_spacing(8).row_spacing(14).build()
}

fn rows(grid: &gtk::Grid) -> i32 {
    let mut n = 0;
    let mut c = grid.first_child();
    while let Some(w) = c {
        let (_, r, _, h) = grid.query_child(&w);
        n = n.max(r + h);
        c = w.next_sibling();
    }
    n
}

/// A form row: the right-aligned label ("On startup:") and the control(s) on the right.
fn row(grid: &gtk::Grid, label: &str, control: &impl IsA<gtk::Widget>) {
    let n = rows(grid);
    let l = gtk::Label::builder().label(label).xalign(1.0).yalign(0.0).margin_top(4).build();
    l.add_css_class("row-label");
    grid.attach(&l, 0, n, 1, 1);
    grid.attach(control, 1, n, 1, 1);
}

fn divider(grid: &gtk::Grid) {
    let n = rows(grid);
    grid.attach(&gtk::Separator::new(gtk::Orientation::Horizontal), 0, n, 2, 1);
}

fn vbox(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Vertical, spacing)
}

fn hbox(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, spacing)
}

fn check(label: &str, get: fn(&Settings) -> bool, put: fn(&mut Settings, bool)) -> gtk::CheckButton {
    let c = gtk::CheckButton::builder().label(label).active(get(&settings::get())).build();
    c.connect_toggled(move |c| {
        let on = c.is_active();
        set(|s| put(s, on));
    });
    c
}

fn subtitle(text: &str) -> gtk::Label {
    let l = gtk::Label::builder().label(text).xalign(0.0).wrap(true).max_width_chars(60).margin_start(26).build();
    l.add_css_class("subtitle");
    l
}

fn spin(min: f64, max: f64, step: f64, digits: u32, get: fn(&Settings) -> f64, put: fn(&mut Settings, f64)) -> gtk::SpinButton {
    let s = gtk::SpinButton::with_range(min, max, step);
    s.set_digits(digits);
    s.set_value(get(&settings::get()));
    s.connect_value_changed(move |s| {
        let v = s.value();
        set(|st| put(st, v));
    });
    s
}

/// A group of radio buttons: (value, label).
fn radios(options: &[(&'static str, &str)], get: fn(&Settings) -> String, put: fn(&mut Settings, &str), vertical: bool) -> gtk::Box {
    let b = if vertical { vbox(4) } else { hbox(14) };
    let current = get(&settings::get());
    let mut first: Option<gtk::CheckButton> = None;
    for (value, label) in options {
        let r = gtk::CheckButton::builder().label(*label).active(current == *value).build();
        if let Some(f) = &first {
            r.set_group(Some(f));
        } else {
            first = Some(r.clone());
        }
        let value = *value;
        r.connect_toggled(move |r| {
            if r.is_active() {
                set(|s| put(s, value));
            }
        });
        b.append(&r);
    }
    b
}

fn dropdown(items: &[&str], selected: usize, on: impl Fn(usize) + 'static) -> gtk::DropDown {
    let d = gtk::DropDown::from_strings(items);
    d.set_selected(selected as u32);
    d.set_halign(gtk::Align::Start);
    d.connect_selected_notify(move |d| on(d.selected() as usize));
    d
}

fn alert(parent: &impl IsA<gtk::Widget>, heading: &str, body: &str) {
    let dlg = adw::AlertDialog::new(Some(heading), Some(body));
    dlg.add_response("ok", "OK");
    dlg.present(Some(parent));
}

/// A bordered list with the + / − bar under it.
fn list_box_with_bar(list: &gtk::ListBox, bar: &gtk::Box, height: i32) -> gtk::Box {
    let frame = vbox(0);
    frame.add_css_class("settings-list");
    let sc = gtk::ScrolledWindow::builder().child(list).min_content_height(height).hscrollbar_policy(gtk::PolicyType::Never).build();
    frame.append(&sc);
    bar.add_css_class("settings-list-bar");
    frame.append(bar);
    frame
}

fn icon_button(icon: &str, tip: &str) -> gtk::Button {
    let b = gtk::Button::builder().icon_name(icon).tooltip_text(tip).build();
    b.add_css_class("flat");
    b
}

fn clear(list: &gtk::ListBox) {
    while let Some(r) = list.row_at_index(0) {
        list.remove(&r);
    }
}

// ---------------------------------------------------------------- 1. General

fn general(win: &adw::Window) -> gtk::Widget {
    let g = form();
    let s = settings::get();
    let startup = vbox(6);
    let reopen = gtk::CheckButton::builder().label("Reopen windows from last session").active(s.reopen_windows).build();
    let warned = std::cell::Cell::new(false);
    let w = win.clone();
    reopen.connect_toggled(move |c| {
        let on = c.is_active();
        set(|s| s.reopen_windows = on);
        if !warned.replace(true) {
            alert(&w, "The change will be applied first on the next launch.", "The app’s behavior will change from the next launch after that one.");
        }
    });
    startup.append(&reopen);
    let nothing = hbox(6);
    nothing.set_margin_start(20);
    nothing.append(&gtk::Label::new(Some("When nothing else is open:")));
    let values = ["new", "open", "none"];
    nothing.append(&dropdown(
        &["Create New Document", "Show Open Dialog", "Do Nothing"],
        values.iter().position(|v| *v == s.nothing_open).unwrap_or(0),
        move |i| set(|s| s.nothing_open = values[i].into()),
    ));
    startup.append(&nothing);
    row(&g, "On startup:", &startup);

    let save = vbox(2);
    save.append(&check("Enable Auto Save", |s| s.auto_save, |s, v| s.auto_save = v));
    save.append(&subtitle("Saves files in place while you edit. Even if turned off, Proza keeps a backup of unsaved text in case it unexpectedly quits."));
    row(&g, "Document save:", &save);

    let n = rows(&g);
    let ext_label = gtk::Label::builder().label("When document is changed by another application:").xalign(0.0).build();
    ext_label.add_css_class("row-label");
    g.attach(&ext_label, 0, n, 2, 1);
    let ext = radios(
        &[("keep", "Keep Proza’s version"), ("ask", "Ask how to resolve"), ("update", "Update to modified version")],
        |s| s.external_change.clone(),
        |s, v| s.external_change = v.into(),
        true,
    );
    g.attach(&ext, 1, n + 1, 1, 1);

    let ie = hbox(8);
    let export = gtk::Button::with_label("Export…");
    let import = gtk::Button::with_label("Import…");
    ie.append(&export);
    ie.append(&import);
    row(&g, "Settings import & export:", &ie);
    let w = win.clone();
    export.connect_clicked(move |_| {
        let dlg = gtk::FileDialog::builder().title("Export Settings").initial_name("proza-settings.json").build();
        dlg.save(Some(&w), gio::Cancellable::NONE, |res| {
            if let Ok(f) = res {
                if let (Some(p), Ok(json)) = (f.path(), serde_json::to_string_pretty(&settings::get())) {
                    let _ = std::fs::write(p, json);
                }
            }
        });
    });
    let w = win.clone();
    import.connect_clicked(move |_| {
        let dlg = gtk::FileDialog::builder().title("Choose a Proza settings file to import.").accept_label("Import").build();
        let w2 = w.clone();
        dlg.open(Some(&w), gio::Cancellable::NONE, move |res| {
            let Ok(f) = res else { return };
            match f.path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str::<Settings>(&t).ok()) {
                Some(imported) => {
                    set(|s| *s = imported);
                    alert(&w2, "The settings were imported.", "Reopen the Settings window to see them.");
                }
                None => alert(&w2, "The file couldn’t be imported.", "It isn’t a Proza settings file."),
            }
        });
    });

    let manage = gtk::Button::builder().label("Manage Warnings…").halign(gtk::Align::Start).build();
    row(&g, "Dialog warnings:", &manage);
    let w = win.clone();
    manage.connect_clicked(move |_| {
        let bx = vbox(8);
        bx.append(&gtk::Label::builder().label("Suppress the following warnings:").xalign(0.0).build());
        bx.append(&check("Inconsistent line endings", |s| s.suppress_inconsistent_endings, |s, v| s.suppress_inconsistent_endings = v));
        let dlg = adw::AlertDialog::new(None, None);
        dlg.set_extra_child(Some(&bx));
        dlg.add_response("done", "Done");
        dlg.present(Some(&w));
    });
    g.upcast()
}

// ---------------------------------------------------------------- 2. Appearance

fn show_font(name: &gtk::Label, d: &pango::FontDescription) {
    let family = d.family().map(|f| f.to_string()).unwrap_or_default();
    name.set_markup(&format!(
        "<span font_desc='{}'>{} {}</span>",
        glib::markup_escape_text(&d.to_string()),
        glib::markup_escape_text(&family),
        d.size() / pango::SCALE
    ));
    name.set_tooltip_text(Some(&d.to_string()));
}

fn put_font(mono: bool, d: &pango::FontDescription) {
    let text = d.to_string();
    set(|s| if mono { s.mono_font = text } else { s.font = text });
}

/// A font row: the font name shown in itself, a size stepper, Select…, then Antialias / Ligatures.
fn font_row(win: &adw::Window, mono: bool) -> gtk::Box {
    let s = settings::get();
    let desc = pango::FontDescription::from_string(if mono { &s.mono_font } else { &s.font });
    let name = gtk::Label::builder().xalign(0.0).max_width_chars(28).ellipsize(pango::EllipsizeMode::End).build();
    show_font(&name, &desc);
    let size = gtk::SpinButton::with_range(4.0, 288.0, 1.0);
    size.set_value((desc.size() / pango::SCALE) as f64);
    let select = gtk::Button::with_label("Select…");
    let top = hbox(8);
    top.append(&name);
    top.append(&size);
    top.append(&select);
    {
        let name = name.clone();
        size.connect_value_changed(move |sp| {
            let s = settings::get();
            let mut d = pango::FontDescription::from_string(if mono { &s.mono_font } else { &s.font });
            if d.size() / pango::SCALE == sp.value() as i32 {
                return;
            }
            d.set_size(sp.value() as i32 * pango::SCALE);
            show_font(&name, &d);
            put_font(mono, &d);
        });
    }
    {
        let w = win.clone();
        let (name, size) = (name.clone(), size.clone());
        select.connect_clicked(move |_| {
            let s = settings::get();
            let current = pango::FontDescription::from_string(if mono { &s.mono_font } else { &s.font });
            let (name, size, w2) = (name.clone(), size.clone(), w.clone());
            gtk::FontDialog::new().choose_font(Some(&w), Some(&current), gio::Cancellable::NONE, move |res| {
                let Ok(d) = res else { return };
                let family = d.family().map(|f| f.to_string()).unwrap_or_default();
                let apply = move || {
                    show_font(&name, &d);
                    put_font(mono, &d);
                    size.set_value((d.size() / pango::SCALE) as f64);
                };
                // CotEditor checks that the monospaced font is fixed-pitch
                let fixed = w2.pango_context().list_families().iter().find(|f| f.name() == family).is_some_and(|f| f.is_monospace());
                if mono && !fixed {
                    let a = adw::AlertDialog::new(Some("The selected font doesn’t seem to be monospaced."), Some("Do you want to use it for the monospaced font?"));
                    a.add_responses(&[("cancel", "Cancel"), ("ok", "OK")]);
                    a.connect_response(Some("ok"), move |_, _| apply());
                    a.present(Some(&w2));
                    return;
                }
                apply();
            });
        });
    }
    let opts = hbox(14);
    if mono {
        opts.append(&check("Antialias", |s| s.mono_antialias, |s, v| s.mono_antialias = v));
        opts.append(&check("Ligatures", |s| s.mono_ligatures, |s, v| s.mono_ligatures = v));
    } else {
        opts.append(&check("Antialias", |s| s.font_antialias, |s, v| s.font_antialias = v));
        opts.append(&check("Ligatures", |s| s.font_ligatures, |s, v| s.font_ligatures = v));
    }
    let b = vbox(6);
    b.append(&top);
    b.append(&opts);
    b
}

fn appearance(win: &adw::Window) -> gtk::Widget {
    let page = vbox(16);
    let g = form();
    row(&g, "Standard font:", &font_row(win, false));
    row(&g, "Monospaced font:", &font_row(win, true));
    let lh = hbox(6);
    lh.append(&spin(0.1, 10.0, 0.1, 1, |s| s.line_height, |s, v| s.line_height = v));
    lh.append(&gtk::Label::new(Some("times")));
    row(&g, "Line height:", &lh);
    row(&g, "Appearance:", &radios(&[("default", "Match System"), ("light", "Light"), ("dark", "Dark")], |s| s.appearance.clone(), |s, v| s.appearance = v.into(), false));
    row(
        &g,
        "Status bar:",
        &radios(&[("tinted", "Tinted"), ("opaque", "Opaque")], |s| if s.opaque_bars { "opaque".into() } else { "tinted".into() }, |s, v| s.opaque_bars = v == "opaque", false),
    );
    let op = hbox(8);
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 20.0, 100.0, 1.0);
    for m in (20..=100).step_by(10) {
        scale.add_mark(m as f64, gtk::PositionType::Bottom, None);
    }
    scale.set_value(settings::get().editor_opacity * 100.0);
    scale.set_width_request(220);
    let pct = gtk::Label::new(Some(&format!("{}%", (settings::get().editor_opacity * 100.0).round())));
    {
        let pct = pct.clone();
        scale.connect_value_changed(move |sc| {
            let v = sc.value().round();
            pct.set_label(&format!("{v}%"));
            set(|s| s.editor_opacity = v / 100.0);
        });
    }
    op.append(&scale);
    op.append(&pct);
    row(&g, "Editor opacity:", &op);
    page.append(&g);
    page.append(&theme_editor(win));
    page.upcast()
}

const COLOR_KEYS: [(&str, &str, bool); 7] = [
    ("text", "Text:", false),
    ("invisibles", "Invisibles:", false),
    ("insertionPoint", "Cursor:", true),
    ("background", "Background:", false),
    ("lineHighlight", "Current Line:", false),
    ("selection", "Selection:", true),
    ("highlight", "Highlight:", true),
];
const SYNTAX_KEYS: [(&str, &str); 10] = [
    ("keywords", "Keywords:"),
    ("commands", "Commands:"),
    ("types", "Types:"),
    ("attributes", "Attributes:"),
    ("variables", "Variables:"),
    ("values", "Values:"),
    ("numbers", "Numbers:"),
    ("strings", "Strings:"),
    ("characters", "Characters:"),
    ("comments", "Comments:"),
];

fn hex(c: &gdk::RGBA) -> String {
    let b = |v: f32| (v * 255.0).round() as u8;
    if c.alpha() < 0.999 {
        format!("#{:02x}{:02x}{:02x}{:02x}", b(c.red()), b(c.green()), b(c.blue()), b(c.alpha()))
    } else {
        format!("#{:02x}{:02x}{:02x}", b(c.red()), b(c.green()), b(c.blue()))
    }
}

/// CotEditor's new theme: black on white, gray invisibles and syntax colours, system colours elsewhere.
fn untitled_theme() -> Value {
    let mut t = json!({
        "text": {"color": "#000000"}, "background": {"color": "#ffffff"}, "invisibles": {"color": "#b3b3b3"},
        "lineHighlight": {"color": "#f2f2f2"},
        "selection": {"color": "#a7caff", "usesSystemSetting": true},
        "insertionPoint": {"color": "#000000", "usesSystemSetting": true},
        "highlight": {"color": "#7f7f7f", "usesSystemSetting": true},
        "metadata": {}
    });
    for (k, _) in SYNTAX_KEYS {
        t[k] = json!({"color": "#7f7f7f"});
    }
    t
}

fn unique_theme_name(base: &str) -> String {
    let names = theme::all_names();
    if !names.iter().any(|n| n == base) {
        return base.into();
    }
    (2..).map(|n| format!("{base} {n}")).find(|c| !names.contains(c)).unwrap_or_default()
}

/// Selecting a theme makes it the default; one whose darkness differs from the appearance is pinned.
fn choose_theme(name: &str) {
    let dark = adw::StyleManager::default().is_dark();
    let pin = name.ends_with("(Dark)") != dark;
    let n = name.to_string();
    settings::update(|s| {
        s.theme = n;
        s.pin_theme_appearance = pin;
    });
    for w in crate::window::all() {
        w.vs.borrow_mut().theme = name.to_string();
    }
    crate::window::settings_changed();
}

fn theme_editor(win: &adw::Window) -> gtk::Widget {
    let outer = hbox(0);
    outer.add_css_class("settings-list");
    let list = gtk::ListBox::new();
    let head = gtk::Label::builder().label("Theme").xalign(0.0).margin_start(8).margin_top(6).margin_bottom(4).build();
    head.add_css_class("caption-heading");
    let left = vbox(0);
    left.set_width_request(190);
    left.append(&head);
    left.append(&gtk::ScrolledWindow::builder().child(&list).min_content_height(240).hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).build());
    let bar = hbox(0);
    bar.add_css_class("settings-list-bar");
    let add = icon_button("list-add-symbolic", "Add new item");
    let remove = icon_button("list-remove-symbolic", "Delete selected items");
    let actions = gtk::MenuButton::builder().icon_name("view-more-horizontal-symbolic").tooltip_text("Actions").build();
    actions.add_css_class("flat");
    let sp = hbox(0);
    sp.set_hexpand(true);
    bar.append(&add);
    bar.append(&remove);
    bar.append(&sp);
    bar.append(&actions);
    left.append(&bar);
    outer.append(&left);
    outer.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let editor = vbox(0);
    editor.set_hexpand(true);
    outer.append(&editor);

    let names = Rc::new(RefCell::new(Vec::<String>::new()));
    let refill: Rc<dyn Fn(&str)> = {
        let (list, names) = (list.clone(), names.clone());
        Rc::new(move |select: &str| {
            clear(&list);
            *names.borrow_mut() = theme::all_names();
            for n in names.borrow().iter() {
                list.append(&gtk::Label::builder().label(n).xalign(0.0).margin_start(8).margin_top(3).margin_bottom(3).build());
            }
            if let Some(i) = names.borrow().iter().position(|n| n == select) {
                list.select_row(list.row_at_index(i as i32).as_ref());
            }
        })
    };
    {
        let (names, editor, remove, actions, win) = (names.clone(), editor.clone(), remove.clone(), actions.clone(), win.clone());
        let refill2 = refill.clone();
        list.connect_row_selected(move |_, row| {
            let Some(name) = row.and_then(|r| names.borrow().get(r.index() as usize).cloned()) else { return };
            if settings::get().theme != name {
                choose_theme(&name);
            }
            remove.set_sensitive(!theme::is_bundled(&name));
            fill_theme_editor(&editor, &name);
            actions.set_menu_model(Some(&theme_actions_menu(&name)));
            install_theme_actions(&win, &name, refill2.clone());
        });
    }
    {
        let refill = refill.clone();
        add.connect_clicked(move |_| {
            let name = unique_theme_name("Untitled");
            theme::save_user(&name, &untitled_theme().to_string());
            refill(&name);
        });
    }
    {
        let (names, list, win, refill) = (names.clone(), list.clone(), win.clone(), refill.clone());
        remove.connect_clicked(move |_| {
            let Some(name) = list.selected_row().and_then(|r| names.borrow().get(r.index() as usize).cloned()) else { return };
            let refill = refill.clone();
            confirm_delete(&win, &name.clone(), move || {
                theme::delete_user(&name);
                refill(theme::DEFAULT_LIGHT);
            });
        });
    }
    refill(&settings::get().theme);
    outer.upcast()
}

fn confirm_delete(win: &adw::Window, name: &str, on: impl Fn() + 'static) {
    let dlg = adw::AlertDialog::new(Some(&format!("Are you sure you want to delete “{name}”?")), Some("This action cannot be undone."));
    dlg.add_responses(&[("cancel", "Cancel"), ("delete", "Delete")]);
    dlg.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    dlg.connect_response(Some("delete"), move |_, _| on());
    dlg.present(Some(win));
}

fn theme_actions_menu(name: &str) -> gio::Menu {
    let m = gio::Menu::new();
    let s1 = gio::Menu::new();
    s1.append(Some(&format!("Duplicate “{name}”")), Some("theme.duplicate"));
    s1.append(Some(&format!("Rename “{name}”")), Some("theme.rename"));
    s1.append(Some(&format!("Restore “{name}”")), Some("theme.restore"));
    s1.append(Some(&format!("Export “{name}”…")), Some("theme.export"));
    m.append_section(None, &s1);
    let s2 = gio::Menu::new();
    s2.append(Some("Import…"), Some("theme.import"));
    m.append_section(None, &s2);
    m
}

fn install_theme_actions(win: &adw::Window, name: &str, refill: Rc<dyn Fn(&str)>) {
    let group = gio::SimpleActionGroup::new();
    let add = |id: &str, enabled: bool, f: Box<dyn Fn()>| {
        let a = gio::SimpleAction::new(id, None);
        a.set_enabled(enabled);
        a.connect_activate(move |_, _| f());
        group.add_action(&a);
    };
    let (n, r) = (name.to_string(), refill.clone());
    add(
        "duplicate",
        true,
        Box::new(move || {
            let new = unique_theme_name(&n);
            if let Some(j) = theme::json_for(&n) {
                theme::save_user(&new, &j);
                r(&new);
            }
        }),
    );
    let (n, r, w) = (name.to_string(), refill.clone(), win.clone());
    add(
        "rename",
        !theme::is_bundled(name),
        Box::new(move || {
            let entry = gtk::Entry::builder().text(&n).activates_default(true).build();
            let dlg = adw::AlertDialog::new(Some("Rename Theme"), None);
            dlg.set_extra_child(Some(&entry));
            dlg.add_responses(&[("cancel", "Cancel"), ("ok", "Rename")]);
            dlg.set_default_response(Some("ok"));
            let (n, r, e) = (n.clone(), r.clone(), entry.clone());
            dlg.connect_response(Some("ok"), move |_, _| {
                let new = e.text().trim().to_string();
                if new.is_empty() || theme::all_names().contains(&new) {
                    return;
                }
                if let Some(j) = theme::json_for(&n) {
                    theme::save_user(&new, &j);
                    theme::delete_user(&n);
                    choose_theme(&new);
                    r(&new);
                }
            });
            dlg.present(Some(&w));
        }),
    );
    let (n, r) = (name.to_string(), refill.clone());
    add(
        "restore",
        theme::is_bundled(name) && theme::is_customized(name),
        Box::new(move || {
            theme::delete_user(&n);
            crate::window::settings_changed();
            r(&n);
        }),
    );
    let (n, w) = (name.to_string(), win.clone());
    add(
        "export",
        true,
        Box::new(move || {
            let dlg = gtk::FileDialog::builder().initial_name(format!("{n}.cottheme")).build();
            let n = n.clone();
            dlg.save(Some(&w), gio::Cancellable::NONE, move |res| {
                if let (Ok(f), Some(j)) = (res, theme::json_for(&n)) {
                    if let Some(p) = f.path() {
                        let _ = std::fs::write(p, j);
                    }
                }
            });
        }),
    );
    let (r, w) = (refill.clone(), win.clone());
    add(
        "import",
        true,
        Box::new(move || {
            let r = r.clone();
            gtk::FileDialog::new().open_multiple(Some(&w), gio::Cancellable::NONE, move |res| {
                let Ok(files) = res else { return };
                let mut last = String::new();
                for f in files.iter::<gio::File>().flatten() {
                    let Some(p) = f.path() else { continue };
                    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    if let Ok(j) = std::fs::read_to_string(&p) {
                        if serde_json::from_str::<Value>(&j).is_ok() {
                            theme::save_user(&stem, &j);
                            last = stem;
                        }
                    }
                }
                if !last.is_empty() {
                    r(&last);
                }
            });
        }),
    );
    win.insert_action_group("theme", Some(&group));
}

/// Colour wells for the selected theme; a change is saved at once (a bundled theme becomes a customized copy).
fn fill_theme_editor(editor: &gtk::Box, name: &str) {
    while let Some(c) = editor.first_child() {
        editor.remove(&c);
    }
    let Some(t) = theme::json_for(name).and_then(|j| serde_json::from_str::<Value>(&j).ok()) else { return };
    let t = Rc::new(RefCell::new(t));
    let grid = gtk::Grid::builder().column_spacing(8).row_spacing(6).margin_top(12).margin_start(12).margin_end(12).margin_bottom(12).build();
    let save: Rc<dyn Fn()> = {
        let (t, name) = (t.clone(), name.to_string());
        Rc::new(move || {
            theme::save_user(&name, &serde_json::to_string_pretty(&*t.borrow()).unwrap_or_default());
            crate::window::settings_changed();
        })
    };
    let well = |key: &'static str, label: &str, system: bool, col: i32, row_n: i32| {
        let l = gtk::Label::builder().label(label).xalign(1.0).build();
        l.add_css_class("caption");
        let dlg = gtk::ColorDialog::new();
        dlg.set_with_alpha(!matches!(key, "text" | "background" | "insertionPoint"));
        let b = gtk::ColorDialogButton::new(Some(dlg));
        let c = t.borrow()[key]["color"].as_str().and_then(|c| gdk::RGBA::parse(c).ok()).unwrap_or(gdk::RGBA::BLACK);
        b.set_rgba(&c);
        let cell = vbox(2);
        cell.append(&b);
        if system {
            let uses = t.borrow()[key]["usesSystemSetting"].as_bool().unwrap_or(false);
            let sys = gtk::CheckButton::builder().label("Use system color").active(uses).build();
            sys.add_css_class("caption");
            b.set_sensitive(!uses);
            let (t, save, b2) = (t.clone(), save.clone(), b.clone());
            sys.connect_toggled(move |s| {
                t.borrow_mut()[key]["usesSystemSetting"] = json!(s.is_active());
                b2.set_sensitive(!s.is_active());
                save();
            });
            cell.append(&sys);
        }
        let (t, save) = (t.clone(), save.clone());
        b.connect_rgba_notify(move |b| {
            t.borrow_mut()[key]["color"] = json!(hex(&b.rgba()));
            save();
        });
        grid.attach(&l, col * 2, row_n, 1, 1);
        grid.attach(&cell, col * 2 + 1, row_n, 1, 1);
    };
    for (i, (k, l, sys)) in COLOR_KEYS.iter().enumerate() {
        let (col, r) = if i < 3 { (0, i as i32) } else { (1, i as i32 - 3) };
        well(k, l, *sys, col, r);
    }
    let syntax = gtk::Label::builder().label("Syntax").xalign(0.0).margin_top(8).build();
    syntax.add_css_class("heading");
    grid.attach(&syntax, 0, 5, 4, 1);
    for (i, (k, l)) in SYNTAX_KEYS.iter().enumerate() {
        let (col, r) = if i < 5 { (0, 6 + i as i32) } else { (1, 6 + i as i32 - 5) };
        well(k, l, false, col, r);
    }
    editor.append(&grid);
}

// ---------------------------------------------------------------- 3. Window

fn enabled_by(check: &gtk::CheckButton, dependent: &impl IsA<gtk::Widget>) {
    dependent.set_sensitive(check.is_active());
    let d = dependent.clone().upcast::<gtk::Widget>();
    check.connect_toggled(move |c| d.set_sensitive(c.is_active()));
}

fn window_pane(_: &adw::Window) -> gtk::Widget {
    let g = form();
    g.set_row_spacing(8);
    let size = hbox(8);
    let sizes: [(&str, fn(&Settings) -> u32, fn(&mut Settings, u32)); 2] =
        [("Width", |s| s.window_width, |s, v| s.window_width = v), ("Height", |s| s.window_height, |s, v| s.window_height = v)];
    for (cap, get, put) in sizes {
        let e = gtk::Entry::builder().placeholder_text("Auto").width_chars(6).input_purpose(gtk::InputPurpose::Digits).build();
        let v = get(&settings::get());
        if v > 0 {
            e.set_text(&v.to_string());
        }
        e.connect_changed(move |e| {
            let v = e.text().trim().parse().unwrap_or(0);
            settings::update(|s| put(s, v));
        });
        let col = vbox(2);
        let r = hbox(4);
        r.append(&e);
        r.append(&gtk::Label::new(Some("px")));
        col.append(&r);
        let c = gtk::Label::new(Some(cap));
        c.add_css_class("caption");
        col.append(&c);
        size.append(&col);
    }
    row(&g, "Window size:", &size);
    divider(&g);
    let show = vbox(6);
    show.append(&check("Line numbers", |s| s.show_line_numbers, |s, v| s.show_line_numbers = v));
    show.append(&check("Invisible characters", |s| s.show_invisibles, |s, v| s.show_invisibles = v));
    let kinds = gtk::Grid::builder().column_spacing(14).row_spacing(4).margin_start(20).build();
    kinds.attach(&check("Line ending", |s| s.invisible_newline, |s, v| s.invisible_newline = v), 0, 0, 1, 1);
    kinds.attach(&check("Tab", |s| s.invisible_tab, |s, v| s.invisible_tab = v), 1, 0, 1, 1);
    kinds.attach(&check("Space", |s| s.invisible_space, |s, v| s.invisible_space = v), 2, 0, 1, 1);
    kinds.attach(&check("Other whitespace", |s| s.invisible_other_whitespace, |s, v| s.invisible_other_whitespace = v), 0, 1, 1, 1);
    kinds.attach(&check("Other control characters", |s| s.invisible_control, |s, v| s.invisible_control = v), 1, 1, 2, 1);
    show.append(&kinds);
    show.append(&check("Indent guides", |s| s.show_indent_guides, |s, v| s.show_indent_guides = v));
    let pg = hbox(6);
    let pg_check = check("Page guide at column:", |s| s.show_page_guide, |s, v| s.show_page_guide = v);
    let pg_spin = spin(1.0, 999.0, 1.0, 0, |s| s.page_guide_column as f64, |s, v| s.page_guide_column = v as u32);
    enabled_by(&pg_check, &pg_spin);
    pg.append(&pg_check);
    pg.append(&pg_spin);
    show.append(&pg);
    row(&g, "Show:", &show);
    row(&g, "Current line:", &check("Change background color", |s| s.highlight_current_line, |s, v| s.highlight_current_line = v));
    divider(&g);
    let wrap = vbox(6);
    wrap.append(&check("Wrap lines to editor width", |s| s.wrap_lines, |s, v| s.wrap_lines = v));
    let hang = hbox(6);
    let hang_check = check("Indent wrapped lines by", |s| s.hanging_indent, |s, v| s.hanging_indent = v);
    let hang_spin = spin(0.0, 99.0, 1.0, 0, |s| s.hanging_indent_width as f64, |s, v| s.hanging_indent_width = v as u32);
    enabled_by(&hang_check, &hang_spin);
    hang.append(&hang_check);
    hang.append(&hang_spin);
    hang.append(&gtk::Label::new(Some("spaces")));
    wrap.append(&hang);
    row(&g, "Line wrapping:", &wrap);
    let dir = radios(&[("ltr", "Left to right"), ("rtl", "Right to left"), ("vertical", "Vertical")], |s| s.writing_direction.clone(), |s, v| s.writing_direction = v.into(), false);
    if let Some(vertical) = dir.last_child() {
        vertical.set_sensitive(false);
        vertical.set_tooltip_text(Some("Vertical text isn’t available in GTK."));
    }
    row(&g, "Writing direction:", &dir);
    let over = hbox(6);
    over.append(&spin(0.0, 100.0, 10.0, 0, |s| (s.overscroll * 100.0).round(), |s, v| s.overscroll = v / 100.0));
    over.append(&gtk::Label::new(Some("%")));
    row(&g, "Overscroll:", &over);
    divider(&g);
    let status = gtk::Grid::builder().column_spacing(24).row_spacing(4).build();
    status.attach(&check("Line count", |s| s.status_lines, |s, v| s.status_lines = v), 0, 0, 1, 1);
    status.attach(&check("Character count", |s| s.status_chars, |s, v| s.status_chars = v), 0, 1, 1, 1);
    status.attach(&check("Word count", |s| s.status_words, |s, v| s.status_words = v), 0, 2, 1, 1);
    status.attach(&check("Location", |s| s.status_location, |s, v| s.status_location = v), 1, 0, 1, 1);
    status.attach(&check("Current line", |s| s.status_line, |s, v| s.status_line = v), 1, 1, 1, 1);
    status.attach(&check("Current column", |s| s.status_column, |s, v| s.status_column = v), 1, 2, 1, 1);
    row(&g, "Status bar shows:", &status);
    g.upcast()
}

// ---------------------------------------------------------------- 4. Edit

fn edit(_: &adw::Window) -> gtk::Widget {
    let g = form();
    let ind = vbox(6);
    let prefer = hbox(6);
    prefer.append(&gtk::Label::new(Some("Prefer using")));
    prefer.append(&dropdown(&["Spaces", "Tabs"], if settings::get().auto_expand_tab { 0 } else { 1 }, |i| set(|s| s.auto_expand_tab = i == 0)));
    ind.append(&prefer);
    let width = hbox(6);
    width.append(&gtk::Label::new(Some("Indent width:")));
    width.append(&spin(1.0, 99.0, 1.0, 0, |s| s.tab_width as f64, |s, v| s.tab_width = v as u32));
    width.append(&gtk::Label::new(Some("spaces")));
    ind.append(&width);
    ind.append(&check("Detect indent style on document opening", |s| s.detect_indent, |s, v| s.detect_indent = v));
    ind.append(&check("Automatically indent while typing", |s| s.auto_indent, |s, v| s.auto_indent = v));
    ind.append(&check("Indent selection with Tab key", |s| s.indent_with_tab_key, |s, v| s.indent_with_tab_key = v));
    row(&g, "Indentation:", &ind);
    let ws = vbox(6);
    let trim = check("Automatically trim trailing whitespace", |s| s.auto_trim_trailing_whitespace, |s, v| s.auto_trim_trailing_whitespace = v);
    let only = check("Including whitespace-only lines", |s| s.trim_whitespace_only_lines, |s, v| s.trim_whitespace_only_lines = v);
    only.set_margin_start(20);
    enabled_by(&trim, &only);
    ws.append(&trim);
    ws.append(&only);
    row(&g, "Whitespace:", &ws);
    let cm = vbox(6);
    cm.append(&check("Insert comment delimiters after indent", |s| s.comment_after_indent, |s, v| s.comment_after_indent = v));
    cm.append(&check("Add a space to comment delimiters", |s| s.space_after_comment, |s, v| s.space_after_comment = v));
    row(&g, "Comment:", &cm);
    let cp = vbox(6);
    cp.append(&check("Link URLs in document", |s| s.link_urls, |s, v| s.link_urls = v));
    cp.append(&check("Highlight matching braces", |s| s.highlight_braces, |s, v| s.highlight_braces = v));
    let inst = check("Highlight instances of selected text", |s| s.highlight_selection_instance, |s, v| s.highlight_selection_instance = v);
    cp.append(&inst);
    let delay = hbox(6);
    delay.set_margin_start(20);
    for (i, w) in [gtk::Label::new(Some("Delay:")).upcast::<gtk::Widget>(), spin(0.0, 10.0, 0.25, 2, |s| s.instance_delay, |s, v| s.instance_delay = v).upcast(), gtk::Label::new(Some("seconds")).upcast()]
        .into_iter()
        .enumerate()
    {
        if i != 1 {
            w.add_css_class("caption");
        }
        delay.append(&w);
    }
    enabled_by(&inst, &delay);
    cp.append(&delay);
    row(&g, "Content parse:", &cp);
    g.upcast()
}

// ---------------------------------------------------------------- 5. Mode

fn syntax_list() -> Vec<(String, String)> {
    let lm = sourceview5::LanguageManager::default();
    let mut v: Vec<(String, String)> =
        lm.language_ids().iter().filter_map(|id| lm.language(id)).filter(|l| !l.is_hidden()).map(|l| (l.id().to_string(), l.name().to_string())).collect();
    v.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
    v
}

fn list_header(t: &str) -> gtk::ListBoxRow {
    let l = gtk::Label::builder().label(t).xalign(0.0).margin_start(6).margin_top(6).build();
    l.add_css_class("caption-heading");
    l.add_css_class("dim-label");
    gtk::ListBoxRow::builder().child(&l).selectable(false).activatable(false).build()
}

fn mode(_: &adw::Window) -> gtk::Widget {
    let outer = hbox(12);
    let list = gtk::ListBox::new();
    // the mode key of each list row (None for headers)
    let keys = Rc::new(RefCell::new(Vec::<Option<String>>::new()));
    let bar = hbox(0);
    let add = gtk::MenuButton::builder().icon_name("list-add-symbolic").tooltip_text("Add new item").build();
    add.add_css_class("flat");
    let remove = icon_button("list-remove-symbolic", "Delete selected items");
    bar.append(&add);
    bar.append(&remove);
    let left = list_box_with_bar(&list, &bar, 320);
    left.set_width_request(170);
    outer.append(&left);
    let options = vbox(0);
    options.set_hexpand(true);
    outer.append(&options);

    let refill: Rc<dyn Fn(&str)> = {
        let (list, keys) = (list.clone(), keys.clone());
        Rc::new(move |select: &str| {
            clear(&list);
            let s = settings::get();
            let lm = sourceview5::LanguageManager::default();
            let mut k: Vec<Option<String>> = vec![None];
            list.append(&list_header("Mode"));
            for n in ["General", "Code"] {
                list.append(&gtk::Label::builder().label(n).xalign(0.0).margin_start(10).build());
                k.push(Some(n.to_string()));
            }
            let syntax: Vec<&String> = s.modes.keys().filter(|k| *k != "General" && *k != "Code").collect();
            if !syntax.is_empty() {
                list.append(&list_header("Syntax"));
                k.push(None);
                for id in syntax {
                    let name = lm.language(id).map(|l| l.name().to_string()).unwrap_or_else(|| id.clone());
                    let r = gtk::Label::builder().label(name).xalign(0.0).margin_start(10).build();
                    if lm.language(id).is_none() {
                        r.set_tooltip_text(Some("This syntax does not exist"));
                    }
                    list.append(&r);
                    k.push(Some(id.clone()));
                }
            }
            let index = k.iter().position(|x| x.as_deref() == Some(select)).unwrap_or(1);
            *keys.borrow_mut() = k;
            list.select_row(list.row_at_index(index as i32).as_ref());
        })
    };
    // + : every syntax; one that already has a mode just gets selected
    {
        let menu = gio::Menu::new();
        let sec = gio::Menu::new();
        for (id, name) in syntax_list() {
            let it = gio::MenuItem::new(Some(&name), None);
            it.set_action_and_target_value(Some("mode.add"), Some(&id.to_variant()));
            sec.append_item(&it);
        }
        menu.append_section(Some("Syntax"), &sec);
        add.set_menu_model(Some(&menu));
        let group = gio::SimpleActionGroup::new();
        let a = gio::SimpleAction::new("add", Some(glib::VariantTy::STRING));
        let refill = refill.clone();
        a.connect_activate(move |_, v| {
            let Some(id) = v.and_then(|v| v.get::<String>()) else { return };
            let initial = settings::mode_for(&id);
            let id2 = id.clone();
            set(move |s| {
                s.modes.entry(id2).or_insert(initial);
            });
            refill(&id);
        });
        group.add_action(&a);
        outer.insert_action_group("mode", Some(&group));
    }
    {
        let (list, keys, refill) = (list.clone(), keys.clone(), refill.clone());
        remove.connect_clicked(move |_| {
            let Some(row) = list.selected_row() else { return };
            let Some(k) = keys.borrow().get(row.index() as usize).cloned().flatten() else { return };
            if k == "General" || k == "Code" {
                return;
            }
            set(|s| {
                s.modes.remove(&k);
            });
            refill("General");
        });
    }
    {
        let (keys, options, remove) = (keys.clone(), options.clone(), remove.clone());
        list.connect_row_selected(move |_, row| {
            let Some(key) = row.and_then(|r| keys.borrow().get(r.index() as usize).cloned().flatten()) else { return };
            remove.set_sensitive(key != "General" && key != "Code");
            fill_mode_options(&options, &key);
        });
    }
    refill("General");
    outer.upcast()
}

fn fill_mode_options(options: &gtk::Box, key: &str) {
    while let Some(c) = options.first_child() {
        options.remove(&c);
    }
    let factory = |k: &str| if k == "Code" { Mode::code() } else { Mode::general() };
    let current = settings::get().modes.get(key).cloned().unwrap_or_else(|| factory(key));
    let update: Rc<dyn Fn(&dyn Fn(&mut Mode))> = {
        let key = key.to_string();
        Rc::new(move |f: &dyn Fn(&mut Mode)| {
            let mut m = settings::get().modes.get(&key).cloned().unwrap_or_else(|| factory(&key));
            f(&mut m);
            let key = key.clone();
            set(move |s| {
                s.modes.insert(key, m);
            });
        })
    };
    let mcheck = |label: &str, on: bool, f: fn(&mut Mode, bool)| {
        let c = gtk::CheckButton::builder().label(label).active(on).build();
        let update = update.clone();
        c.connect_toggled(move |c| {
            let v = c.is_active();
            update(&move |m| f(m, v));
        });
        c
    };
    let unavailable = |label: &str| gtk::CheckButton::builder().label(label).sensitive(false).tooltip_text("Not available on Linux").build();
    let g = form();
    g.set_row_spacing(10);
    g.set_margin_top(12);
    g.set_margin_start(12);
    g.set_margin_end(12);
    g.set_margin_bottom(12);
    let font = hbox(14);
    let std_r = gtk::CheckButton::builder().label("Standard").active(!current.monospaced).build();
    let mono_r = gtk::CheckButton::builder().label("Monospaced").active(current.monospaced).group(&std_r).build();
    {
        let update = update.clone();
        mono_r.connect_toggled(move |r| {
            let v = r.is_active();
            update(&move |m| m.monospaced = v);
        });
    }
    font.append(&std_r);
    font.append(&mono_r);
    row(&g, "Font:", &font);
    let sub = vbox(4);
    sub.append(&unavailable("Smart copy/paste"));
    sub.append(&mcheck("Smart quotes", current.smart_quotes, |m, v| m.smart_quotes = v));
    sub.append(&mcheck("Smart dashes", current.smart_dashes, |m, v| m.smart_dashes = v));
    sub.append(&unavailable("Text replacement"));
    sub.append(&unavailable("Add period with double-space"));
    sub.append(&mcheck("Automatically insert closing brackets and quotes", current.auto_close, |m, v| m.auto_close = v));
    row(&g, "Substitution:", &sub);
    let spell = vbox(4);
    for l in ["Check spelling while typing", "Check grammar with spelling", "Correct spelling automatically"] {
        spell.append(&unavailable(l));
    }
    row(&g, "Spelling:", &spell);
    let comp = vbox(4);
    comp.append(&gtk::Label::builder().label("Completion list includes:").xalign(0.0).build());
    let srcs = vbox(4);
    srcs.set_margin_start(20);
    srcs.append(&unavailable("Standard words"));
    srcs.append(&mcheck("Words in document", current.complete_document, |m, v| m.complete_document = v));
    srcs.append(&mcheck("Words defined in syntax", current.complete_syntax, |m, v| m.complete_syntax = v));
    comp.append(&srcs);
    comp.append(&mcheck("Suggest completions while typing", current.suggest, |m, v| m.suggest = v));
    row(&g, "Completion:", &comp);
    let ind = vbox(4);
    let custom = mcheck("Use custom settings", current.custom_indent, |m, v| m.custom_indent = v);
    ind.append(&custom);
    let detail = vbox(4);
    detail.set_margin_start(20);
    let prefer = hbox(6);
    prefer.append(&gtk::Label::new(Some("Prefer using")));
    {
        let update = update.clone();
        prefer.append(&dropdown(&["Spaces", "Tabs"], if current.soft_tabs { 0 } else { 1 }, move |i| update(&move |m| m.soft_tabs = i == 0)));
    }
    detail.append(&prefer);
    let width = hbox(6);
    width.append(&gtk::Label::new(Some("Indent width:")));
    let ws = gtk::SpinButton::with_range(1.0, 99.0, 1.0);
    ws.set_value(current.tab_width as f64);
    {
        let update = update.clone();
        ws.connect_value_changed(move |s| {
            let v = s.value() as u32;
            update(&move |m| m.tab_width = v);
        });
    }
    width.append(&ws);
    width.append(&gtk::Label::new(Some("spaces")));
    detail.append(&width);
    enabled_by(&custom, &detail);
    ind.append(&detail);
    row(&g, "Indentation:", &ind);
    let frame = gtk::Frame::new(None);
    frame.set_child(Some(&g));
    options.append(&frame);
}

// ---------------------------------------------------------------- 6. Format

fn format(win: &adw::Window) -> gtk::Widget {
    let g = form();
    let s = settings::get();
    let endings = ["LF", "CR", "CRLF"];
    row(
        &g,
        "Default line endings:",
        &dropdown(&["macOS / Unix (LF)", "Classic Mac OS (CR)", "Windows (CRLF)"], endings.iter().position(|e| *e == s.line_ending).unwrap_or(0), move |i| {
            set(|s| s.line_ending = endings[i].into())
        }),
    );
    divider(&g);
    // the default encoding: the list order, UTF-8 twice (without and with BOM)
    let mut encs: Vec<(String, bool, String)> = Vec::new();
    for iana in s.encodings.iter().filter(|e| *e != "-") {
        encs.push((iana.clone(), false, crate::document::Encoding { iana: iana.clone(), bom: false }.label()));
        if iana == "UTF-8" {
            encs.push((iana.clone(), true, crate::document::Encoding { iana: iana.clone(), bom: true }.label()));
        }
    }
    let labels: Vec<&str> = encs.iter().map(|e| e.2.as_str()).collect();
    let current = encs.iter().position(|e| e.0 == s.encoding && e.1 == s.encoding_bom).unwrap_or(0);
    let pick = encs.clone();
    row(
        &g,
        "Default text encoding:",
        &dropdown(&labels, current, move |i| {
            if let Some((iana, bom, _)) = pick.get(i).cloned() {
                set(move |s| {
                    s.encoding = iana;
                    s.encoding_bom = bom;
                });
            }
        }),
    );
    let pri = vbox(6);
    let edit_list = gtk::Button::builder().label("Edit List…").halign(gtk::Align::Start).build();
    let w = win.clone();
    edit_list.connect_clicked(move |_| encoding_list_sheet(&w));
    pri.append(&edit_list);
    pri.append(&check("Refer to encoding declaration in document", |s| s.refer_to_declaration, |s, v| s.refer_to_declaration = v));
    row(&g, "Encoding priorities:", &pri);
    divider(&g);
    let syntaxes = syntax_list();
    let mut names = vec!["None".to_string()];
    names.extend(syntaxes.iter().map(|(_, n)| n.clone()));
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let current = syntaxes.iter().position(|(id, _)| *id == s.default_syntax).map_or(0, |i| i + 1);
    let ids: Vec<String> = syntaxes.iter().map(|(id, _)| id.clone()).collect();
    row(
        &g,
        "Default syntax:",
        &dropdown(&refs, current, move |i| {
            let id = if i == 0 { String::new() } else { ids[i - 1].clone() };
            settings::update(|s| s.default_syntax = id);
        }),
    );
    let list = gtk::ListBox::new();
    for (_, n) in &syntaxes {
        list.append(&gtk::Label::builder().label(n).xalign(0.0).margin_start(8).margin_top(2).margin_bottom(2).build());
    }
    let bar = hbox(0);
    let note = gtk::Label::builder().label("Syntax definitions come from GtkSourceView and Proza; editing them isn’t available yet.").xalign(0.0).margin_start(6).wrap(true).build();
    note.add_css_class("caption");
    note.add_css_class("dim-label");
    bar.append(&note);
    row(&g, "Available syntaxes:", &list_box_with_bar(&list, &bar, 180));
    g.upcast()
}

/// The Encoding List sheet: the detection order, reorderable by dragging; + adds a separator or an encoding.
fn encoding_list_sheet(win: &adw::Window) {
    let items = Rc::new(RefCell::new(settings::get().encodings));
    let list = gtk::ListBox::new();
    let refill_cell: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
    let refill: Rc<dyn Fn()> = {
        let (list, items, cell) = (list.clone(), items.clone(), refill_cell.clone());
        Rc::new(move || {
            clear(&list);
            for (i, iana) in items.borrow().iter().enumerate() {
                let row = gtk::ListBoxRow::new();
                if iana == "-" {
                    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
                    sep.set_margin_top(6);
                    sep.set_margin_bottom(6);
                    row.set_child(Some(&sep));
                } else {
                    let b = hbox(8);
                    b.set_margin_start(8);
                    b.append(&gtk::Label::new(Some(&crate::document::Encoding { iana: iana.clone(), bom: false }.label())));
                    let cs = gtk::Label::new(Some(&iana.to_lowercase()));
                    cs.add_css_class("dim-label");
                    cs.add_css_class("caption");
                    b.append(&cs);
                    row.set_child(Some(&b));
                }
                // drag a row onto another to move it there
                let src = gtk::DragSource::new();
                src.set_actions(gdk::DragAction::MOVE);
                src.connect_prepare(move |_, _, _| Some(gdk::ContentProvider::for_value(&(i as u32).to_value())));
                row.add_controller(src);
                let dst = gtk::DropTarget::new(u32::static_type(), gdk::DragAction::MOVE);
                let (items, cell) = (items.clone(), cell.clone());
                dst.connect_drop(move |_, v, _, _| {
                    let Ok(from) = v.get::<u32>() else { return false };
                    {
                        let mut it = items.borrow_mut();
                        if (from as usize) < it.len() && i < it.len() {
                            let x = it.remove(from as usize);
                            it.insert(i, x);
                        }
                    }
                    if let Some(f) = cell.borrow().clone() {
                        glib::idle_add_local_once(move || f());
                    }
                    true
                });
                row.add_controller(dst);
                list.append(&row);
            }
        })
    };
    *refill_cell.borrow_mut() = Some(refill.clone());
    refill();
    let bx = vbox(8);
    bx.set_width_request(420);
    bx.append(&gtk::Label::builder().label("Drag encodings to change the order:").xalign(0.0).build());
    let sc = gtk::ScrolledWindow::builder().child(&list).min_content_height(320).hscrollbar_policy(gtk::PolicyType::Never).build();
    sc.add_css_class("frame");
    bx.append(&sc);
    let bar = hbox(4);
    let add = gtk::MenuButton::builder().icon_name("list-add-symbolic").build();
    let remove = gtk::Button::from_icon_name("list-remove-symbolic");
    bar.append(&add);
    bar.append(&remove);
    bx.append(&bar);
    let foot = gtk::Label::builder()
        .label("This order is used for the Encoding menu and for encoding detection. Only the encodings listed here are considered during detection, with higher items taking priority.")
        .wrap(true)
        .xalign(0.0)
        .max_width_chars(60)
        .build();
    foot.add_css_class("caption");
    bx.append(&foot);
    let menu = gio::Menu::new();
    let sep_item = gio::MenuItem::new(Some("Separator"), None);
    sep_item.set_action_and_target_value(Some("enc.add"), Some(&"-".to_variant()));
    menu.append_item(&sep_item);
    let sec = gio::Menu::new();
    let mut known: Vec<(String, String)> = crate::document::DEFAULT_ENCODINGS
        .iter()
        .flatten()
        .map(|(label, iana)| (label.to_string(), iana.to_string()))
        .filter(|(_, iana)| !items.borrow().contains(iana))
        .collect();
    known.sort();
    for (label, iana) in known {
        let it = gio::MenuItem::new(Some(&label), None);
        it.set_action_and_target_value(Some("enc.add"), Some(&iana.to_variant()));
        sec.append_item(&it);
    }
    menu.append_section(Some("Text Encoding"), &sec);
    add.set_menu_model(Some(&menu));
    let group = gio::SimpleActionGroup::new();
    let a = gio::SimpleAction::new("add", Some(glib::VariantTy::STRING));
    {
        let (items, list, refill) = (items.clone(), list.clone(), refill.clone());
        a.connect_activate(move |_, v| {
            let Some(iana) = v.and_then(|v| v.get::<String>()) else { return };
            let len = items.borrow().len();
            let at = list.selected_row().map(|r| r.index() as usize + 1).unwrap_or(len).min(len);
            items.borrow_mut().insert(at, iana);
            refill();
        });
    }
    group.add_action(&a);
    bx.insert_action_group("enc", Some(&group));
    {
        let (items, list, refill) = (items.clone(), list.clone(), refill.clone());
        remove.connect_clicked(move |b| {
            let Some(r) = list.selected_row() else { return };
            let i = r.index() as usize;
            let it = items.borrow()[i].clone();
            if it == "UTF-8" || it == settings::get().encoding {
                b.error_bell();
                return;
            }
            items.borrow_mut().remove(i);
            refill();
        });
    }
    let dlg = adw::AlertDialog::new(None, None);
    dlg.set_extra_child(Some(&bx));
    dlg.add_responses(&[("restore", "Restore Defaults"), ("cancel", "Cancel"), ("ok", "OK")]);
    dlg.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
    dlg.set_default_response(Some("ok"));
    dlg.set_close_response("cancel");
    let it = items.clone();
    dlg.connect_response(None, move |_, r| match r {
        "ok" => {
            let list = it.borrow().clone();
            set(move |s| s.encodings = list);
        }
        "restore" => {
            let defaults = Settings::default().encodings;
            set(move |s| s.encodings = defaults);
        }
        _ => {}
    });
    dlg.present(Some(win));
}

// ---------------------------------------------------------------- 7. Snippets

fn accel_label(a: &str) -> String {
    gtk::accelerator_parse(a).map(|(k, m)| gtk::accelerator_get_label(k, m).to_string()).unwrap_or_default()
}

/// A shortcut recorder: shows the keys; click, then press the new keys (Backspace clears, Escape cancels).
fn recorder(accel: &str, on: impl Fn(String) + 'static) -> gtk::Button {
    let b = gtk::Button::with_label(&accel_label(accel));
    b.add_css_class("shortcut-recorder");
    let recording = Rc::new(std::cell::Cell::new(false));
    let current = Rc::new(RefCell::new(accel.to_string()));
    let keys = gtk::EventControllerKey::new();
    {
        let (b2, recording, current) = (b.clone(), recording.clone(), current.clone());
        keys.connect_key_pressed(move |_, key, _, state| {
            if !recording.get() {
                return glib::Propagation::Proceed;
            }
            let mods = state & gtk::accelerator_get_default_mod_mask();
            match key {
                gdk::Key::Escape => {
                    recording.set(false);
                    b2.set_label(&accel_label(&current.borrow()));
                }
                gdk::Key::BackSpace | gdk::Key::Delete if mods.is_empty() => {
                    recording.set(false);
                    current.replace(String::new());
                    b2.set_label("");
                    on(String::new());
                }
                k if matches!(
                    k,
                    gdk::Key::Shift_L | gdk::Key::Shift_R | gdk::Key::Control_L | gdk::Key::Control_R | gdk::Key::Alt_L | gdk::Key::Alt_R
                        | gdk::Key::Super_L | gdk::Key::Super_R | gdk::Key::Meta_L | gdk::Key::Meta_R | gdk::Key::ISO_Level3_Shift | gdk::Key::Caps_Lock
                ) => {}
                k => {
                    let free = (gdk::Key::F1..=gdk::Key::F35).contains(&k) || matches!(k, gdk::Key::Home | gdk::Key::End | gdk::Key::Page_Up | gdk::Key::Page_Down);
                    if mods.is_empty() && !free {
                        b2.error_bell();
                        return glib::Propagation::Stop;
                    }
                    let name = gtk::accelerator_name(k.to_lower(), mods).to_string();
                    recording.set(false);
                    b2.set_label(&accel_label(&name));
                    current.replace(name.clone());
                    on(name);
                }
            }
            glib::Propagation::Stop
        });
    }
    b.add_controller(keys);
    b.connect_clicked(move |b| {
        recording.set(true);
        b.set_label("Type shortcut…");
    });
    b
}

const SNIPPET_VARIABLES: [(&str, &str); 2] = [("SELECTION", "The selected text."), ("CURSOR", "The insertion point after inserting the snippet.")];
const DROP_VARIABLES: [(&str, &str); 11] = [
    ("ABSOLUTE-PATH", "The absolute path of the dropped file."),
    ("RELATIVE-PATH", "The relative path between the dropped file and the document."),
    ("FILENAME", "The name of the dropped file with extension."),
    ("FILENAME-NOSUFFIX", "The name of the dropped file without extension."),
    ("FILEEXTENSION", "The extension of the dropped file."),
    ("FILEEXTENSION-LOWER", "The extension of the dropped file in lowercase."),
    ("FILEEXTENSION-UPPER", "The extension of the dropped file in uppercase."),
    ("DIRECTORY", "The parent directory name of the dropped file."),
    ("FILECONTENT", "The content of the dropped file (text files)."),
    ("IMAGEWIDTH", "The pixel width of the dropped image."),
    ("IMAGEHEIGHT", "The pixel height of the dropped image."),
];

fn syntax_dropdown(current: &str, on: impl Fn(String) + 'static) -> gtk::DropDown {
    let syntaxes = syntax_list();
    let mut names = vec!["All".to_string()];
    names.extend(syntaxes.iter().map(|(_, n)| n.clone()));
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let sel = syntaxes.iter().position(|(id, _)| id == current).map_or(0, |i| i + 1);
    let ids: Vec<String> = syntaxes.into_iter().map(|(id, _)| id).collect();
    let d = dropdown(&refs, sel, move |i| on(if i == 0 { String::new() } else { ids[i - 1].clone() }));
    d.set_width_request(150);
    d
}

/// The "Insertion format:" editor with its Insert Variable menu.
fn format_editor(vars: &'static [(&'static str, &'static str)], group_name: &str) -> (gtk::Box, gtk::TextView) {
    let head = hbox(6);
    head.append(&gtk::Label::builder().label("Insertion format:").xalign(0.0).hexpand(true).build());
    let menu = gio::Menu::new();
    for (v, desc) in vars {
        let it = gio::MenuItem::new(Some(&format!("<<<{v}>>>   {desc}")), None);
        it.set_action_and_target_value(Some(&format!("{group_name}.var")), Some(&v.to_variant()));
        menu.append_item(&it);
    }
    head.append(&gtk::MenuButton::builder().label("Insert Variable").menu_model(&menu).build());
    let view = gtk::TextView::builder().monospace(true).top_margin(6).bottom_margin(6).left_margin(6).right_margin(6).sensitive(false).build();
    let sc = gtk::ScrolledWindow::builder().child(&view).min_content_height(90).build();
    sc.add_css_class("frame");
    let bx = vbox(6);
    bx.append(&head);
    bx.append(&sc);
    let group = gio::SimpleActionGroup::new();
    let a = gio::SimpleAction::new("var", Some(glib::VariantTy::STRING));
    let v = view.clone();
    a.connect_activate(move |_, p| {
        if let Some(name) = p.and_then(|p| p.get::<String>()) {
            v.buffer().insert_at_cursor(&format!("<<<{name}>>>"));
            v.grab_focus();
        }
    });
    group.add_action(&a);
    bx.insert_action_group(group_name, Some(&group));
    (bx, view)
}

/// A table of rows (built by `make_row`) with + / −, and a format editor bound to the selected row.
fn rows_with_format(
    intro: &str,
    vars: &'static [(&'static str, &'static str)],
    group: &str,
    count: fn() -> usize,
    make_row: fn(usize) -> gtk::Widget,
    get_format: fn(usize) -> Option<String>,
    put_format: fn(usize, String),
    add_item: fn(),
    remove_item: fn(usize),
    restore: Option<fn()>,
) -> gtk::Box {
    let page = vbox(8);
    page.append(&gtk::Label::builder().label(intro).xalign(0.0).build());
    let list = gtk::ListBox::new();
    let bar = hbox(0);
    let add = icon_button("list-add-symbolic", "Add new item");
    let remove = icon_button("list-remove-symbolic", "Delete selected items");
    bar.append(&add);
    bar.append(&remove);
    let restore_btn = restore.map(|_| {
        let sp = hbox(0);
        sp.set_hexpand(true);
        bar.append(&sp);
        let b = gtk::Button::with_label("Restore Defaults");
        b.add_css_class("flat");
        bar.append(&b);
        b
    });
    page.append(&list_box_with_bar(&list, &bar, 150));
    let (fmt_box, view) = format_editor(vars, group);
    page.append(&fmt_box);
    let selected = Rc::new(std::cell::Cell::new(None::<usize>));
    let refill: Rc<dyn Fn()> = {
        let list = list.clone();
        Rc::new(move || {
            clear(&list);
            for i in 0..count() {
                list.append(&make_row(i));
            }
        })
    };
    refill();
    {
        let (view, selected) = (view.clone(), selected.clone());
        list.connect_row_selected(move |_, row| {
            selected.set(row.map(|r| r.index() as usize));
            let f = row.and_then(|r| get_format(r.index() as usize));
            view.set_sensitive(f.is_some());
            view.buffer().set_text(&f.unwrap_or_default());
        });
    }
    {
        let (v, selected) = (view.clone(), selected.clone());
        view.buffer().connect_changed(move |b| {
            if let (true, Some(i)) = (v.is_sensitive(), selected.get()) {
                put_format(i, b.text(&b.start_iter(), &b.end_iter(), false).to_string());
            }
        });
    }
    {
        let r = refill.clone();
        add.connect_clicked(move |_| {
            add_item();
            r();
        });
        let (r, selected) = (refill.clone(), selected.clone());
        remove.connect_clicked(move |_| {
            if let Some(i) = selected.get() {
                remove_item(i);
                r();
            }
        });
        if let (Some(b), Some(f)) = (restore_btn, restore) {
            let r = refill.clone();
            b.connect_clicked(move |_| {
                f();
                r();
            });
        }
    }
    page
}

fn snippets(_: &adw::Window) -> gtk::Widget {
    let stack = gtk::Stack::new();
    stack.set_vhomogeneous(false);
    let switcher = gtk::StackSwitcher::builder().stack(&stack).halign(gtk::Align::Center).build();
    let command = rows_with_format(
        "Text to be inserted by a command in the menu or by keyboard shortcut:",
        &SNIPPET_VARIABLES,
        "snip",
        || settings::get().snippets.len(),
        |i| {
            let snip = settings::get().snippets[i].clone();
            let r = hbox(8);
            r.append(&syntax_dropdown(&snip.scope, move |scope| set(|s| s.snippets[i].scope = scope)));
            let name = gtk::Entry::builder().text(&snip.name).hexpand(true).build();
            name.connect_changed(move |e| {
                let t = e.text().to_string();
                set(|s| s.snippets[i].name = t)
            });
            r.append(&name);
            r.append(&recorder(&snip.key, move |k| set(|s| s.snippets[i].key = k)));
            r.upcast()
        },
        |i| settings::get().snippets.get(i).map(|s| s.format.clone()),
        |i, f| settings::update(|s| s.snippets[i].format = f),
        || {
            let names: Vec<String> = settings::get().snippets.iter().map(|s| s.name.clone()).collect();
            let name = std::iter::once("Untitled".to_string()).chain((2..).map(|n| format!("Untitled {n}"))).find(|n| !names.contains(n)).unwrap_or_default();
            set(|s| s.snippets.push(settings::Snippet { name, ..Default::default() }));
        },
        |i| set(|s| {
            s.snippets.remove(i);
        }),
        None,
    );
    stack.add_titled(&command, Some("command"), "Command");
    let drop = rows_with_format(
        "Text to be inserted by dropping files to the editor:",
        &DROP_VARIABLES,
        "drop",
        || settings::get().file_drops.len(),
        |i| {
            let rule = settings::get().file_drops[i].clone();
            let r = hbox(8);
            let d = syntax_dropdown(&rule.scope, move |scope| settings::update(|s| s.file_drops[i].scope = scope));
            d.set_tooltip_text(Some("Syntax in which this file drop setting is used."));
            r.append(&d);
            let ext = gtk::Entry::builder().text(&rule.extensions).placeholder_text("All").tooltip_text("File extensions of dropped file (comma-separated).").width_chars(14).build();
            ext.connect_changed(move |e| {
                let t = e.text().to_string();
                settings::update(|s| s.file_drops[i].extensions = t)
            });
            r.append(&ext);
            let desc = gtk::Entry::builder().text(&rule.description).placeholder_text("Description").hexpand(true).build();
            desc.connect_changed(move |e| {
                let t = e.text().to_string();
                settings::update(|s| s.file_drops[i].description = t)
            });
            r.append(&desc);
            r.upcast()
        },
        |i| settings::get().file_drops.get(i).map(|r| r.format.clone()),
        |i, f| settings::update(|s| s.file_drops[i].format = f),
        || settings::update(|s| s.file_drops.push(settings::FileDrop::default())),
        |i| settings::update(|s| {
            s.file_drops.remove(i);
        }),
        Some(|| settings::update(|s| s.file_drops = settings::default_file_drops())),
    );
    stack.add_titled(&drop, Some("drop"), "File Drop");
    let page = vbox(10);
    page.append(&switcher);
    page.append(&stack);
    page.upcast()
}

// ---------------------------------------------------------------- 8. Shortcuts

fn shortcuts(_: &adw::Window) -> gtk::Widget {
    let page = vbox(8);
    page.append(&gtk::Label::builder().label("To change a shortcut, click the key column, then type the new keys.").xalign(0.0).build());
    let tabs = hbox(0);
    tabs.add_css_class("linked");
    let menu_tab = gtk::ToggleButton::builder().label("Menu").active(true).build();
    let text_tab = gtk::ToggleButton::builder().label("Text Editing").group(&menu_tab).sensitive(false).tooltip_text("Text editing keys follow GTK’s bindings").build();
    tabs.append(&menu_tab);
    tabs.append(&text_tab);
    page.append(&tabs);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    let error = gtk::Label::builder().xalign(0.0).build();
    error.add_css_class("error");
    error.add_css_class("caption");
    let refill: Rc<dyn Fn()> = {
        let (list, error) = (list.clone(), error.clone());
        Rc::new(move || {
            clear(&list);
            let custom = settings::get().shortcuts;
            for (menu, items) in crate::menu::customizable() {
                let h = gtk::Label::builder().label(menu).xalign(0.0).margin_top(8).margin_start(4).build();
                h.add_css_class("heading");
                list.append(&gtk::ListBoxRow::builder().child(&h).activatable(false).build());
                for (action, label) in items {
                    let keys = crate::app::accels_for(action);
                    let r = hbox(8);
                    r.set_margin_start(16);
                    let l = gtk::Label::builder().label(label).xalign(0.0).hexpand(true).build();
                    if custom.contains_key(action) {
                        l.add_css_class("heading");
                    }
                    r.append(&l);
                    let (action_s, error) = (action.to_string(), error.clone());
                    r.append(&recorder(keys.first().map(String::as_str).unwrap_or(""), move |k| {
                        if !k.is_empty() {
                            let taken = crate::menu::customizable()
                                .into_iter()
                                .flat_map(|(_, items)| items)
                                .find(|(a, _)| *a != action_s && crate::app::accels_for(a).iter().any(|x| *x == k));
                            if let Some((_, other)) = taken {
                                error.set_label(&format!("“{}” is already taken by the “{other}” command.", accel_label(&k)));
                                return;
                            }
                        }
                        error.set_label("");
                        let a = action_s.clone();
                        settings::update(move |s| {
                            s.shortcuts.insert(a, if k.is_empty() { Vec::new() } else { vec![k] });
                        });
                        crate::app::apply_shortcuts();
                    }));
                    list.append(&r);
                }
            }
        })
    };
    refill();
    let sc = gtk::ScrolledWindow::builder().child(&list).min_content_height(380).hscrollbar_policy(gtk::PolicyType::Never).build();
    sc.add_css_class("frame");
    page.append(&sc);
    let bottom = hbox(8);
    let restore = gtk::Button::with_label("Restore Defaults");
    {
        let refill = refill.clone();
        restore.connect_clicked(move |_| {
            settings::update(|s| s.shortcuts.clear());
            crate::app::apply_shortcuts();
            refill();
        });
    }
    bottom.append(&restore);
    bottom.append(&error);
    page.append(&bottom);
    page.upcast()
}

// ---------------------------------------------------------------- 9. Integration

fn integration(_: &adw::Window) -> gtk::Widget {
    let g = form();
    let cli = vbox(6);
    let top = hbox(8);
    let learn = gtk::Button::with_label("Learn More…");
    learn.connect_clicked(|_| {
        gtk::UriLauncher::new("https://github.com/shatyrko/proza#command-line").launch(None::<&gtk::Window>, gio::Cancellable::NONE, |_| {});
    });
    top.append(&learn);
    let link = glib::home_dir().join(".local/bin/proza");
    if link.exists() {
        let ok = std::fs::canonicalize(&link).ok() == std::env::current_exe().ok().and_then(|e| std::fs::canonicalize(e).ok());
        let dot = gtk::Label::new(Some("●"));
        dot.add_css_class(if ok { "success" } else { "warning" });
        if !ok {
            dot.set_tooltip_text(Some("The current proza command doesn’t target the running Proza."));
        }
        top.append(&dot);
        top.append(&gtk::Label::new(Some(&format!("installed at {}", link.display()))));
    }
    cli.append(&top);
    let t = gtk::Label::builder().label("With the proza command-line tool, you can launch Proza and let it open files from the command line.").wrap(true).xalign(0.0).max_width_chars(60).build();
    t.add_css_class("caption");
    cli.append(&t);
    row(&g, "Command-line tool:", &cli);
    let rm = vbox(6);
    rm.append(&gtk::CheckButton::builder().label("Accept rmate connections").sensitive(false).tooltip_text("Not available yet").build());
    let port = hbox(6);
    port.append(&gtk::Label::new(Some("Port:")));
    let ps = gtk::SpinButton::with_range(1.0, 65535.0, 1.0);
    ps.set_value(settings::get().rmate_port as f64);
    ps.set_sensitive(false);
    port.append(&ps);
    rm.append(&port);
    let note = gtk::Label::builder().label("Allow remote files to be opened in Proza using the rmate protocol.").wrap(true).xalign(0.0).build();
    note.add_css_class("caption");
    rm.append(&note);
    row(&g, "Remote editing:", &rm);
    g.upcast()
}
