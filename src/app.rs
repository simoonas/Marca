use crate::components::{BookmarkRow, BookmarkRowOutput};
use crate::db::Database;
use adw::prelude::*;
use relm4::factory::{DynamicIndex, FactoryVecDeque};
use relm4::prelude::*;

pub struct App {
    db: Database,
    bookmarks: FactoryVecDeque<BookmarkRow>,
    current_search: String,
}

#[derive(Debug)]
pub enum AppMsg {
    Search(String),
    RefreshBookmarks,
    OpenBookmark(String),
    DeleteBookmark(DynamicIndex),
}

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = Database;
    type Input = AppMsg;
    type Output = ();

    view! {
        adw::ApplicationWindow {
            set_default_width: 900,
            set_default_height: 600,
            set_title: Some("Marca"),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &gtk::SearchEntry {
                        set_placeholder_text: Some("search (^K)"),
                        set_hexpand: false,
                        set_width_request: 400,

                        connect_search_changed[sender] => move |entry| {
                            sender.input(AppMsg::Search(entry.text().to_string()));
                        }
                    }
                },

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 0,

                    // Left sidebar for tags
                    gtk::ScrolledWindow {
                        set_width_request: 200,
                        set_hscrollbar_policy: gtk::PolicyType::Never,
                        set_vscrollbar_policy: gtk::PolicyType::Automatic,

                        gtk::ListBox {
                            set_css_classes: &["navigation-sidebar"],
                            // Tags will be added here in future phase
                        }
                    },

                    gtk::Separator {
                        set_orientation: gtk::Orientation::Vertical,
                    },

                    // Main content area for deeplink items
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,
                        set_vscrollbar_policy: gtk::PolicyType::Automatic,

                        #[local_ref]
                        bookmarks_list -> gtk::ListBox {
                            set_margin_all: 12,
                            set_selection_mode: gtk::SelectionMode::None,
                            add_css_class: "background",
                        }
                    }
                },

                add_bottom_bar = &gtk::ActionBar {
                    set_revealed: true,
                    // Action buttons will be added here in future phase
                }
            }
        }
    }

    fn init(
        db: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Initialize the factory for bookmark rows
        let bookmarks = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |output| match output {
                BookmarkRowOutput::Open(url) => AppMsg::OpenBookmark(url),
                BookmarkRowOutput::Delete(index) => AppMsg::DeleteBookmark(index),
            });

        let model = App {
            db,
            bookmarks,
            current_search: String::new(),
        };

        let bookmarks_list = model.bookmarks.widget();
        let widgets = view_output!();

        // Load all bookmarks initially
        sender.input(AppMsg::RefreshBookmarks);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::Search(query) => {
                self.current_search = query;
                _sender.input(AppMsg::RefreshBookmarks);
            }

            AppMsg::RefreshBookmarks => {
                let results = if self.current_search.is_empty() {
                    self.db.get_all_bookmarks()
                } else {
                    self.db.search_bookmarks(Some(&self.current_search), &[])
                };

                match results {
                    Ok(bookmarks) => {
                        let mut guard = self.bookmarks.guard();
                        guard.clear();
                        for bookmark_with_tags in bookmarks {
                            guard.push_back(bookmark_with_tags);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error loading bookmarks: {}", e);
                    }
                }
            }

            AppMsg::OpenBookmark(url) => {
                eprintln!("Opening URL: {}", url);
                // Open the URL using gio
                use gtk::gio;
                match gio::AppInfo::launch_default_for_uri(&url, None::<&gio::AppLaunchContext>) {
                    Ok(_) => eprintln!("Successfully launched URL"),
                    Err(e) => eprintln!("Failed to open URL: {}", e),
                }
            }

            AppMsg::DeleteBookmark(index) => {
                // For now, just remove from UI
                // TODO: Delete from database as well
                self.bookmarks.guard().remove(index.current_index());
                eprintln!("Bookmark deleted from UI (not from DB yet)");
            }
        }
    }
}
