mod app;
mod components;
mod db;
mod fetch_metadata;
pub mod import;
mod icon_names {
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}
use adw::prelude::*;
use relm4::{RelmApp, main_application};

fn main() {
    relm4_icons::initialize_icons(icon_names::GRESOURCE_BYTES, icon_names::RESOURCE_PREFIX);

    // We use a custom app initializer to handle the command line before RelmApp runs.
    let gtk_app = main_application();
    gtk_app.set_flags(adw::gio::ApplicationFlags::HANDLES_COMMAND_LINE);

    gtk_app.connect_command_line(move |app, cmdline| {
        let args = cmdline.arguments();

        let mut is_add_bookmark = false;
        let mut bookmark_text = String::new();

        // Simple CLI argument parsing
        for i in 1..args.len() {
            let arg = args[i].to_string_lossy();
            if arg == "--add" {
                if i + 1 < args.len() {
                    bookmark_text = args[i + 1].to_string_lossy().to_string();
                    is_add_bookmark = true;
                    break;
                }
            }
        }

        let is_ui_visible = app.windows().iter().any(|w| w.is_visible());

        if is_add_bookmark {
            if is_ui_visible {
                // UI is running and visible, send message to it
                if let Some(sender) = app::APP_SENDER.get() {
                    let _ = sender.send(app::AppMsg::AddBookmarkFromCli(bookmark_text));
                }
            } else {
                // UI is not visible, do it headlessly and quit
                let app_clone = app.clone();
                adw::glib::MainContext::default().spawn_local(async move {
                    if let Err(e) = app::process_background_bookmark(&bookmark_text, None).await {
                        eprintln!("Background bookmark error: {}", e);
                    }
                    app_clone.quit();
                });
            }
        } else {
            // Normal launch, just activate the window
            app.activate();
        }

        0.into()
    });

    // Initialize database
    let db = db::Database::new().expect("Failed to open database");

    // Run garbage collection
    let schema_exists = adw::gio::SettingsSchemaSource::default()
        .and_then(|s| s.lookup("io.github.simoonas.marca", true))
        .is_some();

    if schema_exists {
        let settings = adw::gio::Settings::new("io.github.simoonas.marca");
        let gc_days = settings.int("gc-days");
        if gc_days > 0
            && let Ok(deleted_count) = db.gc_deleted_bookmarks(gc_days as u32)
            && deleted_count > 0
        {
            eprintln!(
                "Garbage collected {} deleted bookmarks older than {} days",
                deleted_count, gc_days
            );
        }
    } else {
        eprintln!(
            "GSettings schema 'io.github.simoonas.marca' not found. Skipping garbage collection."
        );
    }

    // Run the application
    let relm_app = RelmApp::new("io.github.simoonas.marca");
    relm_app.run::<app::App>(db);
}
