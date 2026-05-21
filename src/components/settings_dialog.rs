use adw::prelude::*;
use gtk::gio;
use relm4::prelude::*;

#[derive(Debug)]
pub struct SettingsDialog {
    importing: bool,
    root: adw::PreferencesDialog,
    gc_days: u32,
    settings: Option<gio::Settings>,
}

#[derive(Debug, Clone, Copy)]
pub enum ImportFormat {
    Html,
    Json,
}

#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Json,
}

#[derive(Debug, Clone, Copy)]
pub enum FileAction {
    Import(ImportFormat),
    Export(ExportFormat),
}

#[derive(Debug)]
pub enum SettingsMsg {
    ImportBookmarks(ImportFormat),
    ExportBookmarks(ExportFormat),
    FileSelected(FileAction, Option<std::path::PathBuf>),
    ImportComplete(Result<crate::db::ImportResult, String>),
    ExportComplete(Result<usize, String>),
    SetGcDays(u32),
    ShowAbout,
}

#[derive(Debug)]
pub enum SettingsOutput {
    RefreshBookmarks,
    RefreshTags,
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
                    set_title: "Data",
                    set_description: Some("Note: if data is synced with other devices less often, deleted bookmarks may reappear"),

                    adw::ActionRow {
                        set_title: "Clear deleted bookmarks after",

                        add_suffix = &gtk::Box {
                            set_spacing: 6,
                            set_valign: gtk::Align::Center,

                            append = &gtk::ToggleButton {
                                set_label: "1d",
                                #[watch]
                                set_active: model.gc_days == 1,
                                connect_clicked => SettingsMsg::SetGcDays(1),
                            },
                            append = &gtk::ToggleButton {
                                set_label: "7d",
                                #[watch]
                                set_active: model.gc_days == 7,
                                connect_clicked => SettingsMsg::SetGcDays(7),
                            },
                            append = &gtk::ToggleButton {
                                set_label: "30d",
                                #[watch]
                                set_active: model.gc_days == 30,
                                connect_clicked => SettingsMsg::SetGcDays(30),
                            }
                        }
                    }
                },

                add = &adw::PreferencesGroup {
                    set_title: "Import / Export",
                    set_description: Some("Transfer bookmarks to/from other apps"),

                    adw::ActionRow {
                        set_title: "Import from bookmarks.html",
                        set_subtitle: "Netscape Bookmark Format",

                        add_suffix = &gtk::Button {
                            set_label: "Choose File",
                            set_valign: gtk::Align::Center,
                            add_css_class: "flat",
                            #[watch]
                            set_sensitive: !model.importing,

                            connect_clicked => SettingsMsg::ImportBookmarks(ImportFormat::Html),
                        }
                    },

                    adw::ActionRow {
                        set_title: "Import from JSON",
                        set_subtitle: "Marca JSON Format",

                        add_suffix = &gtk::Button {
                            set_label: "Choose File",
                            set_valign: gtk::Align::Center,
                            add_css_class: "flat",
                            #[watch]
                            set_sensitive: !model.importing,

                            connect_clicked => SettingsMsg::ImportBookmarks(ImportFormat::Json),
                        }
                    },

                    adw::ActionRow {
                        set_title: "Export to JSON",
                        set_subtitle: "Export all bookmarks to JSON",

                        add_suffix = &gtk::Button {
                            set_label: "Export",
                            set_valign: gtk::Align::Center,
                            add_css_class: "flat",
                            #[watch]
                            set_sensitive: !model.importing,

                            connect_clicked => SettingsMsg::ExportBookmarks(ExportFormat::Json),
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
                },

                add = &adw::PreferencesGroup {
                    set_title: "About",

                    adw::ActionRow {
                        set_title: "About",
                        set_activatable: true,
                        add_suffix = &gtk::Image::builder()
                            .icon_name("go-next-symbolic")
                            .build(),

                        connect_activated => SettingsMsg::ShowAbout,
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
        let settings = if gio::SettingsSchemaSource::default()
            .and_then(|source| source.lookup("io.github.simoonas.marca", true))
            .is_some()
        {
            Some(gio::Settings::new("io.github.simoonas.marca"))
        } else {
            None
        };

        let gc_days = settings.as_ref().map(|s| s.int("gc-days")).unwrap_or(30);

        let model = SettingsDialog {
            importing: false,
            root: root.clone(),
            gc_days: gc_days as u32,
            settings,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SettingsMsg::ImportBookmarks(format) => {
                let parent_window = self.root.root().and_then(|r| r.downcast::<gtk::Window>().ok());

                let file_dialog = gtk::FileDialog::new();
                file_dialog.set_title(match format {
                    ImportFormat::Html => "Import HTML Bookmarks",
                    ImportFormat::Json => "Import JSON Bookmarks",
                });
                file_dialog.set_accept_label(Some("Import"));

                let filter = gtk::FileFilter::new();
                match format {
                    ImportFormat::Html => {
                        filter.add_pattern("*.html");
                        filter.add_pattern("*.htm");
                        filter.set_name(Some("HTML Bookmark Files"));
                    }
                    ImportFormat::Json => {
                        filter.add_pattern("*.json");
                        filter.set_name(Some("JSON Bookmark Files"));
                    }
                }

                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                file_dialog.set_filters(Some(&filters));

                if let Some(home) = dirs::home_dir() {
                    file_dialog.set_initial_folder(Some(&gio::File::for_path(&home)));
                }

                let sender_clone = sender.clone();
                file_dialog.open(parent_window.as_ref(), gio::Cancellable::NONE, move |res| {
                    if let Ok(file) = res {
                        sender_clone.input(SettingsMsg::FileSelected(FileAction::Import(format), file.path()));
                    }
                });
            }

            SettingsMsg::ExportBookmarks(format) => {
                let parent_window = self.root.root().and_then(|r| r.downcast::<gtk::Window>().ok());

                let file_dialog = gtk::FileDialog::new();
                file_dialog.set_title("Export Bookmarks");
                file_dialog.set_accept_label(Some("Export"));
                file_dialog.set_initial_name(Some("Marca_bookmarks.json"));

                let filter = gtk::FileFilter::new();
                match format {
                    ExportFormat::Json => {
                        filter.add_pattern("*.json");
                        filter.set_name(Some("JSON Bookmark Files"));
                    }
                }

                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                file_dialog.set_filters(Some(&filters));

                if let Some(home) = dirs::home_dir() {
                    file_dialog.set_initial_folder(Some(&gio::File::for_path(&home)));
                }

                let sender_clone = sender.clone();
                file_dialog.save(parent_window.as_ref(), gio::Cancellable::NONE, move |res| {
                    if let Ok(file) = res {
                        sender_clone.input(SettingsMsg::FileSelected(FileAction::Export(format), file.path()));
                    }
                });
            }

            SettingsMsg::FileSelected(FileAction::Import(format), Some(path)) => {
                self.importing = true;
                let sender_clone = sender.clone();
                tokio::spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        let content = std::fs::read_to_string(&path)
                            .map_err(|e| format!("Failed to read file: {}", e))?;

                        let bookmarks = match format {
                            ImportFormat::Html => crate::import::html::parse_html_bookmarks(&content)?,
                            ImportFormat::Json => crate::import::json::parse_json_bookmarks(&content)?,
                        };

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

            SettingsMsg::FileSelected(FileAction::Export(ExportFormat::Json), Some(path)) => {
                self.importing = true;
                let sender_clone = sender.clone();
                tokio::spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        let db = crate::db::Database::new()
                            .map_err(|e| format!("Database error: {}", e))?;

                        let bookmarks = db.get_all_bookmarks_with_sort(
                            crate::db::SortField::Created,
                            crate::db::SortDirection::Descending,
                        ).map_err(|e| format!("Failed to fetch bookmarks: {}", e))?;

                        let json = crate::import::json::export_to_json(&bookmarks)?;
                        std::fs::write(&path, json)
                            .map_err(|e| format!("Failed to write file: {}", e))?;

                        Ok(bookmarks.len())
                    })
                    .await
                    .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));

                    sender_clone.input(SettingsMsg::ExportComplete(result));
                });
            }

            SettingsMsg::FileSelected(_, None) => {
                // User cancelled
            }

            SettingsMsg::ExportComplete(Ok(count)) => {
                self.importing = false;
                let msg = format!("Exported {} bookmarks", count);
                let _ = sender.output(SettingsOutput::ShowToast(msg));
            }

            SettingsMsg::ExportComplete(Err(error)) => {
                self.importing = false;
                let msg = format!("Export failed: {}", error);
                let _ = sender.output(SettingsOutput::ShowToast(msg));
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

                // Notify parent to refresh bookmarks and tags
                let _ = sender.output(SettingsOutput::RefreshBookmarks);
                let _ = sender.output(SettingsOutput::RefreshTags);

                // Show success toast
                let _ = sender.output(SettingsOutput::ShowToast(msg));

                // Fetch favicons for imported bookmarks asynchronously
                let imported_urls = result.imported_urls.clone();
                let sender_clone = sender.clone();
                if !imported_urls.is_empty() {
                    tokio::spawn(async move {
                        let mut domain_cache: std::collections::HashMap<String, Option<i32>> =
                            std::collections::HashMap::new();

                        for url in imported_urls {
                            let domain = crate::fetch_metadata::extract_domain(&url);
                            let domain_key = domain.clone().unwrap_or_else(|| url.clone());

                            let existing_hash =
                                if let Some(&hash_opt) = domain_cache.get(&domain_key) {
                                    hash_opt
                                } else {
                                    let mut hash_opt = None;

                                    // Check DB for domain
                                    if let (Ok(db), Some(d)) = (crate::db::Database::new(), &domain)
                                        && let Ok(Some(h)) = db.get_favicon_hash_for_domain(d)
                                    {
                                        hash_opt = Some(h);
                                    }

                                    // Fetch if not in DB
                                    if hash_opt.is_none() {
                                        let result = tokio::task::spawn_blocking({
                                            let url = url.clone();
                                            move || crate::fetch_metadata::fetch_favicon_sync(&url)
                                        })
                                        .await
                                        .ok()
                                        .flatten();

                                        if let Some((hash, favicon_data)) = result
                                            && let Ok(db) = crate::db::Database::new()
                                        {
                                            let _ = db.insert_favicon_if_new(hash, &favicon_data);
                                            hash_opt = Some(hash);
                                        }
                                    }

                                    domain_cache.insert(domain_key, hash_opt);
                                    hash_opt
                                };

                            // Update bookmark if we have a hash
                            if let Some(hash) = existing_hash
                                && let Ok(db) = crate::db::Database::new()
                            {
                                let _ = db.conn().execute(
                                    "UPDATE bookmarks SET favicon_hash = ?1 WHERE url = ?2",
                                    rusqlite::params![hash, &url],
                                );
                            }
                        }

                        // Let the app know we updated some favicons
                        let _ = sender_clone.output(SettingsOutput::RefreshBookmarks);
                    });
                }
            }

            SettingsMsg::ImportComplete(Err(error)) => {
                self.importing = false;

                // Show error toast
                let msg = format!("Import failed: {}", error);
                let _ = sender.output(SettingsOutput::ShowToast(msg));
            }

            SettingsMsg::SetGcDays(days) => {
                self.gc_days = days;
                if let Some(settings) = &self.settings {
                    let _ = settings.set_int("gc-days", days as i32);
                }
            }

            SettingsMsg::ShowAbout => {
                // Present adw::AboutDialog
                let about = adw::AboutDialog::builder()
                    .application_name("Marca")
                    .application_icon(crate::icon_names::custom::MARCA)
                    .developer_name("simoonas")
                    .version("0.1.0")
                    .website("https://github.com/simoonas/Marca")
                    .build();

                // Get parent window
                let parent_window: Option<gtk::Window> = self
                    .root
                    .upcast_ref::<gtk::Widget>()
                    .root()
                    .and_then(|root| root.downcast::<gtk::Window>().ok());

                about.present(parent_window.as_ref());
            }
        }
    }
}
