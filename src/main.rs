// Proza: a lightweight plain-text editor for Linux, modelled on CotEditor's interface.

mod app;
mod document;
mod inspector;
mod menu;
mod settings;
mod textcmd;
mod theme;
mod typing;
mod window;

use adw::prelude::*;

const APP_ID: &str = "dev.alecsh.Proza";

fn main() -> gtk::glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gtk::gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    app.connect_startup(|app| {
        app::startup();
        app::install_actions(app);
    });
    app.connect_activate(|app| {
        if let Some(w) = app.active_window() {
            w.present();
            return;
        }
        if app::restore_backups(app) == 0 {
            app::new_document(app);
        }
    });
    app.connect_open(|app, files, _| {
        for f in files {
            app::open_file(app, f);
        }
    });
    app.run()
}
