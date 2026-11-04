// Editor panes: each split editor has its navigation bar (close split · ⌃ ⌄ outline popup · split button) above
// its text view. Up to four editors share the document's buffer, stacked top to bottom or side by side.

use crate::outline::{self, Kind};
use crate::settings;
use crate::window::{DocWindow, EditorPane};
use adw::prelude::*;
use gtk::{gdk, gio, glib, pango};
use sourceview5::prelude::*;
use std::rc::Rc;

const MAX_EDITORS: usize = 4;

fn small_flat(b: &impl IsA<gtk::Widget>) {
    b.add_css_class("flat");
    b.set_focusable(false);
}

/// The kind badge for outline rows: CotEditor's coloured symbol squares as small coloured letters.
fn badge(kind: Kind) -> gtk::Label {
    let (text, class) = kind.badge();
    let l = gtk::Label::new(Some(&text));
    l.add_css_class("outline-badge");
    l.add_css_class(class);
    l
}

pub fn install(w: &Rc<DocWindow>) {
    w.add_action("split-editor", |w| {
        let at = w.active_editor.get();
        w.split_editor(at);
    });
    w.add_action("close-split", |w| {
        let at = w.active_editor.get();
        w.close_editor(at);
    });
    w.add_toggle("split-orientation", settings::get().split_vertical, |w, on| {
        settings::update(|s| s.split_vertical = on);
        w.layout_editors();
    });
    w.add_action("focus-next-editor", |w| w.focus_editor_step(1));
    w.add_action("focus-previous-editor", |w| w.focus_editor_step(-1));
    w.add_action("outline-previous", |w| w.jump_outline(false));
    w.add_action("outline-next", |w| w.jump_outline(true));
    w.add_action("open-outline", |w| {
        if w.outline.borrow().is_empty() {
            w.win.error_bell();
            return;
        }
        if !settings::get().show_navigation_bar {
            gtk::prelude::ActionGroupExt::activate_action(&w.actions, "navigation-bar", None);
        }
        let i = w.active_editor.get();
        if let Some(btn) = w.editors.borrow().get(i).map(|e| e.outline_btn.clone()) {
            // after the menu that ran this has closed
            glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || btn.popup());
        }
    });
}

impl DocWindow {
    pub fn make_editor(self: &Rc<Self>) -> EditorPane {
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
        crate::typing::attach(self, &view);

        let nav = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        nav.add_css_class("navigation-bar");
        let close_btn = gtk::Button::builder().icon_name("window-close-symbolic").tooltip_text("Close split editor").build();
        small_flat(&close_btn);
        close_btn.set_opacity(0.0);
        close_btn.set_can_target(false);
        let divider = gtk::Separator::new(gtk::Orientation::Vertical);
        divider.set_visible(false);
        let prev_btn = gtk::Button::builder().icon_name("go-up-symbolic").tooltip_text("Jump to previous outline item").build();
        let next_btn = gtk::Button::builder().icon_name("go-down-symbolic").tooltip_text("Jump to next outline item").build();
        small_flat(&prev_btn);
        small_flat(&next_btn);
        let outline_label = gtk::Label::builder().ellipsize(pango::EllipsizeMode::End).max_width_chars(48).build();
        let outline_child = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        outline_child.append(&outline_label);
        outline_child.append(&gtk::Image::from_icon_name("proza-updown-symbolic"));
        let outline_list = gtk::ListBox::new();
        outline_list.add_css_class("outline-menu");
        let list_scroller = gtk::ScrolledWindow::builder()
            .child(&outline_list)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(600)
            .build();
        let popover = gtk::Popover::builder().child(&list_scroller).has_arrow(false).build();
        popover.add_css_class("menu");
        let outline_btn = gtk::MenuButton::builder().child(&outline_child).popover(&popover).build();
        outline_btn.add_css_class("flat");
        outline_btn.add_css_class("outline-popup");
        outline_btn.set_focusable(false);
        if let Some(inner) = outline_btn.first_child() {
            inner.set_focusable(false);
        }
        let outline_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        outline_box.append(&prev_btn);
        outline_box.append(&next_btn);
        outline_box.append(&outline_btn);
        outline_box.set_visible(false);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        let split_btn = gtk::Button::builder().icon_name("proza-split-add-symbolic").tooltip_text("Split editor").build();
        small_flat(&split_btn);
        nav.append(&close_btn);
        nav.append(&divider);
        nav.append(&outline_box);
        nav.append(&spacer);
        nav.append(&split_btn);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&nav);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        root.append(&scroller);
        nav.set_visible(settings::get().show_navigation_bar);

        // which pane is this: found by its view, since panes are inserted and removed
        let index_of = {
            let weak = Rc::downgrade(self);
            let v = view.clone();
            move || weak.upgrade().and_then(|me| me.editors.borrow().iter().position(|e| e.view == v).map(|i| (me.clone(), i)))
        };
        {
            let index_of = index_of.clone();
            split_btn.connect_clicked(move |_| {
                if let Some((me, i)) = index_of() {
                    me.split_editor(i);
                }
            });
        }
        {
            let index_of = index_of.clone();
            close_btn.connect_clicked(move |_| {
                if let Some((me, i)) = index_of() {
                    me.close_editor(i);
                }
            });
        }
        // right-click on the split button: the orientation toggle
        {
            let menu = gio::Menu::new();
            menu.append(Some("Stack Editors Vertically"), Some("win.split-orientation"));
            let pop = gtk::PopoverMenu::from_model(Some(&menu));
            pop.set_parent(&split_btn);
            pop.set_has_arrow(false);
            let click = gtk::GestureClick::builder().button(gdk::BUTTON_SECONDARY).build();
            click.connect_pressed(move |_, _, _, _| pop.popup());
            split_btn.add_controller(click);
        }
        {
            let index_of1 = index_of.clone();
            prev_btn.connect_clicked(move |_| {
                if let Some((me, i)) = index_of1() {
                    me.active_editor.set(i);
                    me.jump_outline(false);
                }
            });
            let index_of2 = index_of.clone();
            next_btn.connect_clicked(move |_| {
                if let Some((me, i)) = index_of2() {
                    me.active_editor.set(i);
                    me.jump_outline(true);
                }
            });
        }
        {
            let index_of1 = index_of.clone();
            let list = outline_list.clone();
            popover.connect_show(move |_| {
                if let Some((me, _)) = index_of1() {
                    me.fill_outline_list(&list);
                }
            });
            let index_of = index_of.clone();
            let pop = popover.clone();
            outline_list.connect_row_activated(move |_, row| {
                pop.popdown();
                if let Some((me, i)) = index_of() {
                    let n = row.index() as usize;
                    me.active_editor.set(i);
                    me.select_outline_item(n);
                }
            });
        }
        {
            let index_of = index_of.clone();
            view.connect_has_focus_notify(move |v| {
                if let Some((me, i)) = index_of() {
                    if v.has_focus() {
                        me.active_editor.set(i);
                    }
                    me.schedule_status();
                }
            });
        }
        EditorPane { root, view, nav, close_btn, divider, outline_box, prev_btn, next_btn, outline_btn, outline_label, outline_list, split_btn }
    }

    /// Splits after editor `at`: the new editor gets the window's view settings, scrolls to the selection and
    /// takes the focus. Beeps beyond four editors.
    pub fn split_editor(self: &Rc<Self>, at: usize) {
        if self.editors.borrow().len() >= MAX_EDITORS {
            self.win.error_bell();
            return;
        }
        let pane = self.make_editor();
        let view = pane.view.clone();
        let at = (at + 1).min(self.editors.borrow().len());
        self.editors.borrow_mut().insert(at, pane);
        self.active_editor.set(at);
        self.layout_editors();
        self.apply_view_state();
        self.update_outline_nav();
        let b = self.buffer.clone();
        glib::idle_add_local_once(move || {
            view.scroll_to_mark(&b.get_insert(), 0.1, false, 0.0, 0.0);
            view.grab_focus();
        });
    }

    /// Closes editor `at` (not the last one); focus goes to the previous editor.
    pub fn close_editor(self: &Rc<Self>, at: usize) {
        if self.editors.borrow().len() <= 1 || at >= self.editors.borrow().len() {
            self.win.error_bell();
            return;
        }
        let pane = self.editors.borrow_mut().remove(at);
        detach(pane.root.upcast_ref());
        let focus = at.saturating_sub(1).min(self.editors.borrow().len() - 1);
        self.active_editor.set(focus);
        self.layout_editors();
        let view = self.editors.borrow()[focus].view.clone();
        view.grab_focus();
    }

    pub fn focus_editor_step(&self, step: i32) {
        let n = self.editors.borrow().len() as i32;
        let i = (self.active_editor.get() as i32 + step).rem_euclid(n) as usize;
        self.active_editor.set(i);
        self.editors.borrow()[i].view.grab_focus();
    }

    /// Puts the panes into the holder: one pane alone, else a chain of resizable panes in the window's orientation
    /// (side by side when "Stack Editors Vertically" is off… CotEditor's default is top to bottom).
    pub fn layout_editors(&self) {
        let eds = self.editors.borrow();
        for e in eds.iter() {
            detach(e.root.upcast_ref());
        }
        while let Some(child) = self.editors_holder.first_child() {
            self.editors_holder.remove(&child);
        }
        let side_by_side = settings::get().split_vertical;
        let orientation = if side_by_side { gtk::Orientation::Horizontal } else { gtk::Orientation::Vertical };
        let mut node: gtk::Widget = eds.last().map(|e| e.root.clone().upcast()).expect("an editor");
        for (k, e) in eds.iter().rev().skip(1).enumerate() {
            let paned = gtk::Paned::builder().orientation(orientation).wide_handle(false).shrink_start_child(false).shrink_end_child(false).build();
            paned.set_start_child(Some(&e.root));
            paned.set_end_child(Some(&node));
            // equal sizes: this pane gets 1/(panes in this subtree)
            let share = k + 2;
            let done = std::cell::Cell::new(false);
            paned.connect_max_position_notify(move |p| {
                if !done.get() && p.max_position() > 0 {
                    done.set(true);
                    p.set_position(p.max_position() / share as i32);
                }
            });
            node = paned.upcast();
        }
        self.editors_holder.append(&node);
        let several = eds.len() > 1;
        for e in eds.iter() {
            e.close_btn.set_opacity(if several { 1.0 } else { 0.0 });
            e.close_btn.set_can_target(several);
            if side_by_side {
                e.split_btn.add_css_class("rotated");
            } else {
                e.split_btn.remove_css_class("rotated");
            }
        }
        drop(eds);
        self.update_outline_nav();
    }

    // ------------------------------------------------------------------ outline

    /// Re-extracts the outline 0.3 s after the last edit (at once with `now`).
    pub fn schedule_outline(self: &Rc<Self>, now: bool) {
        if let Some(id) = self.outline_id.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(std::time::Duration::from_millis(if now { 0 } else { 300 }), move || {
            if let Some(me) = weak.upgrade() {
                me.outline_id.borrow_mut().take();
                let lang = me.buffer.language().map(|l| l.id().to_string()).unwrap_or_default();
                *me.outline.borrow_mut() = outline::extract(&lang, &me.text());
                me.update_outline_nav();
                crate::inspector::outline_changed(&me);
            }
        });
        self.outline_id.replace(Some(id));
    }

    fn selection_offsets(&self) -> (usize, usize) {
        let b = &self.buffer;
        let (s, e) = b.selection_bounds().unwrap_or_else(|| {
            let i = b.iter_at_mark(&b.get_insert());
            (i, i)
        });
        (s.offset() as usize, e.offset() as usize)
    }

    /// The navigation bars follow the caret: current item title, ⌃ ⌄ enabled when there is somewhere to go.
    pub fn update_outline_nav(&self) {
        let items = self.outline.borrow();
        let (s, e) = self.selection_offsets();
        let title = match outline::current(&items, s) {
            Some(i) => items[i].title.clone(),
            None => "No Selection".into(),
        };
        let has = !items.is_empty();
        let (prev, next) = (outline::previous(&items, s).is_some(), outline::next(&items, e).is_some());
        let several = self.editors.borrow().len() > 1;
        for p in self.editors.borrow().iter() {
            p.outline_box.set_visible(has);
            p.divider.set_visible(has && several);
            p.outline_label.set_label(&title);
            if outline::current(&items, s).is_none() {
                p.outline_label.add_css_class("dim-label");
            } else {
                p.outline_label.remove_css_class("dim-label");
            }
            p.prev_btn.set_sensitive(prev);
            p.next_btn.set_sensitive(next);
        }
    }

    fn fill_outline_list(&self, list: &gtk::ListBox) {
        while let Some(row) = list.row_at_index(0) {
            list.remove(&row);
        }
        let items = self.outline.borrow();
        let current = outline::current(&items, self.selection_offsets().0);
        for (n, item) in items.iter().enumerate() {
            let row = gtk::ListBoxRow::new();
            if item.kind == Kind::Separator {
                row.set_child(Some(&gtk::Separator::new(gtk::Orientation::Horizontal)));
                row.set_selectable(false);
                row.set_activatable(false);
            } else {
                let bx = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                bx.set_margin_start(6 + 14 * item.level as i32);
                bx.append(&badge(item.kind));
                let label = gtk::Label::builder().label(format!("{}{}", item.indent.replace('\t', "    "), item.title)).xalign(0.0).build();
                if item.kind.bold() {
                    label.add_css_class("heading");
                }
                bx.append(&label);
                row.set_child(Some(&bx));
            }
            list.append(&row);
            if Some(n) == current {
                list.select_row(Some(&row));
            }
        }
    }

    /// Selects item `n`'s title in the document and scrolls the active editor to it.
    pub fn select_outline_item(&self, n: usize) {
        let Some(item) = self.outline.borrow().get(n).cloned() else { return };
        let b = &self.buffer;
        b.select_range(&b.iter_at_offset(item.start as i32), &b.iter_at_offset(item.end as i32));
        let view = self.view();
        view.scroll_to_mark(&b.get_insert(), 0.1, false, 0.0, 0.0);
        view.grab_focus();
    }

    pub fn jump_outline(&self, forward: bool) {
        let (s, e) = self.selection_offsets();
        let target = {
            let items = self.outline.borrow();
            if forward {
                outline::next(&items, e)
            } else {
                outline::previous(&items, s)
            }
        };
        match target {
            Some(n) => self.select_outline_item(n),
            None => self.win.error_bell(),
        }
    }
}

/// Takes a pane root out of its parent (a box or one side of a paned).
fn detach(w: &gtk::Widget) {
    let Some(parent) = w.parent() else { return };
    if let Some(p) = parent.downcast_ref::<gtk::Paned>() {
        if p.start_child().as_ref() == Some(w) {
            p.set_start_child(None::<&gtk::Widget>);
        } else {
            p.set_end_child(None::<&gtk::Widget>);
        }
    } else if let Some(b) = parent.downcast_ref::<gtk::Box>() {
        b.remove(w);
    }
}
