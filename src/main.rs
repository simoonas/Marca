mod app;
mod components;
mod db;
mod fetch_metadata;
mod fixtures;
pub mod import;
mod icon_names {
    pub use shipped::*; // Include all shipped icons by default
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}
use relm4::RelmApp;
use adw::prelude::*;

fn main() {
    relm4_icons::initialize_icons(icon_names::GRESOURCE_BYTES, icon_names::RESOURCE_PREFIX);
    // Initialize database
    let db = db::Database::new().expect("Failed to open database");

    // Load sample data if database is empty
    if db.is_empty().unwrap_or(false) {
        eprintln!("Database is empty, loading sample data...");
        fixtures::load_sample_data(&db).expect("Failed to load sample data");
    } else {
        eprintln!("Database already contains data");
    }

    // Run garbage collection
    let schema_exists = adw::gio::SettingsSchemaSource::default()
        .and_then(|s| s.lookup("com.marca.app", true))
        .is_some();

    if schema_exists {
        let settings = adw::gio::Settings::new("com.marca.app");
        let gc_days = settings.int("gc-days");
        if gc_days > 0 {
            if let Ok(deleted_count) = db.gc_deleted_bookmarks(gc_days as u32) {
                if deleted_count > 0 {
                    eprintln!("Garbage collected {} deleted bookmarks older than {} days", deleted_count, gc_days);
                }
            }
        }
    } else {
        eprintln!("GSettings schema 'com.marca.app' not found. Skipping garbage collection.");
    }

    // Run the application
    let app = RelmApp::new("com.marca.app");
    app.run::<app::App>(db);
}
