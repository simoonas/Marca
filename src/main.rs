mod app;
mod components;
mod db;
mod fixtures;
mod fetch_metadata;
pub mod import;

use relm4::RelmApp;

fn main() {
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
