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

    // Run the application
    let app = RelmApp::new("com.marca.app");
    app.run::<app::App>(db);
}
