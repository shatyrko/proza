// The document window: CotEditor's layout and behaviour on GTK (see docs/coteditor-ui-spec.md).
//
//   title bar      [menu]  file name                              [Syntax ⌃⌄]  [ⓘ]
//   navigation     ✕ | ⌃ ⌄  outline item ⌃⌄                                      [split]
//   editor         line numbers | text                                   (inspector on the right)
//   status bar     Lines: n  Characters: n  Location: n  Line: n    size | encoding ⌃⌄ | LF ⌃⌄

use crate::document::{self, Encoding, LineEnding};
use crate::settings;
use adw::prelude::*;
use gtk::{gdk, gio, glib, pango};
use sourceview5::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use unicode_segmentation::UnicodeSegmentation;

/// General (prose) syntaxes use the standard font, everything else the monospaced one (CotEditor's syntax kinds).
const GENERAL_SYNTAXES: &[&str] = &["", "markdown", "latex", "bibtex", "rst", "textile", "bbcode"];
const AUTOSAVE_DELAY: u32 = 5;

/// Per-document state.
pub struct DocState {
    pub file: Option<gio::File>,
    pub encoding: Encoding,
    pub ending: LineEnding,
    pub syntax: String,
    pub read_only: bool,
    /// bytes last written or read, to tell our own saves from someone else's change
    pub disk_bytes: Option<Vec<u8>>,
    pub backup_id: String,
}

/// Per-window display state (initialised from Settings, then changed from the Format/View menus).
pub struct ViewState {
    pub mono: bool,
    pub zoom: f64,
    pub wrap: bool,
    pub invisibles: bool,
    pub line_numbers: bool,
    pub page_guide: bool,
    pub tab_width: u32,
    pub soft_tabs: bool,
    pub theme: String,
}

pub struct EditorPane {
    pub root: gtk::Box,
    pub view: sourceview5::View,
    pub nav: gtk::Box,
    pub close_btn: gtk::Button,
    pub outline_box: gtk::Box,
}

pub struct DocWindow {
    pub win: adw::ApplicationWindow,
    pub buffer: sourceview5::Buffer,
    pub editors: RefCell<Vec<EditorPane>>,
    pub doc: RefCell<DocState>,
    pub vs: RefCell<ViewState>,
    title: gtk::Label,
    cls: String,
    brace_id: RefCell<Option<glib::SourceId>>,
    syntax_label: gtk::Label,
    counts: gtk::Label,
    size: gtk::Label,
    encoding_label: gtk::Label,
    ending_label: gtk::Label,
    not_editable: gtk::Image,
    status_bar: gtk::Box,
    pub split: adw::OverlaySplitView,
    css: gtk::CssProvider,
    pub actions: gio::SimpleActionGroup,
    dynm: crate::menu::Dynamic,
    autosave_id: RefCell<Option<glib::SourceId>>,
    status_id: RefCell<Option<glib::SourceId>>,
    monitor: RefCell<Option<gio::FileMonitor>>,
    loading: Cell<bool>,
}

thread_local! {
    /// Owns every open document window's state; an entry is dropped when its window is destroyed.
    static WINDOWS: RefCell<Vec<Rc<DocWindow>>> = const { RefCell::new(Vec::new()) };
    static NEXT_ID: Cell<u32> = const { Cell::new(1) };
}

/// All open document windows.
pub fn all() -> Vec<Rc<DocWindow>> {
    WINDOWS.with(|w| w.borrow().clone())
}

fn updown_button(label: &gtk::Label) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    b.append(label);
    b.append(&gtk::Image::from_icon_name("proza-updown-symbolic"));
    b
}

fn flat(b: &impl IsA<gtk::Widget>) {
    b.add_css_class("flat");
}

impl DocWindow {
    pub fn new(app: &adw::Application) -> Rc<Self> {
        let s = settings::get();
        let win = adw::ApplicationWindow::builder().application(app).default_width(640).default_height(720).build();
        win.add_css_class("proza-window");
        // the font stylesheet is per window: each window gets its own class to scope it
        let cls = format!("proza-w{}", NEXT_ID.with(|n| n.replace(n.get() + 1)));
        win.add_css_class(&cls);

        // ---- title bar: menu · title · syntax popup · inspector
        let header = adw::HeaderBar::new();
        let (menu_model, dynm) = crate::menu::build();
        let menu_btn = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu_model)
            .tooltip_text("Menu")
            .primary(true)
            .build();
        header.pack_start(&menu_btn);
        // CotEditor shows the document name left-aligned next to the window buttons
        let title = gtk::Label::new(Some("Untitled"));
        title.add_css_class("document-title");
        title.set_ellipsize(pango::EllipsizeMode::Middle);
        header.pack_start(&title);
        header.set_title_widget(Some(&gtk::Box::new(gtk::Orientation::Horizontal, 0)));
        let inspector_btn = gtk::ToggleButton::builder()
            .icon_name("proza-info-symbolic")
            .tooltip_text("Show Inspector")
            .action_name("win.inspector")
            .build();
        inspector_btn.add_css_class("circular");
        header.pack_end(&inspector_btn);
        let syntax_label = gtk::Label::new(Some("Plain Text"));
        syntax_label.set_xalign(0.0);
        syntax_label.set_width_chars(12);
        let syntax_btn = gtk::MenuButton::builder()
            .child(&updown_button(&syntax_label))
            .menu_model(&dynm.syntaxes)
            .tooltip_text("Change syntax")
            .always_show_arrow(false)
            .build();
        syntax_btn.add_css_class("syntax-popup");
        header.pack_end(&syntax_btn);

        // ---- content: editor stack + status bar, inspector on the right
        let buffer = sourceview5::Buffer::new(None);
        buffer.set_highlight_matching_brackets(false);
        buffer.set_implicit_trailing_newline(false);

        let editors_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        editors_box.set_vexpand(true);

        let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        status_bar.add_css_class("status-bar");
        let not_editable = gtk::Image::from_icon_name("proza-not-editable-symbolic");
        not_editable.set_tooltip_text(Some("The document is not editable."));
        not_editable.set_visible(false);
        status_bar.append(&not_editable);
        let counts = gtk::Label::new(None);
        counts.set_ellipsize(pango::EllipsizeMode::End);
        counts.set_hexpand(true);
        counts.set_xalign(0.0);
        counts.add_css_class("numeric");
        status_bar.append(&counts);
        let size = gtk::Label::new(Some("–"));
        size.set_tooltip_text(Some("File size"));
        size.add_css_class("numeric");
        status_bar.append(&size);
        status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        let encoding_label = gtk::Label::new(Some("Unicode (UTF-8)"));
        let enc_btn = gtk::MenuButton::builder()
            .child(&updown_button(&encoding_label))
            .menu_model(&dynm.encodings)
            .tooltip_text("Text Encoding")
            .direction(gtk::ArrowType::Up)
            .build();
        flat(&enc_btn);
        status_bar.append(&enc_btn);
        status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        let ending_label = gtk::Label::new(Some("LF"));
        ending_label.set_width_chars(4);
        ending_label.set_xalign(0.0);
        let end_btn = gtk::MenuButton::builder()
            .child(&updown_button(&ending_label))
            .menu_model(&dynm.line_endings)
            .tooltip_text("Line Endings")
            .direction(gtk::ArrowType::Up)
            .build();
        flat(&end_btn);
        status_bar.append(&end_btn);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&editors_box);
        content.append(&status_bar);
        let inspector = crate::inspector::placeholder();
        let split = adw::OverlaySplitView::builder()
            .content(&content)
            .sidebar(&inspector)
            .sidebar_position(gtk::PackType::End)
            .show_sidebar(false)
            .min_sidebar_width(200.0)
            .build();

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&split));
        win.set_content(Some(&toolbar));

        let css = gtk::CssProvider::new();
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("display"),
            &css,
            // above the user's stylesheet, which may colour every selection
            gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
        );

        let me = Rc::new(DocWindow {
            win: win.clone(),
            buffer: buffer.clone(),
            editors: RefCell::new(Vec::new()),
            doc: RefCell::new(DocState {
                file: None,
                encoding: Encoding { iana: s.encoding.clone(), bom: s.encoding_bom },
                ending: LineEnding::from_short(&s.line_ending).unwrap_or(LineEnding::Lf),
                syntax: String::new(),
                read_only: false,
                disk_bytes: None,
                backup_id: glib::uuid_string_random().to_string(),
            }),
            vs: RefCell::new(ViewState {
                mono: false,
                zoom: 1.0,
                wrap: s.wrap_lines,
                invisibles: s.show_invisibles,
                line_numbers: s.show_line_numbers,
                page_guide: s.show_page_guide,
                tab_width: s.tab_width,
                soft_tabs: s.auto_expand_tab,
                theme: s.theme.clone(),
            }),
            title,
            cls,
            brace_id: RefCell::new(None),
            syntax_label,
            counts,
            size,
            encoding_label,
            ending_label,
            not_editable,
            status_bar,
            split,
            css,
            actions: gio::SimpleActionGroup::new(),
            dynm,
            autosave_id: RefCell::new(None),
            status_id: RefCell::new(None),
            monitor: RefCell::new(None),
            loading: Cell::new(false),
        });
        WINDOWS.with(|w| w.borrow_mut().push(me.clone()));

        let first = me.make_editor();
        editors_box.append(&first.root);
        GtkWindowExt::set_focus(&win, Some(&first.view));
        me.editors.borrow_mut().push(first);

        {
            let css = me.css.clone();
            win.connect_destroy(move |win| {
                gtk::style_context_remove_provider_for_display(&gdk::Display::default().expect("display"), &css);
                WINDOWS.with(|w| w.borrow_mut().retain(|d| &d.win != win));
            });
        }
        // like CotEditor, the text keeps keyboard focus: closing a menu or pop-up returns it to the editor
        // (the bar buttons never take keyboard focus, as on macOS, so a closing dialog can't leave it on them)
        for w in [inspector_btn.upcast_ref::<gtk::Widget>()] {
            w.set_focusable(false);
        }
        for b in [&menu_btn, &syntax_btn, &enc_btn, &end_btn] {
            b.set_focus_on_click(false);
            b.set_focusable(false);
            if let Some(inner) = b.first_child() {
                inner.set_focusable(false);
            }
            if let Some(pop) = b.popover() {
                let weak = Rc::downgrade(&me);
                // after the menu button's own focus handling, which would otherwise keep the focus
                pop.connect_closed(move |_| {
                    let weak = weak.clone();
                    glib::idle_add_local_once(move || {
                        if let Some(me) = weak.upgrade() {
                            me.view().grab_focus();
                        }
                    });
                });
            }
        }
        me.install_actions();
        me.fill_dynamic_menus();
        me.apply_view_state();
        me.set_syntax("");
        me.connect_buffer();
        me.update_document_labels();
        me.update_status_now();
        {
            let weak = Rc::downgrade(&me);
            adw::StyleManager::default().connect_dark_notify(move |_| {
                if let Some(me) = weak.upgrade() {
                    me.apply_view_state();
                }
            });
        }
        {
            let weak = Rc::downgrade(&me);
            win.connect_close_request(move |_| match weak.upgrade() {
                Some(me) => me.on_close_request(),
                None => glib::Propagation::Proceed,
            });
        }
        me
    }

    // ------------------------------------------------------------------ editor pane

    fn make_editor(self: &Rc<Self>) -> EditorPane {
        let view = sourceview5::View::with_buffer(&self.buffer);
        view.add_css_class("proza-editor");
        view.set_show_line_numbers(true);
        view.set_auto_indent(true);
        view.set_smart_home_end(sourceview5::SmartHomeEndType::Before);
        view.set_top_margin(6);
        view.set_bottom_margin(6);
        view.set_left_margin(12);
        view.set_right_margin(4);
        view.set_right_margin_position(settings::get().page_guide_column);
        let scroller = gtk::ScrolledWindow::builder().child(&view).vexpand(true).hexpand(true).build();

        let nav = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        nav.add_css_class("navigation-bar");
        let close_btn = gtk::Button::builder().icon_name("window-close-symbolic").tooltip_text("Close split editor").build();
        flat(&close_btn);
        close_btn.set_opacity(0.0);
        close_btn.set_sensitive(false);
        nav.append(&close_btn);
        let outline_box = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        nav.append(&outline_box);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        nav.append(&spacer);
        let split_btn = gtk::Button::builder()
            .icon_name("proza-split-add-symbolic")
            .tooltip_text("Split editor")
            .action_name("win.split-editor")
            .build();
        flat(&split_btn);
        split_btn.set_focusable(false);
        close_btn.set_focusable(false);
        nav.append(&split_btn);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&nav);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        root.append(&scroller);
        nav.set_visible(settings::get().show_navigation_bar);

        let weak = Rc::downgrade(self);
        view.connect_has_focus_notify(move |_| {
            if let Some(me) = weak.upgrade() {
                me.schedule_status();
            }
        });
        EditorPane { root, view, nav, close_btn, outline_box }
    }

    /// The editor with keyboard focus (or the first one).
    pub fn view(&self) -> sourceview5::View {
        let eds = self.editors.borrow();
        eds.iter().find(|e| e.view.has_focus()).unwrap_or(&eds[0]).view.clone()
    }

    // ------------------------------------------------------------------ appearance

    pub fn apply_view_state(&self) {
        let vs = self.vs.borrow();
        let s = settings::get();
        let font = pango::FontDescription::from_string(if vs.mono { &s.mono_font } else { &s.font });
        let size = font.size() as f64 / pango::SCALE as f64 * vs.zoom;
        let family = font.family().map(|f| f.to_string()).unwrap_or_else(|| "Monospace".into());
        // the selection: CotEditor's "system" themes keep the system's selection colour (here the user's GTK
        // stylesheet); the others bring their own
        let dark = adw::StyleManager::default().is_dark();
        let selection = crate::theme::editor_colors(&crate::theme::counterpart(&vs.theme, dark))
            .and_then(|c| c.selection.map(|bg| (bg, c.text)))
            .map(|(bg, fg)| format!(".{} .proza-editor text selection {{ background-color: {bg}; color: {fg}; }}", self.cls))
            .unwrap_or_default();
        self.css.load_from_string(&format!(
            ".{cls} .proza-editor {{ font-family: \"{family}\"; font-size: {size:.1}pt; line-height: {lh}; }}\n\
             .{cls} .proza-editor gutter {{ font-family: \"{family}\"; font-size: {gsize:.1}pt; }}\n{selection}",
            cls = self.cls,
            lh = s.line_height,
            gsize = size * 0.9
        ));
        for e in self.editors.borrow().iter() {
            let v = &e.view;
            v.set_wrap_mode(if vs.wrap { gtk::WrapMode::WordChar } else { gtk::WrapMode::None });
            v.set_show_line_numbers(vs.line_numbers);
            v.set_show_right_margin(vs.page_guide);
            v.set_tab_width(vs.tab_width);
            v.set_indent_width(-1);
            v.set_insert_spaces_instead_of_tabs(vs.soft_tabs);
            v.set_highlight_current_line(s.highlight_current_line);
            v.set_editable(!self.doc.borrow().read_only);
            let drawer = v.space_drawer();
            drawer.set_types_for_locations(sourceview5::SpaceLocationFlags::ALL, sourceview5::SpaceTypeFlags::NONE);
            drawer.set_types_for_locations(
                sourceview5::SpaceLocationFlags::ALL,
                sourceview5::SpaceTypeFlags::NEWLINE | sourceview5::SpaceTypeFlags::TAB | sourceview5::SpaceTypeFlags::NBSP,
            );
            drawer.set_enable_matrix(vs.invisibles);
        }
        drop(vs);
        self.apply_theme();
        self.sync_toggle_states();
    }

    fn apply_theme(&self) {
        let dark = adw::StyleManager::default().is_dark();
        let name = crate::theme::counterpart(&self.vs.borrow().theme, dark);
        let scheme = sourceview5::StyleSchemeManager::default().scheme(&crate::theme::scheme_id(&name));
        self.buffer.set_style_scheme(scheme.as_ref());
        if let Some(tag) = self.buffer.tag_table().lookup("proza-brace") {
            self.style_brace_tag(&tag);
        }
    }

    // ------------------------------------------------------------------ syntax

    pub fn set_syntax(&self, id: &str) {
        let lm = sourceview5::LanguageManager::default();
        let lang = if id.is_empty() { None } else { lm.language(id) };
        self.buffer.set_language(lang.as_ref());
        let name = lang.as_ref().map(|l| l.name().to_string()).unwrap_or_else(|| "Plain Text".into());
        self.syntax_label.set_label(&name);
        let mono = !GENERAL_SYNTAXES.contains(&id);
        self.doc.borrow_mut().syntax = id.to_string();
        if self.vs.borrow().mono != mono {
            self.vs.borrow_mut().mono = mono;
            self.apply_view_state();
        }
        if let Some(a) = self.actions.lookup_action("syntax") {
            a.change_state(&id.to_variant());
        }
        if let Some(a) = self.actions.lookup_action("font-type") {
            a.change_state(&(if mono { "monospaced" } else { "standard" }).to_variant());
        }
    }

    fn guess_syntax(&self, name: &str, text: &str) -> String {
        let lm = sourceview5::LanguageManager::default();
        let first = text.lines().next().unwrap_or("");
        let ctype = if first.starts_with("#!") {
            gio::content_type_guess(Some(name), first.as_bytes()).0.to_string()
        } else {
            String::new()
        };
        // Proza's own syntaxes win over the built-in ones that claim the same files (SVG over XML)
        let lower = name.to_lowercase();
        for (id, _) in crate::app::LANGUAGES {
            let globs = lm.language(id).map(|l| l.globs()).unwrap_or_default();
            if globs.iter().any(|g| g.strip_prefix('*').is_some_and(|ext| lower.ends_with(&ext.to_lowercase()))) {
                return id.to_string();
            }
        }
        lm.guess_language(Some(name), if ctype.is_empty() { None } else { Some(ctype.as_str()) })
            .map(|l| l.id().to_string())
            .unwrap_or_default()
    }

    // ------------------------------------------------------------------ dynamic menus

    fn fill_dynamic_menus(&self) {
        let d = &self.dynm;
        // line endings
        let main = gio::Menu::new();
        for e in [LineEnding::Lf, LineEnding::Cr, LineEnding::CrLf] {
            let item = gio::MenuItem::new(Some(e.short()), None);
            item.set_action_and_target_value(Some("win.line-ending"), Some(&e.short().to_variant()));
            main.append_item(&item);
        }
        d.line_endings.append_section(Some("Line Endings"), &main);
        let more = gio::Menu::new();
        for e in [LineEnding::Nel, LineEnding::Ls, LineEnding::Ps] {
            let item = gio::MenuItem::new(Some(e.short()), None);
            item.set_action_and_target_value(Some("win.line-ending"), Some(&e.short().to_variant()));
            more.append_item(&item);
        }
        d.line_endings.append_section(None, &more);
        self.fill_encodings_menu();
        // syntaxes: None, recently used, all
        let lm = sourceview5::LanguageManager::default();
        let mut langs: Vec<(String, String)> = lm
            .language_ids()
            .iter()
            .filter_map(|id| lm.language(id))
            .filter(|l| !l.is_hidden())
            .map(|l| (l.id().to_string(), l.name().to_string()))
            .collect();
        langs.sort_by_key(|(_, n)| n.to_lowercase());
        let none = gio::Menu::new();
        let it = gio::MenuItem::new(Some("None"), None);
        it.set_action_and_target_value(Some("win.syntax"), Some(&"".to_variant()));
        none.append_item(&it);
        d.syntaxes.append_section(None, &none);
        let recent = settings::get().recent_syntaxes;
        if !recent.is_empty() {
            let r = gio::Menu::new();
            for id in recent.iter().take(6) {
                if let Some((_, n)) = langs.iter().find(|(i, _)| i == id) {
                    let it = gio::MenuItem::new(Some(n), None);
                    it.set_action_and_target_value(Some("win.syntax"), Some(&id.to_variant()));
                    r.append_item(&it);
                }
            }
            d.syntaxes.append_section(Some("Recently Used"), &r);
        }
        let all = gio::Menu::new();
        for (id, n) in &langs {
            let it = gio::MenuItem::new(Some(n), None);
            it.set_action_and_target_value(Some("win.syntax"), Some(&id.to_variant()));
            all.append_item(&it);
        }
        d.syntaxes.append_section(None, &all);
        // themes
        let t = gio::Menu::new();
        for (name, _) in crate::theme::BUNDLED {
            let it = gio::MenuItem::new(Some(name), None);
            it.set_action_and_target_value(Some("win.theme"), Some(&name.to_variant()));
            t.append_item(&it);
        }
        d.themes.append_section(None, &t);
        crate::app::fill_recent(&d.recent);
    }

    fn fill_encodings_menu(&self) {
        let d = &self.dynm;
        d.encodings.remove_all();
        let current = self.doc.borrow().encoding.clone();
        let list = settings::get().encodings;
        let listed = list.iter().any(|e| e.eq_ignore_ascii_case(&current.iana));
        let item = |enc: &Encoding| {
            let it = gio::MenuItem::new(Some(&enc.label()), None);
            it.set_action_and_target_value(
                Some("win.encoding"),
                Some(&format!("{}|{}", enc.iana, if enc.bom { "bom" } else { "" }).to_variant()),
            );
            it
        };
        if !listed {
            let top = gio::Menu::new();
            top.append_item(&item(&current));
            d.encodings.append_section(Some("Text Encoding"), &top);
        }
        let mut sec = gio::Menu::new();
        let mut first = listed;
        for e in &list {
            if e == "-" {
                if sec.n_items() > 0 {
                    d.encodings.append_section(if first { Some("Text Encoding") } else { None }, &sec);
                    first = false;
                    sec = gio::Menu::new();
                }
                continue;
            }
            sec.append_item(&item(&Encoding { iana: e.clone(), bom: false }));
            if e.eq_ignore_ascii_case("UTF-8") {
                sec.append_item(&item(&Encoding { iana: e.clone(), bom: true }));
            }
        }
        if sec.n_items() > 0 {
            d.encodings.append_section(None, &sec);
        }
    }

    // ------------------------------------------------------------------ actions

    pub fn add_action(self: &Rc<Self>, name: &str, f: impl Fn(&Rc<Self>) + 'static) {
        let a = gio::SimpleAction::new(name, None);
        let weak = Rc::downgrade(self);
        a.connect_activate(move |_, _| {
            if let Some(me) = weak.upgrade() {
                f(&me)
            }
        });
        self.actions.add_action(&a);
    }

    pub fn add_toggle(self: &Rc<Self>, name: &str, initial: bool, f: impl Fn(&Rc<Self>, bool) + 'static) {
        let a = gio::SimpleAction::new_stateful(name, None, &initial.to_variant());
        let weak = Rc::downgrade(self);
        a.connect_activate(move |a, _| {
            let on = !a.state().and_then(|v| v.get::<bool>()).unwrap_or(false);
            a.set_state(&on.to_variant());
            if let Some(me) = weak.upgrade() {
                f(&me, on)
            }
        });
        self.actions.add_action(&a);
    }

    pub fn add_radio(self: &Rc<Self>, name: &str, initial: &str, f: impl Fn(&Rc<Self>, String) + 'static) {
        let a = gio::SimpleAction::new_stateful(name, Some(glib::VariantTy::STRING), &initial.to_variant());
        let weak = Rc::downgrade(self);
        a.connect_activate(move |a, v| {
            let Some(val) = v.and_then(|v| v.get::<String>()) else { return };
            a.set_state(&val.to_variant());
            if let Some(me) = weak.upgrade() {
                f(&me, val)
            }
        });
        self.actions.add_action(&a);
    }

    fn sync_toggle_states(&self) {
        let vs = self.vs.borrow();
        let s = settings::get();
        for (name, on) in [
            ("wrap", vs.wrap),
            ("invisibles", vs.invisibles),
            ("line-numbers", vs.line_numbers),
            ("page-guide", vs.page_guide),
            ("soft-tabs", vs.soft_tabs),
            ("status-bar", s.show_status_bar),
            ("navigation-bar", s.show_navigation_bar),
            ("prevent-editing", self.doc.borrow().read_only),
        ] {
            if let Some(a) = self.actions.lookup_action(name) {
                a.change_state(&on.to_variant());
            }
        }
        if let Some(a) = self.actions.lookup_action("tab-width") {
            a.change_state(&vs.tab_width.to_string().to_variant());
        }
        if let Some(a) = self.actions.lookup_action("theme") {
            a.change_state(&vs.theme.to_variant());
        }
        self.status_bar.set_visible(s.show_status_bar);
        for e in self.editors.borrow().iter() {
            e.nav.set_visible(s.show_navigation_bar);
        }
    }

    fn install_actions(self: &Rc<Self>) {
        self.win.insert_action_group("win", Some(&self.actions));
        let s = settings::get();

        // File
        self.add_action("close", |me| me.win.close());
        self.add_action("save", |me| me.save(false));
        self.add_action("save-as", |me| me.save(true));
        self.add_action("revert", |me| {
            if let Some(f) = me.doc.borrow().file.clone() {
                me.load_file(&f, None);
            }
        });
        self.add_action("copy-path", |me| {
            if let Some(p) = me.doc.borrow().file.as_ref().and_then(|f| f.path()) {
                me.win.clipboard().set_text(&p.display().to_string());
            }
        });
        self.add_action("show-in-files", |me| {
            if let Some(f) = me.doc.borrow().file.clone() {
                crate::app::show_in_files(&f);
            }
        });

        // Edit
        self.add_action("undo", |me| {
            if me.buffer.can_undo() {
                me.buffer.undo()
            }
        });
        self.add_action("redo", |me| {
            if me.buffer.can_redo() {
                me.buffer.redo()
            }
        });
        for (name, signal) in [("cut", "cut-clipboard"), ("copy", "copy-clipboard"), ("paste", "paste-clipboard")] {
            self.add_action(name, move |me| {
                me.view().emit_by_name::<()>(signal, &[]);
            });
        }
        self.add_action("paste-exactly", |me| me.view().emit_by_name::<()>("paste-clipboard", &[]));
        self.add_action("delete", |me| {
            me.buffer.delete_selection(true, !me.doc.borrow().read_only);
        });
        self.add_action("select-all", |me| {
            let (s, e) = me.buffer.bounds();
            me.buffer.select_range(&s, &e);
        });
        self.add_action("emoji", |me| {
            me.view().emit_by_name::<()>("insert-emoji", &[]);
        });

        // Format
        self.add_radio("line-ending", "LF", |me, v| {
            if let Some(e) = LineEnding::from_short(&v) {
                me.change_line_ending(e);
            }
        });
        self.add_radio("encoding", "UTF-8|", |me, v| me.change_encoding_request(&v));
        self.add_radio("syntax", "", |me, v| {
            me.set_syntax(&v);
            settings::update(|s| {
                s.recent_syntaxes.retain(|x| x != &v);
                if !v.is_empty() {
                    s.recent_syntaxes.insert(0, v.clone());
                    s.recent_syntaxes.truncate(6);
                }
            });
        });
        self.add_action("reset-syntax", |me| {
            let id = me.doc.borrow().syntax.clone();
            me.set_syntax("");
            me.set_syntax(&id);
        });
        self.add_radio("font-type", "monospaced", |me, v| {
            me.vs.borrow_mut().mono = v == "monospaced";
            me.apply_view_state();
        });
        self.add_action("bigger", |me| me.zoom(1.1));
        self.add_action("smaller", |me| me.zoom(1.0 / 1.1));
        self.add_action("reset-zoom", |me| me.zoom(0.0));
        self.add_radio("theme", &s.theme, |me, v| {
            me.vs.borrow_mut().theme = v;
            me.apply_view_state();
        });
        self.add_radio("tab-width", &s.tab_width.to_string(), |me, v| {
            me.vs.borrow_mut().tab_width = v.parse().unwrap_or(4);
            me.apply_view_state();
        });
        self.add_toggle("soft-tabs", s.auto_expand_tab, |me, on| {
            me.vs.borrow_mut().soft_tabs = on;
            me.apply_view_state();
        });
        self.add_toggle("invisibles", s.show_invisibles, |me, on| {
            me.vs.borrow_mut().invisibles = on;
            me.apply_view_state();
        });
        self.add_toggle("page-guide", s.show_page_guide, |me, on| {
            me.vs.borrow_mut().page_guide = on;
            me.apply_view_state();
        });
        self.add_toggle("wrap", s.wrap_lines, |me, on| {
            me.vs.borrow_mut().wrap = on;
            me.apply_view_state();
        });
        self.add_toggle("prevent-editing", false, |me, on| {
            me.doc.borrow_mut().read_only = on;
            me.not_editable.set_visible(on);
            me.apply_view_state();
        });

        // View
        self.add_toggle("inspector", false, |me, on| me.split.set_show_sidebar(on));
        self.add_radio("inspector-pane", &s.inspector_pane.to_string(), |me, v| {
            me.split.set_show_sidebar(true);
            if let Some(a) = me.actions.lookup_action("inspector") {
                a.change_state(&true.to_variant());
            }
            let pane = v.parse().unwrap_or(0);
            settings::update(|s| s.inspector_pane = pane);
        });
        self.add_toggle("line-numbers", s.show_line_numbers, |me, on| {
            me.vs.borrow_mut().line_numbers = on;
            me.apply_view_state();
        });
        self.add_toggle("status-bar", s.show_status_bar, |_, on| {
            settings::update(|s| s.show_status_bar = on);
            for w in all() {
                w.sync_toggle_states();
            }
        });
        self.add_toggle("navigation-bar", s.show_navigation_bar, |_, on| {
            settings::update(|s| s.show_navigation_bar = on);
            for w in all() {
                w.sync_toggle_states();
            }
        });
        self.add_action("fullscreen", |me| {
            if me.win.is_fullscreen() {
                me.win.unfullscreen()
            } else {
                me.win.fullscreen()
            }
        });
        crate::textcmd::install(self);
    }

    fn zoom(&self, factor: f64) {
        {
            let mut vs = self.vs.borrow_mut();
            vs.zoom = if factor == 0.0 { 1.0 } else { (vs.zoom * factor).clamp(0.25, 4.0) };
        }
        self.apply_view_state();
    }

    // ------------------------------------------------------------------ status bar

    fn connect_buffer(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.buffer.connect_changed(move |_| {
            if let Some(me) = weak.upgrade() {
                me.schedule_status();
                if !me.loading.get() {
                    me.schedule_autosave();
                }
            }
        });
        let weak = Rc::downgrade(self);
        self.buffer.connect_mark_set(move |_, _, mark| {
            if mark.name().as_deref() == Some("insert") || mark.name().as_deref() == Some("selection_bound") {
                if let Some(me) = weak.upgrade() {
                    me.schedule_status();
                    // a cursor move sets "insert" first and "selection_bound" second: only then is it not a selection
                    me.flash_brace();
                }
            }
        });
        let weak = Rc::downgrade(self);
        self.buffer.connect_modified_changed(move |_| {
            if let Some(me) = weak.upgrade() {
                me.update_title();
            }
        });
        // typed or pasted line endings become the document's line ending
        let weak = Rc::downgrade(self);
        self.buffer.connect_insert_text(move |buf, iter, text| {
            let Some(me) = weak.upgrade() else { return };
            if me.loading.get() {
                return;
            }
            let ending = me.doc.borrow().ending;
            if !text.contains(['\n', '\r', '\u{85}', '\u{2028}', '\u{2029}']) {
                return;
            }
            let converted = document::convert_line_endings(text, ending);
            if converted != text {
                buf.stop_signal_emission_by_name("insert-text");
                me.loading.set(true);
                buf.insert(iter, &converted);
                me.loading.set(false);
                me.schedule_autosave();
            }
        });
    }

    /// CotEditor marks the brace matching the one just before the cursor, only for a moment.
    fn flash_brace(self: &Rc<Self>) {
        let b = &self.buffer;
        if !settings::get().highlight_braces || b.has_selection() || self.loading.get() {
            return;
        }
        let cursor = b.iter_at_mark(&b.get_insert());
        let mut before = cursor;
        if !before.backward_char() {
            return;
        }
        let Some(target) = matching_brace(&before) else { return };
        let tag = self.brace_tag();
        b.remove_tag(&tag, &b.start_iter(), &b.end_iter());
        let mut end = target;
        end.forward_char();
        b.apply_tag(&tag, &target, &end);
        if let Some(id) = self.brace_id.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(std::time::Duration::from_millis(600), move || {
            if let Some(me) = weak.upgrade() {
                me.brace_id.borrow_mut().take();
                let tag = me.brace_tag();
                me.buffer.remove_tag(&tag, &me.buffer.start_iter(), &me.buffer.end_iter());
            }
        });
        self.brace_id.replace(Some(id));
    }

    /// The tag for the brace flash, coloured by the theme's bracket-match style.
    fn brace_tag(&self) -> gtk::TextTag {
        let table = self.buffer.tag_table();
        table.lookup("proza-brace").unwrap_or_else(|| {
            let tag = gtk::TextTag::new(Some("proza-brace"));
            table.add(&tag);
            self.style_brace_tag(&tag);
            tag
        })
    }

    fn style_brace_tag(&self, tag: &gtk::TextTag) {
        if let Some(style) = self.buffer.style_scheme().and_then(|sc| sc.style("bracket-match")) {
            style.apply(tag);
        }
        tag.set_priority(self.buffer.tag_table().size() - 1);
    }

    fn schedule_status(self: &Rc<Self>) {
        if self.status_id.borrow().is_some() {
            return;
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(std::time::Duration::from_millis(60), move || {
            if let Some(me) = weak.upgrade() {
                me.status_id.borrow_mut().take();
                me.update_status_now();
            }
        });
        self.status_id.replace(Some(id));
    }

    fn update_status_now(&self) {
        let s = settings::get();
        let b = &self.buffer;
        let text = b.text(&b.start_iter(), &b.end_iter(), false);
        let (sel_s, sel_e) = b.selection_bounds().unwrap_or_else(|| {
            let i = b.iter_at_mark(&b.get_insert());
            (i, i)
        });
        let selected = b.text(&sel_s, &sel_e, false);
        let mut parts: Vec<String> = Vec::new();
        let item = |label: &str, value: String| format!("<span alpha='55%'>{label}:</span> {value}");
        let with_sel = |total: usize, sel: usize| if sel > 0 { format!("{} ({})", group(total), group(sel)) } else { group(total) };
        if s.status_lines {
            // distinct lines touched by the selection; a selection ending at a line start doesn't touch that line
            let sel = if selected.is_empty() {
                0
            } else {
                let end_line = if sel_e.starts_line() && sel_e.line() > sel_s.line() { sel_e.line() - 1 } else { sel_e.line() };
                (end_line - sel_s.line() + 1) as usize
            };
            parts.push(item("Lines", with_sel(count_lines(&text), sel)));
        }
        if s.status_chars {
            parts.push(item("Characters", with_sel(text.graphemes(true).count(), selected.graphemes(true).count())));
        }
        if s.status_words {
            parts.push(item("Words", with_sel(text.unicode_words().count(), selected.unicode_words().count())));
        }
        if s.status_location {
            let before = b.text(&b.start_iter(), &sel_s, false);
            parts.push(item("Location", group(before.graphemes(true).count())));
        }
        if s.status_line {
            parts.push(item("Line", group(sel_s.line() as usize + 1)));
        }
        if s.status_column {
            let mut ls = sel_s;
            ls.set_line_offset(0);
            parts.push(item("Column", group(b.text(&ls, &sel_s, false).graphemes(true).count())));
        }
        self.counts.set_markup(&parts.join("   "));
    }

    fn update_size(&self) {
        let text = match self.doc.borrow().disk_bytes.as_ref() {
            Some(b) => human_size(b.len()),
            None => "–".into(),
        };
        self.size.set_label(&text);
    }

    fn update_title(&self) {
        let d = self.doc.borrow();
        let name = d.file.as_ref().and_then(|f| f.basename()).map(|p| p.display().to_string()).unwrap_or_else(|| "Untitled".into());
        let edited = self.buffer.is_modified() && !(settings::get().auto_save && d.file.is_some());
        self.title.set_label(&if edited { format!("{name} •") } else { name.clone() });
        self.win.set_title(Some(&name));
    }

    fn update_document_labels(&self) {
        let d = self.doc.borrow();
        self.encoding_label.set_label(&d.encoding.label());
        self.ending_label.set_label(d.ending.short());
        drop(d);
        if let Some(a) = self.actions.lookup_action("line-ending") {
            a.change_state(&self.doc.borrow().ending.short().to_variant());
        }
        let enc = self.doc.borrow().encoding.clone();
        if let Some(a) = self.actions.lookup_action("encoding") {
            a.change_state(&format!("{}|{}", enc.iana, if enc.bom { "bom" } else { "" }).to_variant());
        }
        self.fill_encodings_menu();
        self.update_size();
        self.update_title();
    }

    // ------------------------------------------------------------------ line endings & encodings

    fn change_line_ending(&self, e: LineEnding) {
        if self.doc.borrow().read_only {
            self.update_document_labels();
            return;
        }
        let text = self.text();
        let converted = document::convert_line_endings(&text, e);
        self.doc.borrow_mut().ending = e;
        if converted != text {
            self.replace_all_text(&converted);
        }
        self.update_document_labels();
        self.save_if_auto();
    }

    /// Replaces the whole text as one undo step, keeping the cursor line.
    pub fn replace_all_text(&self, text: &str) {
        let b = &self.buffer;
        let line = b.iter_at_mark(&b.get_insert()).line();
        self.loading.set(true);
        b.begin_user_action();
        let (mut s, mut e) = b.bounds();
        b.delete(&mut s, &mut e);
        b.insert(&mut b.start_iter(), text);
        b.end_user_action();
        self.loading.set(false);
        if let Some(it) = b.iter_at_line(line) {
            b.place_cursor(&it);
        }
    }

    fn change_encoding_request(self: &Rc<Self>, v: &str) {
        let (iana, bom) = v.split_once('|').unwrap_or((v, ""));
        let enc = Encoding { iana: iana.to_string(), bom: bom == "bom" };
        if enc == self.doc.borrow().encoding {
            return;
        }
        let empty = self.buffer.char_count() == 0;
        if self.doc.borrow().file.is_none() || empty {
            self.doc.borrow_mut().encoding = enc;
            self.update_document_labels();
            return;
        }
        let dlg = adw::AlertDialog::new(
            Some("Text encoding change"),
            Some(&format!("Do you want to convert or reinterpret this document using “{}”?", enc.label())),
        );
        dlg.add_responses(&[("cancel", "Cancel"), ("reinterpret", "Reinterpret"), ("convert", "Convert")]);
        dlg.set_default_response(Some("convert"));
        dlg.set_close_response("cancel");
        let weak = Rc::downgrade(self);
        dlg.choose(Some(&self.win), gio::Cancellable::NONE, move |r| {
            let Some(me) = weak.upgrade() else { return };
            match r.as_str() {
                "convert" => me.convert_encoding(enc),
                "reinterpret" => me.reinterpret(enc),
                _ => me.update_document_labels(),
            }
        });
    }

    fn convert_encoding(self: &Rc<Self>, enc: Encoding) {
        if !document::incompatible_chars(&self.text(), &enc).is_empty() {
            let dlg = adw::AlertDialog::new(
                Some(&format!("The document contains characters incompatible with “{}.”", enc.label())),
                Some("Incompatible characters are either substituted or removed in saving. Do you want to change the text encoding and review the incompatible characters?"),
            );
            dlg.add_responses(&[("cancel", "Cancel"), ("change", "Change Encoding")]);
            dlg.set_close_response("cancel");
            let weak = Rc::downgrade(self);
            dlg.choose(Some(&self.win), gio::Cancellable::NONE, move |r| {
                let Some(me) = weak.upgrade() else { return };
                if r == "change" {
                    me.set_encoding(enc);
                    me.actions.activate_action("inspector-pane", Some(&"2".to_variant()));
                } else {
                    me.update_document_labels();
                }
            });
            return;
        }
        self.set_encoding(enc);
    }

    fn set_encoding(&self, enc: Encoding) {
        self.doc.borrow_mut().encoding = enc;
        self.update_document_labels();
        self.buffer.set_modified(true);
        self.save_if_auto();
    }

    fn reinterpret(self: &Rc<Self>, enc: Encoding) {
        let Some(file) = self.doc.borrow().file.clone() else { return };
        let go = {
            let weak = Rc::downgrade(self);
            let enc = enc.clone();
            move || {
                if let Some(me) = weak.upgrade() {
                    if !me.load_file(&file, Some(&enc)) {
                        let dlg = adw::AlertDialog::new(
                            Some(&format!("The document could not be reinterpreted using text encoding “{}.”", enc.label())),
                            Some("The document may have been saved using a different text encoding, or it may not be a text file."),
                        );
                        dlg.add_response("ok", "OK");
                        dlg.present(Some(&me.win));
                        me.update_document_labels();
                    }
                }
            }
        };
        if self.buffer.is_modified() {
            let dlg = adw::AlertDialog::new(
                Some("The document has unsaved changes."),
                Some(&format!("Are you sure you want to discard your changes and reopen the document using “{}”?", enc.label())),
            );
            dlg.add_responses(&[("cancel", "Cancel"), ("discard", "Discard Changes")]);
            dlg.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
            dlg.set_close_response("cancel");
            let weak = Rc::downgrade(self);
            dlg.choose(Some(&self.win), gio::Cancellable::NONE, move |r| {
                if r == "discard" {
                    go()
                } else if let Some(me) = weak.upgrade() {
                    me.update_document_labels();
                }
            });
        } else {
            go();
        }
    }

    // ------------------------------------------------------------------ opening

    /// Shows `text` as an untitled document (crash backups).
    pub fn set_untitled_text(&self, text: &str) {
        self.loading.set(true);
        self.buffer.set_text(text);
        self.loading.set(false);
        self.buffer.set_modified(!text.is_empty());
        self.update_status_now();
    }

    /// Untouched untitled window (CotEditor replaces it with the first file opened).
    pub fn is_blank(&self) -> bool {
        self.doc.borrow().file.is_none() && self.buffer.char_count() == 0 && !self.buffer.can_undo()
    }

    /// Reads `file` (with `forced` encoding, or detection). Returns false if it can't be decoded.
    pub fn load_file(self: &Rc<Self>, file: &gio::File, forced: Option<&Encoding>) -> bool {
        let Ok((bytes, _)) = file.load_contents(gio::Cancellable::NONE) else { return false };
        let s = settings::get();
        let Some(dec) = document::decode(&bytes, &settings::candidate_encodings(), forced, s.refer_to_declaration) else {
            return false;
        };
        let (dominant, inconsistent) = document::dominant_line_ending(&dec.text);
        let first_open = self.doc.borrow().file.as_ref().map(|f| !f.equal(file)).unwrap_or(true);
        let cursor_line = self.buffer.iter_at_mark(&self.buffer.get_insert()).line();
        self.loading.set(true);
        self.buffer.begin_irreversible_action();
        self.buffer.set_text(&dec.text);
        self.buffer.end_irreversible_action();
        self.loading.set(false);
        self.buffer.set_modified(false);
        {
            let mut d = self.doc.borrow_mut();
            d.file = Some(file.clone());
            d.encoding = dec.encoding;
            if let Some(e) = dominant {
                d.ending = e;
            }
            d.disk_bytes = Some(bytes.to_vec());
        }
        if first_open {
            let name = file.basename().map(|p| p.display().to_string()).unwrap_or_default();
            let id = self.guess_syntax(&name, &dec.text);
            self.set_syntax(&id);
            self.buffer.place_cursor(&self.buffer.start_iter());
            // indentation style detection: tabs or spaces (width is not detected)
            let (tabs, spaces) = dec.text.lines().fold((0, 0), |(t, sp), l| {
                (t + l.starts_with('\t') as usize, sp + l.starts_with("  ") as usize)
            });
            if tabs + spaces > 0 {
                self.vs.borrow_mut().soft_tabs = spaces > tabs;
                self.apply_view_state();
            }
            crate::app::add_recent(file);
        } else if let Some(it) = self.buffer.iter_at_line(cursor_line) {
            self.buffer.place_cursor(&it);
        }
        self.update_document_labels();
        self.update_status_now();
        self.watch_file(file);
        if first_open && inconsistent {
            self.warn_inconsistent_endings();
        }
        true
    }

    fn warn_inconsistent_endings(self: &Rc<Self>) {
        let ending = self.doc.borrow().ending;
        let dlg = adw::AlertDialog::new(
            Some("The document has inconsistent line endings."),
            Some(&format!(
                "Do you want to convert all line endings to {}, the most common line ending in this document?",
                ending.short()
            )),
        );
        dlg.add_responses(&[("ignore", "Ignore"), ("review", "Review"), ("convert", "Convert")]);
        dlg.set_default_response(Some("convert"));
        dlg.set_close_response("ignore");
        let weak = Rc::downgrade(self);
        dlg.choose(Some(&self.win), gio::Cancellable::NONE, move |r| {
            let Some(me) = weak.upgrade() else { return };
            match r.as_str() {
                "convert" => me.change_line_ending(ending),
                "review" => me.actions.activate_action("inspector-pane", Some(&"2".to_variant())),
                _ => {}
            }
        });
    }

    fn watch_file(self: &Rc<Self>, file: &gio::File) {
        let Ok(mon) = file.monitor_file(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE) else { return };
        let weak = Rc::downgrade(self);
        mon.connect_changed(move |_, f, _, ev| {
            if !matches!(ev, gio::FileMonitorEvent::ChangesDoneHint | gio::FileMonitorEvent::Created) {
                return;
            }
            let Some(me) = weak.upgrade() else { return };
            let Ok((bytes, _)) = f.load_contents(gio::Cancellable::NONE) else { return };
            // CotEditor's default "Update to modified version": silent, keeps the cursor line; compared byte for
            // byte, so our own saves never count
            if me.doc.borrow().disk_bytes.as_deref() == Some(&bytes[..]) {
                return;
            }
            let enc = me.doc.borrow().encoding.clone();
            me.load_file(f, Some(&enc));
        });
        self.monitor.replace(Some(mon));
    }

    // ------------------------------------------------------------------ saving

    /// Saves right away when auto-save applies (after a line-ending or encoding change).
    fn save_if_auto(&self) {
        if settings::get().auto_save && self.doc.borrow().file.is_some() && !self.doc.borrow().read_only {
            self.write_file();
        }
    }

    fn schedule_autosave(self: &Rc<Self>) {
        if let Some(id) = self.autosave_id.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_seconds_local_once(AUTOSAVE_DELAY, move || {
            let Some(me) = weak.upgrade() else { return };
            me.autosave_id.borrow_mut().take();
            if settings::get().auto_save && me.doc.borrow().file.is_some() {
                if me.buffer.is_modified() && !me.doc.borrow().read_only {
                    me.write_file();
                }
            } else {
                crate::app::write_backup(&me);
            }
        });
        self.autosave_id.replace(Some(id));
    }

    pub fn text(&self) -> String {
        self.buffer.text(&self.buffer.start_iter(), &self.buffer.end_iter(), false).to_string()
    }

    /// Writes the document to its file. Returns false on failure (and says why).
    fn write_file(&self) -> bool {
        let (file, enc) = {
            let d = self.doc.borrow();
            match d.file.clone() {
                Some(f) => (f, d.encoding.clone()),
                None => return false,
            }
        };
        if settings::get().auto_trim_trailing_whitespace {
            crate::textcmd::trim_trailing_whitespace(self, true);
        }
        let bytes = document::encode(&self.text(), &enc);
        match file.replace_contents(&bytes, None, false, gio::FileCreateFlags::NONE, gio::Cancellable::NONE) {
            Ok(_) => {
                self.doc.borrow_mut().disk_bytes = Some(bytes);
                self.buffer.set_modified(false);
                crate::app::remove_backup(&self.doc.borrow().backup_id);
                self.update_size();
                self.update_title();
                true
            }
            Err(e) => {
                let dlg = adw::AlertDialog::new(Some("The document could not be saved."), Some(e.message()));
                dlg.add_response("ok", "OK");
                dlg.present(Some(&self.win));
                false
            }
        }
    }

    pub fn save(self: &Rc<Self>, ask: bool) {
        if !ask && self.doc.borrow().file.is_some() {
            self.write_file();
            return;
        }
        let dialog = gtk::FileDialog::builder().title(if ask { "Save As" } else { "Save" }).modal(true).build();
        let name = self.doc.borrow().file.as_ref().and_then(|f| f.basename()).map(|p| p.display().to_string());
        dialog.set_initial_name(Some(name.as_deref().unwrap_or("Untitled.txt")));
        if let Some(dir) = self.doc.borrow().file.as_ref().and_then(|f| f.parent()) {
            dialog.set_initial_folder(Some(&dir));
        }
        let weak = Rc::downgrade(self);
        dialog.save(Some(&self.win), gio::Cancellable::NONE, move |res| {
            let (Some(me), Ok(file)) = (weak.upgrade(), res) else { return };
            me.doc.borrow_mut().file = Some(file.clone());
            if me.write_file() {
                let name = file.basename().map(|p| p.display().to_string()).unwrap_or_default();
                if me.doc.borrow().syntax.is_empty() {
                    let id = me.guess_syntax(&name, &me.text());
                    me.set_syntax(&id);
                }
                crate::app::add_recent(&file);
                me.watch_file(&file);
                me.update_document_labels();
            }
        });
    }

    fn on_close_request(self: &Rc<Self>) -> glib::Propagation {
        let has_file = self.doc.borrow().file.is_some();
        if !self.buffer.is_modified() || (self.buffer.char_count() == 0 && !has_file) {
            crate::app::remove_backup(&self.doc.borrow().backup_id);
            return glib::Propagation::Proceed;
        }
        if has_file && settings::get().auto_save {
            return if self.write_file() { glib::Propagation::Proceed } else { glib::Propagation::Stop };
        }
        let name = self.title.label();
        let dlg = adw::AlertDialog::new(
            Some(&format!("Do you want to save the changes made to the document “{}”?", name.trim_end_matches(" •"))),
            Some("Your changes will be lost if you don’t save them."),
        );
        dlg.add_responses(&[("discard", "Don’t Save"), ("cancel", "Cancel"), ("save", "Save…")]);
        dlg.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dlg.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dlg.set_default_response(Some("save"));
        dlg.set_close_response("cancel");
        let weak = Rc::downgrade(self);
        dlg.choose(Some(&self.win), gio::Cancellable::NONE, move |r| {
            let Some(me) = weak.upgrade() else { return };
            match r.as_str() {
                "discard" => {
                    crate::app::remove_backup(&me.doc.borrow().backup_id);
                    me.buffer.set_modified(false);
                    me.win.close();
                }
                "save" => {
                    if me.doc.borrow().file.is_some() {
                        if me.write_file() {
                            me.win.close();
                        }
                    } else {
                        me.save(true);
                    }
                }
                _ => {}
            }
        });
        glib::Propagation::Stop
    }
}

/// Lines as CotEditor counts them: the empty line after a final line ending doesn't count.
/// The brace that pairs with the one at `at`: () [] {} — searched backward from a closing brace and forward from
/// an opening one, nesting counted, at most 50,000 characters away.
fn matching_brace(at: &gtk::TextIter) -> Option<gtk::TextIter> {
    const PAIRS: [(char, char); 3] = [('(', ')'), ('[', ']'), ('{', '}')];
    let c = at.char();
    let (open, close, forward) = PAIRS
        .iter()
        .find_map(|&(o, cl)| if c == o { Some((o, cl, true)) } else if c == cl { Some((o, cl, false)) } else { None })?;
    let mut it = *at;
    let mut depth = 0usize;
    for _ in 0..50_000 {
        let moved = if forward { it.forward_char() } else { it.backward_char() };
        if !moved || it.is_end() {
            return None;
        }
        let ch = it.char();
        if ch == (if forward { open } else { close }) {
            depth += 1;
        } else if ch == (if forward { close } else { open }) {
            if depth == 0 {
                return Some(it);
            }
            depth -= 1;
        }
    }
    None
}

pub fn count_lines(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let n = document::line_endings(text).len();
    if text.ends_with(['\n', '\r', '\u{85}', '\u{2028}', '\u{2029}']) {
        n
    } else {
        n + 1
    }
}

/// 4512 -> "4,512"
pub fn group(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub fn human_size(bytes: usize) -> String {
    match bytes {
        1 => "1 byte".into(),
        b if b < 1000 => format!("{b} bytes"),
        b if b < 1_000_000 => format!("{} kB", (b as f64 / 1000.0).round() as u64),
        b if b < 1_000_000_000 => format!("{:.1} MB", b as f64 / 1_000_000.0),
        b => format!("{:.2} GB", b as f64 / 1_000_000_000.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counting_like_coteditor() {
        assert_eq!(count_lines(""), 0);
        assert_eq!(count_lines("a\n"), 1);
        assert_eq!(count_lines("a\nb"), 2);
        assert_eq!(count_lines("a\r\nb\r\n"), 2);
        assert_eq!(group(4512), "4,512");
        assert_eq!(group(999), "999");
        assert_eq!(human_size(2048), "2 kB");
    }
}
