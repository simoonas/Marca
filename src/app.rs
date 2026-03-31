use crate::components::{
    BookmarkEditDialog, BookmarkEditInit, BookmarkEditOutput, BookmarkRow, BookmarkRowOutput,
    TagRow, TagRowOutput,
};
use crate::db::Database;
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

pub struct App {
    db: Database,
    bookmarks: FactoryVecDeque<BookmarkRow>,
    pinned_tags: FactoryVecDeque<TagRow>,
    unpinned_tags: FactoryVecDeque<TagRow>,
    pinned_tag_ids: Vec<i64>,
    bookmark_search: String,
    tag_search: String,
    tag_search_active: bool,
    edit_dialog: Option<Controller<BookmarkEditDialog>>,
    toast_overlay: adw::ToastOverlay,
    last_deleted_bookmark: Option<(i64, crate::db::models::BookmarkWithTags)>,
    window: adw::ApplicationWindow,
}

#[derive(Debug)]
pub enum AppMsg {
    BookmarkSearch(String),
    TagSearch(String),
    TagSearchToggle,
    TagToggled(i64),
    ClearPinnedTags,
    RefreshBookmarks,
    RefreshTags,
    OpenBookmark(String),
    EditBookmark(i64),
    DeleteBookmark(i64),
    ConfirmSaveBookmark {
        id: i64,
        title: String,
        url: String,
        note: Option<String>,
        tag_titles: Vec<String>,
    },
    ShowToast(String),
    UndoDelete,
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

            #[local_ref]
            toast_overlay -> adw::ToastOverlay {
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    adw::NavigationSplitView {
                    set_vexpand: true,
                    set_min_sidebar_width: 200.0,
                    set_max_sidebar_width: 300.0,
                    set_sidebar_width_fraction: 0.25,
                    set_show_content: true,

                    #[wrap(Some)]
                    set_sidebar = &adw::NavigationPage {
                        set_title: "Tags",

                        #[wrap(Some)]
                        set_child = &adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {
                                pack_start = &gtk::ToggleButton {
                                    set_icon_name: "folder-saved-search-symbolic",
                                    add_css_class: "flat",
                                    set_tooltip_text: Some("Search tags"),
                                    #[watch]
                                    set_active: model.tag_search_active,
                                    connect_toggled[sender] => move |_| {
                                        sender.input(AppMsg::TagSearchToggle);
                                    }
                                },

                                pack_start = &gtk::SearchEntry {
                                    set_placeholder_text: Some("search tags..."),
                                    #[watch]
                                    set_visible: model.tag_search_active,
                                    set_hexpand: true,
                                    connect_search_changed[sender] => move |entry| {
                                        sender.input(AppMsg::TagSearch(entry.text().to_string()));
                                    }
                                }
                            },

                            #[wrap(Some)]
                            set_content = &gtk::ScrolledWindow {
                                set_hscrollbar_policy: gtk::PolicyType::Never,
                                set_vscrollbar_policy: gtk::PolicyType::Automatic,

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,

                                    #[local_ref]
                                    pinned_tags_list -> gtk::ListBox {
                                        set_css_classes: &["navigation-sidebar"],
                                    },

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_spacing: 8,
                                        set_margin_top: 6,
                                        set_margin_bottom: 6,
                                        set_margin_start: 12,
                                        set_margin_end: 12,
                                        #[watch]
                                        set_visible: !model.pinned_tag_ids.is_empty(),

                                        gtk::Separator {
                                            set_orientation: gtk::Orientation::Horizontal,
                                            set_hexpand: true,
                                            set_valign: gtk::Align::Center,
                                            set_vexpand: false,
                                        },

                                        gtk::Label {
                                            set_label: "clear",
                                            add_css_class: "dim-label",
                                            add_css_class: "caption",
                                            set_valign: gtk::Align::Center,
                                            set_cursor_from_name: Some("pointer"),

                                            add_controller = gtk::GestureClick {
                                                connect_released[sender] => move |_, _, _, _| {
                                                    sender.input(AppMsg::ClearPinnedTags);
                                                }
                                            }
                                        },

                                        gtk::Separator {
                                            set_orientation: gtk::Orientation::Horizontal,
                                            set_hexpand: true,
                                            set_valign: gtk::Align::Center,
                                            set_vexpand: false,
                                        },
                                    },

                                    #[local_ref]
                                    unpinned_tags_list -> gtk::ListBox {
                                        set_css_classes: &["navigation-sidebar"],
                                    }
                                }
                            }
                        }
                    },

                    #[wrap(Some)]
                    set_content = &adw::NavigationPage {
                        set_title: "Bookmarks",

                        #[wrap(Some)]
                        set_child = &adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {
                                #[wrap(Some)]
                                set_title_widget = &gtk::SearchEntry {
                                    set_placeholder_text: Some("search bookmarks (^K)"),
                                    set_hexpand: false,
                                    set_width_request: 400,
                                    #[watch]
                                    set_visible: !model.tag_search_active,
                                    connect_search_changed[sender] => move |entry| {
                                        sender.input(AppMsg::BookmarkSearch(entry.text().to_string()));
                                    }
                                }
                            },

                            #[wrap(Some)]
                            set_content = &gtk::ScrolledWindow {
                                set_hscrollbar_policy: gtk::PolicyType::Never,
                                set_vscrollbar_policy: gtk::PolicyType::Automatic,

                                #[local_ref]
                                bookmarks_list -> gtk::ListBox {
                                    set_margin_all: 12,
                                    set_selection_mode: gtk::SelectionMode::None,
                                    add_css_class: "background",
                                }
                            }
                        }
                    }
                },

                gtk::ActionBar {
                    set_revealed: true,
                    // Action buttons will be added here in future phase
                }
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
                BookmarkRowOutput::Edit(id) => AppMsg::EditBookmark(id),
                BookmarkRowOutput::Delete(id) => AppMsg::DeleteBookmark(id),
            });

        // Initialize factories for pinned and unpinned tags
        let pinned_tags = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |output| match output {
                TagRowOutput::Toggle(tag_id) => AppMsg::TagToggled(tag_id),
            });

        let unpinned_tags = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |output| match output {
                TagRowOutput::Toggle(tag_id) => AppMsg::TagToggled(tag_id),
            });

        let toast_overlay = adw::ToastOverlay::new();

        let model = App {
            db,
            bookmarks,
            pinned_tags,
            unpinned_tags,
            pinned_tag_ids: Vec::new(),
            bookmark_search: String::new(),
            tag_search: String::new(),
            tag_search_active: false,
            edit_dialog: None,
            toast_overlay: toast_overlay.clone(),
            last_deleted_bookmark: None,
            window: root.clone(),
        };

        let bookmarks_list = model.bookmarks.widget();
        let pinned_tags_list = model.pinned_tags.widget();
        let unpinned_tags_list = model.unpinned_tags.widget();
        let toast_overlay = &model.toast_overlay;
        let widgets = view_output!();

        // Load all bookmarks and tags initially
        sender.input(AppMsg::RefreshBookmarks);
        sender.input(AppMsg::RefreshTags);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::BookmarkSearch(query) => {
                self.bookmark_search = query;
                _sender.input(AppMsg::RefreshBookmarks);
            }

            AppMsg::TagSearch(query) => {
                self.tag_search = query;
                _sender.input(AppMsg::RefreshTags);
            }

            AppMsg::TagSearchToggle => {
                self.tag_search_active = !self.tag_search_active;
            }

            AppMsg::TagToggled(tag_id) => {
                // Toggle pin state
                if let Some(pos) = self.pinned_tag_ids.iter().position(|&id| id == tag_id) {
                    // Unpin
                    self.pinned_tag_ids.remove(pos);
                } else {
                    // Pin
                    self.pinned_tag_ids.push(tag_id);
                }
                _sender.input(AppMsg::RefreshTags);
                _sender.input(AppMsg::RefreshBookmarks);
            }

            AppMsg::ClearPinnedTags => {
                self.pinned_tag_ids.clear();
                _sender.input(AppMsg::RefreshTags);
                _sender.input(AppMsg::RefreshBookmarks);
            }

            AppMsg::RefreshTags => {
                match self.db.get_all_tags() {
                    Ok(mut tags) => {
                        // Filter by search query if active
                        if !self.tag_search.is_empty() {
                            let query_lower = self.tag_search.to_lowercase();
                            tags.retain(|tag| tag.title.to_lowercase().contains(&query_lower));
                        }

                        // Separate into pinned and unpinned
                        let (pinned, unpinned): (Vec<_>, Vec<_>) =
                            tags.into_iter().partition(|tag| {
                                tag.id
                                    .map(|id| self.pinned_tag_ids.contains(&id))
                                    .unwrap_or(false)
                            });

                        // Update pinned tags factory
                        {
                            let mut guard = self.pinned_tags.guard();
                            guard.clear();
                            for tag in pinned {
                                guard.push_back((tag, true));
                            }
                        }

                        // Update unpinned tags factory
                        {
                            let mut guard = self.unpinned_tags.guard();
                            guard.clear();
                            for tag in unpinned {
                                guard.push_back((tag, false));
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error loading tags: {}", e);
                    }
                }
            }

            AppMsg::RefreshBookmarks => {
                let results = if self.bookmark_search.is_empty() && self.pinned_tag_ids.is_empty() {
                    self.db.get_all_bookmarks()
                } else {
                    let query = if self.bookmark_search.is_empty() {
                        None
                    } else {
                        Some(self.bookmark_search.as_str())
                    };
                    self.db.search_bookmarks(query, &self.pinned_tag_ids)
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

            AppMsg::EditBookmark(id) => {
                // Fetch bookmark data
                match self.db.get_bookmark_by_id(id) {
                    Ok(bookmark_with_tags) => {
                        // Get all tags for autocomplete
                        let all_tags = self.db.get_all_tags().unwrap_or_default();

                        let init = BookmarkEditInit {
                            bookmark: bookmark_with_tags.bookmark,
                            current_tags: bookmark_with_tags.tags,
                            all_tags,
                        };

                        // Create and show dialog
                        let dialog = BookmarkEditDialog::builder().launch(init).forward(
                            _sender.input_sender(),
                            |output| match output {
                                BookmarkEditOutput::Save {
                                    id,
                                    title,
                                    url,
                                    note,
                                    tag_titles,
                                } => AppMsg::ConfirmSaveBookmark {
                                    id,
                                    title,
                                    url,
                                    note,
                                    tag_titles,
                                },
                                BookmarkEditOutput::ValidationError(msg) => AppMsg::ShowToast(msg),
                            },
                        );

                        // Present the dialog - get window from widgets
                        dialog.widget().present(Some(&self.window));

                        self.edit_dialog = Some(dialog);
                    }
                    Err(e) => {
                        eprintln!("Error loading bookmark: {}", e);
                    }
                }
            }

            AppMsg::ConfirmSaveBookmark {
                id,
                title,
                url,
                note,
                tag_titles,
            } => {
                // Validate fields (defensive check)
                if title.trim().is_empty() || url.trim().is_empty() {
                    eprintln!("Validation error: Title or URL is empty");
                    let toast = adw::Toast::new("Title and URL are required");
                    self.toast_overlay.add_toast(toast);
                    return;
                }

                // Update bookmark in database
                match self.db.update_bookmark(id, &title, &url, note.as_deref()) {
                    Ok(_) => {
                        // Update tags
                        if let Err(e) = self.db.update_bookmark_tags(id, &tag_titles) {
                            eprintln!("Error updating bookmark tags: {}", e);
                            let toast = adw::Toast::new("Failed to update tags");
                            self.toast_overlay.add_toast(toast);
                        }

                        // Close dialog explicitly
                        if let Some(dialog) = self.edit_dialog.take() {
                            dialog.widget().close();
                        }

                        // Refresh bookmarks and tags
                        _sender.input(AppMsg::RefreshBookmarks);
                        _sender.input(AppMsg::RefreshTags);

                        // Show success toast
                        let toast = adw::Toast::new("Bookmark updated");
                        self.toast_overlay.add_toast(toast);
                    }
                    Err(e) => {
                        eprintln!("Error updating bookmark: {}", e);
                        let toast = adw::Toast::new("Failed to update bookmark");
                        self.toast_overlay.add_toast(toast);
                        // Don't close dialog on error - let user retry
                    }
                }
            }

            AppMsg::ShowToast(msg) => {
                let toast = adw::Toast::new(&msg);
                self.toast_overlay.add_toast(toast);
            }

            AppMsg::DeleteBookmark(id) => {
                // Store bookmark before deleting for undo
                if let Ok(bookmark_with_tags) = self.db.get_bookmark_by_id(id) {
                    self.last_deleted_bookmark = Some((id, bookmark_with_tags.clone()));

                    // Delete from database
                    match self.db.delete_bookmark(id) {
                        Ok(_) => {
                            // Refresh bookmarks list
                            _sender.input(AppMsg::RefreshBookmarks);

                            // Show toast with undo
                            let toast = adw::Toast::new("Bookmark deleted");
                            toast.set_button_label(Some("Undo"));
                            toast.set_action_name(Some("app.undo-delete"));
                            toast.set_timeout(5); // 5 seconds to undo

                            // Connect undo action
                            let sender = _sender.clone();
                            toast.connect_button_clicked(move |_| {
                                sender.input(AppMsg::UndoDelete);
                            });

                            self.toast_overlay.add_toast(toast);
                        }
                        Err(e) => {
                            eprintln!("Error deleting bookmark: {}", e);
                            let toast = adw::Toast::new("Failed to delete bookmark");
                            self.toast_overlay.add_toast(toast);
                        }
                    }
                }
            }

            AppMsg::UndoDelete => {
                if let Some((id, bookmark_with_tags)) = self.last_deleted_bookmark.take() {
                    // Re-insert the bookmark
                    let mut bookmark = bookmark_with_tags.bookmark;
                    bookmark.id = Some(id); // Keep the same ID

                    // For now, just refresh to show it's working
                    // TODO: Properly re-insert with same ID
                    eprintln!("Undo delete not fully implemented yet - need to re-insert bookmark");

                    let toast = adw::Toast::new("Undo is not fully implemented yet");
                    self.toast_overlay.add_toast(toast);
                }
            }
        }
    }
}
