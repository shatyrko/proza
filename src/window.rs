// The document window: CotEditor's layout on GTK.
//
//   title bar      [file browser]  folder / file name          [syntax ⌃⌄] [inspector]
//   navigation     ⌃ ⌄  outline item ⌃⌄                                      [split]
//   editor         line numbers | text
//   status bar     Lines: n  Characters: n  Words: n          size | encoding ⌃⌄ | LF ⌃⌄

use adw::prelude::*;
use gtk::{gio, glib};
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Window;

struct State {
    file: RefCell<Option<gio::File>>,
}

impl Window {
    pub fn new(app: &adw::Application, file: Option<gio::File>) -> adw::ApplicationWindow {
        let win = adw::ApplicationWindow::builder()
            .application(app)
            .default_width(720)
            .default_height(640)
            .build();

        // ---- title bar
        let header = adw::HeaderBar::new();
        let browser_btn = gtk::ToggleButton::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text("Show File Browser")
            .build();
        header.pack_start(&browser_btn);
        let title = adw::WindowTitle::new("Untitled", "");
        header.set_title_widget(Some(&title));
        let inspector_btn = gtk::ToggleButton::builder()
            .icon_name("info-outline-symbolic")
            .tooltip_text("Show Inspector")
            .build();
        header.pack_end(&inspector_btn);
        let lm = sourceview5::LanguageManager::default();
        let mut names: Vec<(String, String)> = lm
            .language_ids()
            .iter()
            .filter_map(|id| lm.language(id))
            .filter(|l| !l.is_hidden())
            .map(|l| (l.id().to_string(), l.name().to_string()))
            .collect();
        names.sort_by_key(|(_, n)| n.to_lowercase());
        names.insert(0, (String::new(), "Plain Text".into()));
        let labels: Vec<&str> = names.iter().map(|(_, n)| n.as_str()).collect();
        let syntax = gtk::DropDown::from_strings(&labels);
        syntax.set_enable_search(true);
        syntax.set_tooltip_text(Some("Syntax"));
        syntax.add_css_class("syntax-popup");
        header.pack_end(&syntax);

        // ---- navigation bar
        let nav = gtk::CenterBox::new();
        nav.add_css_class("navigation-bar");
        let nav_start = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        for (icon, tip) in [("go-up-symbolic", "Previous Outline Item"), ("go-down-symbolic", "Next Outline Item")] {
            let b = gtk::Button::builder().icon_name(icon).tooltip_text(tip).build();
            b.add_css_class("flat");
            nav_start.append(&b);
        }
        let outline = gtk::MenuButton::builder().label("").build();
        outline.add_css_class("flat");
        outline.add_css_class("outline-popup");
        nav_start.append(&outline);
        nav.set_start_widget(Some(&nav_start));
        let split_btn = gtk::Button::builder()
            .icon_name("view-dual-symbolic")
            .tooltip_text("Split Editor")
            .build();
        split_btn.add_css_class("flat");
        nav.set_end_widget(Some(&split_btn));

        // ---- editor
        let buffer = sourceview5::Buffer::new(None);
        buffer.set_highlight_matching_brackets(true);
        let view = sourceview5::View::with_buffer(&buffer);
        view.set_show_line_numbers(true);
        view.set_highlight_current_line(true);
        view.set_monospace(true);
        view.set_wrap_mode(gtk::WrapMode::WordChar);
        view.set_auto_indent(true);
        view.set_tab_width(4);
        view.set_top_margin(6);
        view.set_left_margin(6);
        let drawer = view.space_drawer();
        drawer.set_types_for_locations(
            sourceview5::SpaceLocationFlags::ALL,
            sourceview5::SpaceTypeFlags::NEWLINE | sourceview5::SpaceTypeFlags::TAB,
        );
        drawer.set_enable_matrix(true);
        // the colour scheme follows the system's light / dark style
        let sm = adw::StyleManager::default();
        let scheme_for = |dark: bool| {
            sourceview5::StyleSchemeManager::default().scheme(if dark { "Adwaita-dark" } else { "Adwaita" })
        };
        buffer.set_style_scheme(scheme_for(sm.is_dark()).as_ref());
        {
            let buffer = buffer.clone();
            sm.connect_dark_notify(move |sm| buffer.set_style_scheme(scheme_for(sm.is_dark()).as_ref()));
        }
        let scroller = gtk::ScrolledWindow::builder().child(&view).vexpand(true).build();

        // ---- status bar
        let status = gtk::CenterBox::new();
        status.add_css_class("status-bar");
        let counts = gtk::Label::new(None);
        status.set_start_widget(Some(&counts));
        let status_end = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let size = gtk::Label::new(Some(""));
        let encoding = gtk::MenuButton::builder().label("Unicode (UTF-8)").build();
        let newline = gtk::MenuButton::builder().label("LF").build();
        for w in [encoding.upcast_ref::<gtk::Widget>(), newline.upcast_ref()] {
            w.add_css_class("flat");
        }
        status_end.append(&size);
        status_end.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        status_end.append(&encoding);
        status_end.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        status_end.append(&newline);
        status.set_end_widget(Some(&status_end));

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&nav);
        content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        content.append(&scroller);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&content));
        toolbar.add_bottom_bar(&status);
        win.set_content(Some(&toolbar));

        let state = Rc::new(State { file: RefCell::new(None) });

        // counts in the status bar
        let update = {
            let counts = counts.clone();
            let size = size.clone();
            move |b: &sourceview5::Buffer| {
                let text = b.text(&b.start_iter(), &b.end_iter(), false);
                let lines = b.line_count();
                let chars = text.chars().count();
                let words = text.split_whitespace().count();
                counts.set_markup(&format!(
                    "<span alpha='60%'>Lines:</span> {lines}   <span alpha='60%'>Characters:</span> {chars}   \
                     <span alpha='60%'>Words:</span> {words}"
                ));
                size.set_label(&human_size(text.len()));
            }
        };
        update(&buffer);
        buffer.connect_changed(move |b| update(b));

        if let Some(f) = file {
            load(&win, &buffer, &title, &syntax, &names, &f);
            state.file.replace(Some(f));
        }
        win
    }
}

fn load(
    _win: &adw::ApplicationWindow,
    buffer: &sourceview5::Buffer,
    title: &adw::WindowTitle,
    syntax: &gtk::DropDown,
    names: &[(String, String)],
    file: &gio::File,
) {
    let src = sourceview5::File::builder().location(file).build();
    let loader = sourceview5::FileLoader::new(buffer, &src);
    let name = file.basename().map(|p| p.display().to_string()).unwrap_or_default();
    title.set_title(&name);
    // the callback runs on the main thread; the bindings still ask for Send
    let guard = glib::thread_guard::ThreadGuard::new((buffer.clone(), syntax.clone(), names.to_vec()));
    loader.load_async(glib::Priority::DEFAULT, gio::Cancellable::NONE, move |res| {
        let (buffer, syntax, names) = guard.get_ref();
        if res.is_ok() {
            let lm = sourceview5::LanguageManager::default();
            if let Some(lang) = lm.guess_language(Some(name.as_str()), None) {
                buffer.set_language(Some(&lang));
                if let Some(i) = names.iter().position(|(id, _)| id == lang.id().as_str()) {
                    syntax.set_selected(i as u32);
                }
            }
            buffer.place_cursor(&buffer.start_iter());
        }
    });
}

fn human_size(bytes: usize) -> String {
    if bytes < 1000 {
        format!("{bytes} bytes")
    } else if bytes < 1_000_000 {
        format!("{} kB", (bytes as f64 / 1000.0).round())
    } else {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    }
}
