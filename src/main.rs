mod app;
mod components;
mod db;
mod fetch_metadata;
pub mod import;
mod icon_names {
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}
use adw::prelude::*;
use clap::Parser;
use relm4::{RelmApp, main_application};

#[derive(Parser)]
#[command(name = "marca", version)]
struct Cli {
    #[arg(long)]
    add: Option<String>,
}

fn main() {
    relm4_icons::initialize_icons(icon_names::GRESOURCE_BYTES, icon_names::RESOURCE_PREFIX);

    // We use a custom app initializer to handle the command line before RelmApp runs.
    let gtk_app = main_application();
    gtk_app.set_flags(adw::gio::ApplicationFlags::HANDLES_COMMAND_LINE);

    gtk_app.connect_command_line(move |app, cmdline| {
        let args = cmdline.arguments();

        let cli = Cli::try_parse_from(args.iter().map(|p| p.to_string_lossy().into_owned())).ok();

        let is_ui_visible = app.windows().iter().any(|w| w.is_visible());

        if let Some(Cli { add: Some(ref url) }) = cli {
            if is_ui_visible {
                if let Some(sender) = app::APP_SENDER.get() {
                    let _ = sender.send(app::AppMsg::AddBookmarkFromCli(url.clone()));
                }
            } else {
                let app_clone = app.clone();
                let url = url.clone();
                adw::glib::MainContext::default().spawn_local(async move {
                    if let Err(e) = app::process_background_bookmark(&url, None).await {
                        eprintln!("Background bookmark error: {}", e);
                    }
                    app_clone.quit();
                });
            }
        } else {
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

    // Seed sample data if database is empty
    if db.count_bookmarks().unwrap_or(0) == 0 {
        db::seed::seed_sample_data(&db).expect("Failed to seed sample data");
    }

    // Run the application
    let relm_app = RelmApp::new("io.github.simoonas.marca");
    relm_app.run::<app::App>(db);
}
