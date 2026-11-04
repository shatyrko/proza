// The inspector on the right of the window: Document (file, text settings, counts, character), Outline (filterable
// list) and Warnings (incompatible characters, inconsistent line endings). Panes only compute while visible.
// Also the Character Inspector popover (Text ▸ Inspect Character).

use crate::document::{self, LineEnding};
use crate::outline::Kind;
use crate::settings;
use crate::unicode;
use crate::window::{count_lines, group, human_size, DocWindow};
use adw::prelude::*;
use gtk::{gio, glib, pango};
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use unicode_segmentation::UnicodeSegmentation;

const INCOMPATIBLE_TAG: &str = "proza-incompatible";

pub struct Inspector {
    root: gtk::Box,
    stack: gtk::Stack,
    buttons: Vec<gtk::ToggleButton>,
    // Document
    file: Vec<gtk::Label>,   // created, modified, size, permissions, owner, full path
    text: Vec<gtk::Label>,   // encoding, line endings, mode
    counts: Vec<gtk::Label>, // lines, characters, words, location, line, column
    character: gtk::Box,
    // Outline
    outline_list: gtk::ListBox,
    outline_filter: gtk::SearchEntry,
    outline_rows: RefCell<Vec<usize>>,
    // Warnings
    incompatible_status: gtk::Label,
    incompatible_list: gtk::ListBox,
    incompatible: RefCell<Vec<(usize, char)>>,
    endings_status: gtk::Label,
    endings_list: gtk::ListBox,
    endings: RefCell<Vec<(usize, LineEnding)>>,
    warnings_id: RefCell<Option<glib::SourceId>>,
}

const PANES: [(&str, &str, &str); 3] = [
    ("document", "Document Inspector", "x-office-document-symbolic"),
    ("outline", "Outline", "view-list-bullet-symbolic"),
    ("warnings", "Warnings", "dialog-warning-symbolic"),
];

fn value_label() -> gtk::Label {
    let l = gtk::Label::builder().label("–").xalign(0.0).selectable(true).wrap(true).wrap_mode(pango::WrapMode::WordChar).build();
    l.add_css_class("caption");
    l
}

fn caption_heading(text: &str) -> gtk::Label {
    let l = gtk::Label::builder().label(text).xalign(0.0).build();
    l.add_css_class("caption-heading");
    l.add_css_class("dim-label");
    l
}

/// A collapsible section with label → value rows.
fn section(title: &str, rows: &[&str]) -> (gtk::Expander, Vec<gtk::Label>, gtk::Grid) {
    let grid = gtk::Grid::builder().column_spacing(8).row_spacing(3).margin_top(4).margin_start(4).build();
    let mut values = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let k = gtk::Label::builder().label(format!("{r}:")).xalign(1.0).yalign(0.0).build();
        k.add_css_class("caption");
        k.add_css_class("dim-label");
        let v = value_label();
        v.set_hexpand(true);
        grid.attach(&k, 0, i as i32, 1, 1);
        grid.attach(&v, 1, i as i32, 1, 1);
        values.push(v);
    }
    let exp = gtk::Expander::builder().label_widget(&caption_heading(title)).expanded(true).child(&grid).build();
    (exp, values, grid)
}

fn margins(w: &impl IsA<gtk::Widget>, m: i32) {
    w.set_margin_top(m);
    w.set_margin_bottom(m);
    w.set_margin_start(m);
    w.set_margin_end(m);
}

pub fn attach(w: &Rc<DocWindow>) {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("inspector");
    root.set_size_request(240, -1);
    let switcher = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    switcher.add_css_class("linked");
    switcher.set_halign(gtk::Align::Center);
    switcher.set_margin_top(8);
    switcher.set_margin_bottom(8);
    let stack = gtk::Stack::new();
    stack.set_vexpand(true);
    let mut buttons: Vec<gtk::ToggleButton> = Vec::new();
    for (_, tip, icon) in PANES {
        let b = gtk::ToggleButton::builder().icon_name(icon).tooltip_text(tip).build();
        b.set_focusable(false);
        if let Some(first) = buttons.first() {
            b.set_group(Some(first));
        }
        switcher.append(&b);
        buttons.push(b);
    }
    root.append(&switcher);
    root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    // ---- Document
    let doc = gtk::Box::new(gtk::Orientation::Vertical, 8);
    margins(&doc, 12);
    let (file_exp, file, file_grid) = section("File", &["Created", "Modified", "Size", "Permissions", "Owner", "Full Path"]);
    let reveal = gtk::Button::builder().icon_name("go-next-symbolic").tooltip_text("Show in Files").valign(gtk::Align::Start).build();
    reveal.add_css_class("circular");
    reveal.add_css_class("flat");
    file_grid.attach(&reveal, 2, 5, 1, 1);
    file[5].set_ellipsize(pango::EllipsizeMode::Middle);
    file[5].set_lines(5);
    let (text_exp, text, _) = section("Text Settings", &["Encoding", "Line Endings", "Mode"]);
    let (count_exp, counts, _) = section("Count", &["Lines", "Characters", "Words", "Location", "Line", "Column"]);
    let character = gtk::Box::new(gtk::Orientation::Vertical, 4);
    character.set_margin_top(4);
    let char_exp = gtk::Expander::builder().label_widget(&caption_heading("Character")).expanded(true).child(&character).build();
    for e in [&file_exp, &text_exp, &count_exp, &char_exp] {
        doc.append(e);
    }
    let doc_scroll = gtk::ScrolledWindow::builder().child(&doc).hscrollbar_policy(gtk::PolicyType::Never).build();
    stack.add_named(&doc_scroll, Some("document"));

    // ---- Outline
    let outline = gtk::Box::new(gtk::Orientation::Vertical, 6);
    margins(&outline, 8);
    outline.append(&caption_heading("Outline"));
    let outline_list = gtk::ListBox::new();
    outline_list.add_css_class("outline-menu");
    let placeholder = gtk::Label::new(Some("No Filter Results"));
    placeholder.add_css_class("dim-label");
    outline_list.set_placeholder(Some(&placeholder));
    let ol_scroll = gtk::ScrolledWindow::builder().child(&outline_list).vexpand(true).hscrollbar_policy(gtk::PolicyType::Never).build();
    outline.append(&ol_scroll);
    let outline_filter = gtk::SearchEntry::builder().placeholder_text("Filter").build();
    outline.append(&outline_filter);
    stack.add_named(&outline, Some("outline"));

    // ---- Warnings
    let warn_section = |title: &str, columns: &[&str]| {
        let bx = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let status = gtk::Label::builder().label("No issues found.").xalign(0.0).wrap(true).build();
        status.add_css_class("caption");
        status.add_css_class("dim-label");
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        for (i, c) in columns.iter().enumerate() {
            let l = gtk::Label::builder().label(*c).xalign(if i == 0 { 1.0 } else { 0.0 }).width_chars(if i == 0 { 5 } else { 8 }).build();
            l.add_css_class("caption");
            l.add_css_class("dim-label");
            head.append(&l);
        }
        let list = gtk::ListBox::new();
        let sc = gtk::ScrolledWindow::builder().child(&list).vexpand(true).min_content_height(60).build();
        sc.add_css_class("frame");
        bx.append(&caption_heading(title));
        bx.append(&status);
        bx.append(&head);
        bx.append(&sc);
        (bx, status, list)
    };
    let (inc_box, incompatible_status, incompatible_list) = warn_section("Incompatible Characters", &["Line", "Character", "Converted"]);
    let (end_box, endings_status, endings_list) = warn_section("Inconsistent Line Endings", &["Line", "Line Ending"]);
    let warnings = gtk::Paned::builder().orientation(gtk::Orientation::Vertical).start_child(&inc_box).end_child(&end_box).build();
    // the two sections share the height evenly at first
    let done = std::cell::Cell::new(false);
    warnings.connect_max_position_notify(move |p| {
        if !done.get() && p.max_position() > 0 {
            done.set(true);
            p.set_position(p.max_position() / 2);
        }
    });
    warnings.set_margin_top(16);
    warnings.set_margin_start(12);
    warnings.set_margin_end(12);
    warnings.set_margin_bottom(12);
    stack.add_named(&warnings, Some("warnings"));
    root.append(&stack);

    let insp = Rc::new(Inspector {
        root,
        stack,
        buttons,
        file,
        text,
        counts,
        character,
        outline_list,
        outline_filter,
        outline_rows: RefCell::new(Vec::new()),
        incompatible_status,
        incompatible_list,
        incompatible: RefCell::new(Vec::new()),
        endings_status,
        endings_list,
        endings: RefCell::new(Vec::new()),
        warnings_id: RefCell::new(None),
    });
    w.split.set_sidebar(Some(&insp.root));
    w.inspector.replace(Some(insp.clone()));

    // pane buttons ↔ the window's inspector-pane action
    for (i, b) in insp.buttons.iter().enumerate() {
        let weak = Rc::downgrade(w);
        b.connect_toggled(move |b| {
            if let (true, Some(w)) = (b.is_active(), weak.upgrade()) {
                let current = w.actions.action_state("inspector-pane").and_then(|v| v.get::<String>());
                if current.as_deref() != Some(i.to_string().as_str()) {
                    w.actions.activate_action("inspector-pane", Some(&i.to_string().to_variant()));
                }
            }
        });
    }
    show_pane(w, settings::get().inspector_pane as usize);
    let weak = Rc::downgrade(w);
    w.split.connect_show_sidebar_notify(move |_| {
        if let Some(w) = weak.upgrade() {
            refresh(&w);
        }
    });
    let weak = Rc::downgrade(w);
    reveal.connect_clicked(move |_| {
        if let Some(f) = weak.upgrade().and_then(|w| w.doc.borrow().file.clone()) {
            crate::app::show_in_files(&f);
        }
    });
    let weak = Rc::downgrade(w);
    insp.outline_filter.connect_search_changed(move |_| {
        if let Some(w) = weak.upgrade() {
            fill_outline(&w);
        }
    });
    let weak = Rc::downgrade(w);
    insp.outline_list.connect_row_activated(move |_, row| {
        let Some(w) = weak.upgrade() else { return };
        let Some(n) = inspector(&w).and_then(|i| i.outline_rows.borrow().get(row.index() as usize).copied()) else { return };
        w.select_outline_item(n);
        w.view().scroll_to_mark(&w.buffer.get_insert(), 0.0, true, 0.0, 0.5);
    });
    let weak = Rc::downgrade(w);
    insp.incompatible_list.connect_row_activated(move |_, row| {
        let Some(w) = weak.upgrade() else { return };
        let Some(at) = inspector(&w).and_then(|i| i.incompatible.borrow().get(row.index() as usize).map(|(o, _)| *o)) else { return };
        select_and_centre(&w, at, at + 1);
    });
    let weak = Rc::downgrade(w);
    insp.endings_list.connect_row_activated(move |_, row| {
        let Some(w) = weak.upgrade() else { return };
        let Some((at, e)) = inspector(&w).and_then(|i| i.endings.borrow().get(row.index() as usize).copied()) else { return };
        select_and_centre(&w, at, at + if e == LineEnding::CrLf { 2 } else { 1 });
    });
}

fn inspector(w: &DocWindow) -> Option<Rc<Inspector>> {
    w.inspector.borrow().clone()
}

fn select_and_centre(w: &DocWindow, a: usize, z: usize) {
    let b = &w.buffer;
    b.select_range(&b.iter_at_offset(a as i32), &b.iter_at_offset(z as i32));
    let v = w.view();
    v.scroll_to_mark(&b.get_insert(), 0.0, true, 0.0, 0.5);
    v.grab_focus();
}

fn current_pane(w: &DocWindow) -> Option<usize> {
    if !w.split.shows_sidebar() {
        return None;
    }
    let i = inspector(w)?;
    let name = i.stack.visible_child_name()?;
    PANES.iter().position(|(n, _, _)| name == *n)
}

/// Switches to pane `n` (called by the window's inspector-pane action).
pub fn show_pane(w: &Rc<DocWindow>, n: usize) {
    let Some(i) = inspector(w) else { return };
    let n = n.min(2);
    i.stack.set_visible_child_name(PANES[n].0);
    if !i.buttons[n].is_active() {
        i.buttons[n].set_active(true);
    }
    refresh(w);
}

/// Recomputes the visible pane; clears what a hidden Warnings pane put into the editor.
pub fn refresh(w: &Rc<DocWindow>) {
    let pane = current_pane(w);
    if pane != Some(2) {
        clear_incompatible_highlight(w);
    }
    match pane {
        Some(0) => {
            update_file(w);
            update_document(w);
        }
        Some(1) => fill_outline(w),
        Some(2) => scan_warnings(w),
        _ => {}
    }
}

// ---------------------------------------------------------------- Document

fn permissions(mode: u32) -> String {
    let mut s = String::new();
    for (shift, special, set, unset) in [(6, 0o4000, 's', 'S'), (3, 0o2000, 's', 'S'), (0, 0o1000, 't', 'T')] {
        let bits = (mode >> shift) & 7;
        s.push(if bits & 4 != 0 { 'r' } else { '-' });
        s.push(if bits & 2 != 0 { 'w' } else { '-' });
        s.push(match (mode & special != 0, bits & 1 != 0) {
            (true, true) => set,
            (true, false) => unset,
            (false, true) => 'x',
            (false, false) => '-',
        });
    }
    let octal = if mode & 0o7000 != 0 { format!("{:04o}", mode & 0o7777) } else { format!("{:03o}", mode & 0o777) };
    format!("{octal} (-{s})")
}

/// File and Text Settings sections (on showing the pane, after saving and after format changes).
pub fn update_file(w: &DocWindow) {
    if current_pane(w) != Some(0) {
        return;
    }
    let Some(i) = inspector(w) else { return };
    let file = w.doc.borrow().file.clone();
    let info = file.as_ref().and_then(|f| {
        f.query_info("time::created,time::modified,standard::size,unix::mode,owner::user", gio::FileQueryInfoFlags::NONE, gio::Cancellable::NONE).ok()
    });
    let date = |dt: Option<glib::DateTime>| {
        dt.and_then(|d| d.to_local().ok()).and_then(|d| d.format("%e %b %Y, %H:%M").ok()).map(|s| s.trim().to_string()).unwrap_or_else(|| "–".into())
    };
    let values: [String; 6] = match &info {
        Some(info) => {
            let size = info.size().max(0) as usize;
            [
                date(Some(info.attribute_uint64("time::created")).filter(|t| *t > 0).and_then(|t| glib::DateTime::from_unix_utc(t as i64).ok())),
                date(info.modification_date_time()),
                format!("{} ({} bytes)", human_size(size), group(size)),
                if info.has_attribute("unix::mode") { permissions(info.attribute_uint32("unix::mode")) } else { "–".into() },
                info.attribute_string("owner::user").map(|s| s.to_string()).unwrap_or_else(|| "–".into()),
                file.as_ref().and_then(|f| f.path()).map(|p| p.display().to_string()).unwrap_or_else(|| "–".into()),
            ]
        }
        None => std::array::from_fn(|_| "–".to_string()),
    };
    for (l, v) in i.file.iter().zip(values) {
        l.set_tooltip_text(Some(&v));
        l.set_label(&v);
    }
    let d = w.doc.borrow();
    i.text[0].set_label(&d.encoding.label());
    i.text[1].set_label(d.ending.short());
    let lang = w.buffer.language().map(|l| l.id().to_string()).unwrap_or_default();
    i.text[2].set_label(if crate::window::GENERAL_SYNTAXES.contains(&lang.as_str()) { "General" } else { "Code" });
}

/// Counts and the character section; called on every status update while the Document pane is visible.
pub fn update_document(w: &DocWindow) {
    if current_pane(w) != Some(0) {
        return;
    }
    let Some(i) = inspector(w) else { return };
    let b = &w.buffer;
    let text = b.text(&b.start_iter(), &b.end_iter(), false);
    let (s, e) = b.selection_bounds().unwrap_or_else(|| {
        let c = b.iter_at_mark(&b.get_insert());
        (c, c)
    });
    let sel = b.text(&s, &e, false);
    let with = |t: usize, n: usize| if n > 0 { format!("{} ({})", group(t), group(n)) } else { group(t) };
    let sel_lines = if sel.is_empty() {
        0
    } else {
        ((if e.starts_line() && e.line() > s.line() { e.line() - 1 } else { e.line() }) - s.line() + 1) as usize
    };
    let mut ls = s;
    ls.set_line_offset(0);
    let values = [
        with(count_lines(&text), sel_lines),
        with(text.graphemes(true).count(), sel.graphemes(true).count()),
        with(text.unicode_words().count(), sel.unicode_words().count()),
        group(b.text(&b.start_iter(), &s, false).graphemes(true).count()),
        group(s.line() as usize + 1),
        group(b.text(&ls, &s, false).graphemes(true).count()),
    ];
    for (l, v) in i.counts.iter().zip(values) {
        l.set_label(&v);
    }
    // Character: exactly one grapheme cluster selected
    while let Some(c) = i.character.first_child() {
        i.character.remove(&c);
    }
    let graphemes: Vec<&str> = sel.graphemes(true).collect();
    if graphemes.len() != 1 {
        let l = gtk::Label::builder().label("Not selected").xalign(0.0).tooltip_text("Select a single character to show Unicode information.").build();
        l.add_css_class("dim-label");
        l.add_css_class("caption");
        i.character.append(&l);
        return;
    }
    let scalars: Vec<char> = graphemes[0].chars().collect();
    let grid = gtk::Grid::builder().column_spacing(8).row_spacing(3).build();
    let chips = gtk::FlowBox::builder().selection_mode(gtk::SelectionMode::None).max_children_per_line(4).build();
    for c in &scalars {
        let chip = gtk::Label::new(Some(&format!("U+{:04X}", *c as u32)));
        chip.add_css_class("code-point-chip");
        chip.add_css_class("caption");
        chips.insert(&chip, -1);
    }
    let mut rows: Vec<(&str, gtk::Widget)> = vec![(if scalars.len() > 1 { "Code Points" } else { "Code Point" }, chips.upcast())];
    if let [c] = scalars[..] {
        let label = |s: String| {
            let l = value_label();
            l.set_label(&s);
            l.upcast::<gtk::Widget>()
        };
        rows.push(("Name", label(unicode::name(c).unwrap_or_else(|| "Unknown".into()))));
        rows.push(("Block", label(unicode::block(c).unwrap_or("No Block").to_string())));
        rows.push(("Category", label(unicode::category(c))));
    }
    for (n, (k, v)) in rows.into_iter().enumerate() {
        let kl = gtk::Label::builder().label(format!("{k}:")).xalign(1.0).yalign(0.0).build();
        kl.add_css_class("caption");
        kl.add_css_class("dim-label");
        v.set_hexpand(true);
        grid.attach(&kl, 0, n as i32, 1, 1);
        grid.attach(&v, 1, n as i32, 1, 1);
    }
    i.character.append(&grid);
}

// ---------------------------------------------------------------- Outline

/// CotEditor's abbreviation filter: the filter's characters appear in order, case-insensitively. Returns the
/// matched char positions.
pub fn abbreviation_match(title: &str, filter: &str) -> Option<Vec<usize>> {
    let mut want = filter.chars().flat_map(char::to_lowercase).filter(|c| !c.is_whitespace()).peekable();
    let mut hits = Vec::new();
    for (i, c) in title.chars().enumerate() {
        let Some(&w) = want.peek() else { break };
        if c.to_lowercase().eq(std::iter::once(w)) {
            hits.push(i);
            want.next();
        }
    }
    want.peek().is_none().then_some(hits)
}

pub fn outline_changed(w: &Rc<DocWindow>) {
    if current_pane(w) == Some(1) {
        fill_outline(w);
    }
}

fn fill_outline(w: &DocWindow) {
    let Some(i) = inspector(w) else { return };
    while let Some(row) = i.outline_list.row_at_index(0) {
        i.outline_list.remove(&row);
    }
    let filter = i.outline_filter.text().to_string();
    let items = w.outline.borrow();
    let mut rows = Vec::new();
    for (n, item) in items.iter().enumerate() {
        if item.kind == Kind::Separator {
            if filter.is_empty() {
                let row = gtk::ListBoxRow::builder().selectable(false).activatable(false).child(&gtk::Separator::new(gtk::Orientation::Horizontal)).build();
                i.outline_list.append(&row);
                rows.push(n);
            }
            continue;
        }
        let hits = if filter.is_empty() { Some(Vec::new()) } else { abbreviation_match(&item.title, &filter) };
        let Some(hits) = hits else { continue };
        let bx = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        bx.set_margin_start(4 + 14 * item.level as i32);
        let (txt, class) = item.kind.badge();
        let badge = gtk::Label::new(Some(&txt));
        badge.add_css_class("outline-badge");
        badge.add_css_class(class);
        bx.append(&badge);
        let title: String = item
            .title
            .chars()
            .enumerate()
            .map(|(k, c)| {
                let e = glib::markup_escape_text(&c.to_string()).to_string();
                if hits.contains(&k) {
                    format!("<span background='#ffd60a' foreground='#1d1d1d'>{e}</span>")
                } else {
                    e
                }
            })
            .collect();
        let indent = glib::markup_escape_text(&item.indent.replace('\t', "    ")).to_string();
        let label = gtk::Label::builder().xalign(0.0).ellipsize(pango::EllipsizeMode::End).build();
        label.set_markup(&format!("{indent}{title}"));
        if item.kind.bold() {
            label.add_css_class("heading");
        }
        bx.append(&label);
        i.outline_list.append(&bx);
        rows.push(n);
    }
    *i.outline_rows.borrow_mut() = rows;
    drop(items);
    select_current_outline(w);
}

/// The row of the item containing the caret is selected as the caret moves.
pub fn select_current_outline(w: &DocWindow) {
    if current_pane(w) != Some(1) {
        return;
    }
    let Some(i) = inspector(w) else { return };
    let b = &w.buffer;
    let caret = b.iter_at_mark(&b.get_insert()).offset() as usize;
    let current = crate::outline::current(&w.outline.borrow(), caret);
    let row = current.and_then(|n| i.outline_rows.borrow().iter().position(|r| *r == n));
    match row.and_then(|r| i.outline_list.row_at_index(r as i32)) {
        Some(r) => i.outline_list.select_row(Some(&r)),
        None => i.outline_list.unselect_all(),
    }
}

// ---------------------------------------------------------------- Warnings

/// Re-scans 0.3 s after edits (and on encoding changes) while the Warnings pane is visible.
pub fn schedule_warnings(w: &Rc<DocWindow>) {
    if current_pane(w) != Some(2) {
        return;
    }
    let Some(i) = inspector(w) else { return };
    if let Some(id) = i.warnings_id.borrow_mut().take() {
        id.remove();
    }
    let weak = Rc::downgrade(w);
    let id = glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
        if let Some(w) = weak.upgrade() {
            if let Some(i) = inspector(&w) {
                i.warnings_id.borrow_mut().take();
            }
            scan_warnings(&w);
        }
    });
    i.warnings_id.replace(Some(id));
}

fn clear_incompatible_highlight(w: &DocWindow) {
    let b = &w.buffer;
    if let Some(t) = b.tag_table().lookup(INCOMPATIBLE_TAG) {
        b.remove_tag(&t, &b.start_iter(), &b.end_iter());
    }
}

fn warning_row(cells: &[(String, bool)]) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (n, (text, dim)) in cells.iter().enumerate() {
        let l = gtk::Label::builder().label(text).xalign(if n == 0 { 1.0 } else { 0.0 }).width_chars(if n == 0 { 5 } else { 8 }).build();
        l.add_css_class("caption");
        if n == 0 {
            l.add_css_class("numeric");
        }
        if *dim {
            l.add_css_class("dim-label");
        }
        row.append(&l);
    }
    row
}

fn set_status(l: &gtk::Label, text: &str, ok: bool) {
    l.set_label(text);
    if ok {
        l.add_css_class("dim-label");
    } else {
        l.remove_css_class("dim-label");
    }
}

/// The encoding or line ending changed: the Warnings pane rescans at once.
pub fn format_changed(w: &DocWindow) {
    update_file(w);
    if current_pane(w) == Some(2) {
        scan_warnings(w);
    }
}

fn scan_warnings(w: &DocWindow) {
    let Some(i) = inspector(w) else { return };
    let b = &w.buffer;
    let text = w.text();
    let enc = w.doc.borrow().encoding.clone();
    for list in [&i.incompatible_list, &i.endings_list] {
        while let Some(r) = list.row_at_index(0) {
            list.remove(&r);
        }
    }
    // incompatible characters (no scan for Unicode encodings or an empty document)
    let found = if text.is_empty() || enc.iana.starts_with("UTF") { Vec::new() } else { document::incompatible_chars(&text, &enc) };
    clear_incompatible_highlight(w);
    let tag = b.tag_table().lookup(INCOMPATIBLE_TAG).unwrap_or_else(|| {
        let t = gtk::TextTag::new(Some(INCOMPATIBLE_TAG));
        t.set_background_rgba(Some(&gtk::gdk::RGBA::new(0.5, 0.5, 0.5, 0.3)));
        b.tag_table().add(&t);
        t
    });
    for (at, c) in &found {
        let it = b.iter_at_offset(*at as i32);
        let mut end = it;
        end.forward_char();
        b.apply_tag(&tag, &it, &end);
        let (shown, dim) = if c.is_control() || c.is_whitespace() { (format!("U+{:04X}", *c as u32), true) } else { (c.to_string(), false) };
        let converted = String::from_utf8_lossy(&document::encode(&c.to_string(), &enc)).to_string();
        i.incompatible_list.append(&warning_row(&[(group(it.line() as usize + 1), false), (shown, dim), (converted, false)]));
    }
    let status = match found.len() {
        0 => "No issues found.".to_string(),
        1 => "Found an incompatible character.".to_string(),
        n => format!("Found {} incompatible characters.", group(n)),
    };
    set_status(&i.incompatible_status, &status, found.is_empty());
    *i.incompatible.borrow_mut() = found;

    // line endings other than the document's
    let ending = w.doc.borrow().ending;
    let mut others = Vec::new();
    let (mut byte, mut chars) = (0usize, 0usize);
    for (at, e) in document::line_endings(&text) {
        if e == ending {
            continue;
        }
        chars += text[byte..at].chars().count();
        byte = at;
        others.push((chars, e));
    }
    for (at, e) in &others {
        let line = b.iter_at_offset(*at as i32).line() as usize + 1;
        i.endings_list.append(&warning_row(&[(group(line), false), (e.short().to_string(), false)]));
    }
    let status = match others.len() {
        0 => "No issues found.".to_string(),
        1 => format!("Found a line ending other than {}.", ending.short()),
        n => format!("Found {} line endings other than {}.", group(n), ending.short()),
    };
    set_status(&i.endings_status, &status, others.is_empty());
    *i.endings.borrow_mut() = others;
}

// ---------------------------------------------------------------- Character Inspector (Text ▸ Inspect Character)

/// A popover under the single selected character: the glyph large, its name, code point, block, category and
/// Unicode version (one row per scalar for clusters).
pub fn inspect_character(w: &Rc<DocWindow>) {
    let b = &w.buffer;
    let Some((s, e)) = b.selection_bounds() else {
        w.win.error_bell();
        return;
    };
    let sel = b.text(&s, &e, false).to_string();
    let mut graphemes = sel.graphemes(true);
    let (Some(g), None) = (graphemes.next(), graphemes.next()) else {
        w.win.error_bell();
        return;
    };
    let view = w.view();
    let rect = view.iter_location(&s);
    let (x, y) = view.buffer_to_window_coords(gtk::TextWindowType::Widget, rect.x(), rect.y());
    let pop = gtk::Popover::builder().position(gtk::PositionType::Bottom).build();
    pop.set_parent(&view);
    pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x, y, rect.width().max(8), rect.height())));

    let scalars: Vec<char> = g.chars().collect();
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    margins(&row, 14);
    let (shown, dim) = if scalars.len() == 1 { unicode::picture(scalars[0]) } else { (g.to_string(), false) };
    let glyph = gtk::Label::new(Some(&shown));
    glyph.add_css_class("character-glyph");
    glyph.set_width_request(64);
    if dim {
        glyph.add_css_class("dim-label");
    }
    row.append(&glyph);
    let details = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let title = gtk::Label::builder().xalign(0.0).selectable(true).wrap(true).max_width_chars(30).build();
    details.append(&title);
    let grid = gtk::Grid::builder().column_spacing(8).row_spacing(3).build();
    let add = |n: i32, k: &str, v: String| {
        let kl = gtk::Label::builder().label(format!("{k}:")).xalign(1.0).build();
        kl.add_css_class("dim-label");
        kl.add_css_class("caption");
        let vl = gtk::Label::builder().label(v).xalign(0.0).selectable(true).build();
        vl.add_css_class("caption");
        grid.attach(&kl, 0, n, 1, 1);
        grid.attach(&vl, 1, n, 1, 1);
    };
    if let [c] = scalars[..] {
        match unicode::name(c) {
            Some(n) => {
                title.set_label(&n);
                title.add_css_class("heading");
            }
            None => {
                title.set_label("Unknown");
                title.add_css_class("dim-label");
            }
        }
        add(0, "Code Point", unicode::code_point(c));
        add(1, "Block", unicode::block(c).unwrap_or("No Block").to_string());
        add(2, "Category", unicode::category(c));
        add(3, "Version", unicode::version(c).unwrap_or_else(|| "–".into()));
    } else {
        title.set_label(&format!("<a letter consisting of {} characters>", scalars.len()));
        for (n, c) in scalars.iter().enumerate() {
            add(n as i32, &format!("U+{:04X}", *c as u32), unicode::name(*c).unwrap_or_else(|| "Unknown".into()));
        }
    }
    details.append(&grid);
    row.append(&details);
    pop.set_child(Some(&row));
    pop.connect_closed(|p| {
        let p = p.clone();
        glib::idle_add_local_once(move || p.unparent());
    });
    pop.popup();
    // a selectable label taking the focus would select all of its text
    glib::idle_add_local_once(move || title.select_region(0, 0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_and_filter() {
        assert_eq!(permissions(0o644), "644 (-rw-r--r--)");
        assert_eq!(permissions(0o4755), "4755 (-rwsr-xr-x)");
        assert_eq!(abbreviation_match("selectOutlineItem", "soi"), Some(vec![0, 6, 10]));
        assert_eq!(abbreviation_match("open", "x"), None);
    }
}
