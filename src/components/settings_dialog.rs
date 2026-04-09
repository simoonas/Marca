use adw::prelude::*;
use gtk::gio;
use relm4::prelude::*;

#[derive(Debug)]
pub struct SettingsDialog {
    importing: bool,
    root: adw::PreferencesDialog,
}

#[derive(Debug)]
pub enum SettingsMsg {
    ImportBookmarks,
    FileSelected(Option<std::path::PathBuf>),
    ImportComplete(Result<crate::db::ImportResult, String>),
}

#[derive(Debug)]
pub enum SettingsOutput {
    RefreshBookmarks,
    ShowToast(String),
}

#[relm4::component(pub)]
impl SimpleComponent for SettingsDialog {
    type Init = ();
    type Input = SettingsMsg;
    type Output = SettingsOutput;

    view! {
        #[root]
        adw::PreferencesDialog {
            set_title: "Settings",
            set_search_enabled: false,

            add = &adw::PreferencesPage {
                set_title: "General",
                set_icon_name: Some("preferences-system-symbolic"),

                add = &adw::PreferencesGroup {
                    set_title: "Import",
                    set_description: Some("Import bookmarks from other browsers"),

                    adw::ActionRow {
                        set_title: "Import from HTML",
                        set_subtitle: "Import bookmarks from a Netscape Bookmark Format file",

                        add_suffix = &gtk::Button {
                            set_label: "Choose File...",
                            set_valign: gtk::Align::Center,
                            add_css_class: "flat",
                            #[watch]
                            set_sensitive: !model.importing,
                            
                            connect_clicked => SettingsMsg::ImportBookmarks,
                        }
                    },

                    #[name = "progress_bar"]
                    gtk::ProgressBar {
                        #[watch]
                        set_visible: model.importing,
                        set_margin_top: 6,
                        set_margin_bottom: 6,
                        set_margin_start: 12,
                        set_margin_end: 12,
                        set_pulse_step: 0.1,
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SettingsDialog { 
            importing: false,
            root: root.clone(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SettingsMsg::ImportBookmarks => {
                // Get parent window by finding the root widget
                // The dialog will be parented to the main window when presented
                let parent_window: Option<gtk::Window> = self.root
                    .upcast_ref::<gtk::Widget>()
                    .root()
                    .and_then(|root| root.downcast::<gtk::Window>().ok());

                // Create file chooser using FileChooserNative (supports portals)
                let file_chooser = gtk::FileChooserNative::new(
                    Some("Import HTML Bookmarks"),
                    parent_window.as_ref(),
                    gtk::FileChooserAction::Open,
                    Some("Import"),
                    Some("Cancel"),
                );

                // Set modal
                file_chooser.set_modal(true);

                // Create filter for HTML files
                let filter = gtk::FileFilter::new();
                filter.add_pattern("*.html");
                filter.add_pattern("*.htm");
                filter.set_name(Some("HTML Bookmark Files"));
                file_chooser.add_filter(&filter);

                // Set initial folder to home directory
                if let Some(home) = dirs::home_dir() {
                    let _ = file_chooser.set_current_folder(Some(&gio::File::for_path(&home)));
                }

                // Connect response handler
                let sender_clone = sender.clone();
                file_chooser.connect_response(move |chooser, response| {
                    if response == gtk::ResponseType::Accept {
                        let path = chooser.file().and_then(|f| f.path());
                        sender_clone.input(SettingsMsg::FileSelected(path));
                    }
                });

                // Show file chooser
                file_chooser.show();
            }

            SettingsMsg::FileSelected(Some(path)) => {
                // Start import process
                self.importing = true;

                // Start pulsing the progress bar - we need access to widgets
                // Store progress bar widget in model
                let sender_clone = sender.clone();
                tokio::spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        // Read file
                        let html = std::fs::read_to_string(&path)
                            .map_err(|e| format!("Failed to read file: {}", e))?;

                        // Parse HTML bookmarks
                        let bookmarks = crate::import::html::parse_html_bookmarks(&html)?;

                        // Import to database
                        let db = crate::db::Database::new()
                            .map_err(|e| format!("Database error: {}", e))?;

                        db.import_bookmarks(bookmarks)
                            .map_err(|e| format!("Import failed: {}", e))
                    })
                    .await
                    .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));

                    sender_clone.input(SettingsMsg::ImportComplete(result));
                });
            }

            SettingsMsg::FileSelected(None) => {
                // User cancelled file picker
            }

            SettingsMsg::ImportComplete(Ok(result)) => {
                self.importing = false;

                // Log errors to console for debugging
                if !result.errors.is_empty() {
                    eprintln!("Import errors:");
                    for error in &result.errors {
                        eprintln!("  - {}", error);
                    }
                }

                // Format success message
                let msg = if result.skipped > 0 {
                    format!(
                        "Imported {} bookmarks, skipped {} duplicates",
                        result.imported, result.skipped
                    )
                } else {
                    format!("Imported {} bookmarks", result.imported)
                };

                // Add error count if any
                let msg = if result.errors.is_empty() {
                    msg
                } else {
                    format!("{}, {} errors", msg, result.errors.len())
                };

                // Notify parent to refresh bookmarks
                let _ = sender.output(SettingsOutput::RefreshBookmarks);
                
                // Show success toast
                let _ = sender.output(SettingsOutput::ShowToast(msg));

                // Fetch favicons for imported bookmarks asynchronously
                let imported_urls = result.imported_urls.clone();
                if !imported_urls.is_empty() {
                    tokio::spawn(async move {
                        for url in imported_urls {
                            // Run favicon fetch in blocking thread pool
                            let result = tokio::task::spawn_blocking({
                                let url = url.clone();
                                move || crate::fetch_metadata::fetch_favicon_sync(&url)
                            })
                            .await
                            .ok()
                            .flatten();

                            if let Some((hash, favicon_data)) = result {
                                // Create new DB connection for async task
                                if let Ok(db) = crate::db::Database::new() {
                                    // Insert favicon if new (INSERT OR IGNORE handles hash collisions)
                                    let _ = db.insert_favicon_if_new(hash, &favicon_data);
                                    // Update bookmarks with this URL to use the favicon hash
                                    let _ = db.conn().execute(
                                        "UPDATE bookmarks SET favicon_hash = ?1 WHERE url = ?2",
                                        (hash, &url),
                                    );
                                }
                            }
                        }
                    });
                }
            }

            SettingsMsg::ImportComplete(Err(error)) => {
                self.importing = false;

                // Show error toast
                let msg = format!("Import failed: {}", error);
                let _ = sender.output(SettingsOutput::ShowToast(msg));
            }

        }
    }
}
