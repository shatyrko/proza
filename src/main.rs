// Proza: a lightweight plain-text editor for Linux, modelled on CotEditor's interface.

mod window;

use adw::prelude::*;

const APP_ID: &str = "dev.alecsh.Proza";

fn main() -> gtk::glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gtk::gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    app.connect_startup(|_| sourceview5::init());   // after GTK is up
    app.connect_activate(|app| {
        window::Window::new(app, None).present();
    });
    app.connect_open(|app, files, _hint| {
        for f in files {
            window::Window::new(app, Some(f.clone())).present();
        }
    });
    app.run()
}
