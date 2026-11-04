// The inspector on the right of the window (Document, Outline, Warnings). Filled in with the inspector phase.

use gtk::prelude::*;

pub fn placeholder() -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
    b.add_css_class("inspector");
    b.set_size_request(240, -1);
    b
}

/// The document's outline changed (the Outline pane follows it).
pub fn outline_changed(_w: &std::rc::Rc<crate::window::DocWindow>) {}
