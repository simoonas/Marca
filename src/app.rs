use crate::components::{
    BookmarkEditDialog, BookmarkEditInit, BookmarkEditOutput, BookmarkRow, BookmarkRowOutput,
    SettingsDialog, SettingsOutput, TagRow, TagRowOutput,
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
    edit_dialog: Option<Controller<BookmarkEditDialog>>,
    toast_overlay: adw::ToastOverlay,
    last_deleted_bookmark: Option<(i64, crate::db::models::BookmarkWithTags)>,
    window: adw::ApplicationWindow,
    settings_dialog: Option<Controller<SettingsDialog>>,

    // Hotkey widgets
    tag_search_entry: gtk::SearchEntry,
    bookmark_search_entry: gtk::SearchEntry,
    shortcut_label: adw::ShortcutLabel,
}

#[derive(Debug)]
pub enum AppMsg {
    BookmarkSearch(String),
    TagSearch(String),
    TagToggled(i64),
    ClearPinnedTags,
    RefreshBookmarks,
    RefreshTags,
    OpenBookmark(String),
    CreateBookmark,
    EditBookmark(i64),
    DeleteBookmark(i64),
    ConfirmSaveBookmark {
        id: i64,
        title: String,
        url: String,
        note: Option<String>,
        tag_titles: Vec<String>,
    },
    ConfirmCreateBookmark {
        title: String,
        url: String,
        note: Option<String>,
        tag_titles: Vec<String>,
    },
    ShowToast(String),
    UndoDelete,
    OpenSettings,

    // Hotkey system messages
    FocusChanged,
    FocusTagSearch,
    FocusBookmarkSearch,
    NavigateNext,
    NavigatePrev,
    NavigateTab,
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

            connect_focus_widget_notify[sender] => move |_| {
                sender.input(AppMsg::FocusChanged);
            },

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
                                #[wrap(Some)]
                                #[name = "tag_search_entry"]
                                set_title_widget = &gtk::SearchEntry {
                                    set_placeholder_text: Some("Search tags..."),
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
                                pack_start = &gtk::Button {
                                    set_icon_name: "list-add-symbolic",
                                    add_css_class: "flat",
                                    set_tooltip_text: Some("Create bookmark"),
                                    connect_clicked => AppMsg::CreateBookmark,
                                },

                                #[wrap(Some)]
                                #[name = "bookmark_search_entry"]
                                set_title_widget = &gtk::SearchEntry {
                                    set_placeholder_text: Some("search bookmarks (^K)"),
                                    set_hexpand: false,
                                    set_width_request: 400,
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
                    
                    pack_start = &gtk::Button {
                        set_icon_name: "cogged-wheel",
                        set_tooltip_text: Some("Settings"),
                        connect_clicked => AppMsg::OpenSettings,
                    },

                    #[name = "shortcut_label"]
                    pack_end = &adw::ShortcutLabel::new("<Ctrl>l") {
                        set_disabled_text: "Search bookmarks",
                    }
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

        let mut model = App {
            db,
            bookmarks,
            pinned_tags,
            unpinned_tags,
            pinned_tag_ids: Vec::new(),
            bookmark_search: String::new(),
            tag_search: String::new(),
            edit_dialog: None,
            toast_overlay: toast_overlay.clone(),
            last_deleted_bookmark: None,
            window: root.clone(),
            settings_dialog: None,

            tag_search_entry: gtk::SearchEntry::new(),
            bookmark_search_entry: gtk::SearchEntry::new(),
            shortcut_label: adw::ShortcutLabel::new(""),
        };

        let bookmarks_list = model.bookmarks.widget();
        let pinned_tags_list = model.pinned_tags.widget();
        let unpinned_tags_list = model.unpinned_tags.widget();
        let toast_overlay = &model.toast_overlay;
        let widgets = view_output!();

        model.tag_search_entry = widgets.tag_search_entry.clone();
        model.bookmark_search_entry = widgets.bookmark_search_entry.clone();
        model.shortcut_label = widgets.shortcut_label.clone();

        let key_controller = gtk::EventControllerKey::new();
        let sender_clone = sender.clone();
        key_controller.connect_key_pressed(move |_, key, _keycode, state| {
            use gtk::gdk::Key;
            use gtk::gdk::ModifierType;
            let ctrl = state.contains(ModifierType::CONTROL_MASK);
            match (key, ctrl) {
                (Key::j | Key::n, true) => { sender_clone.input(AppMsg::NavigateNext); gtk::glib::Propagation::Stop }
                (Key::k | Key::p, true) => { sender_clone.input(AppMsg::NavigatePrev); gtk::glib::Propagation::Stop }
                (Key::Down, false) => { sender_clone.input(AppMsg::NavigateNext); gtk::glib::Propagation::Stop }
                (Key::Up, false) => { sender_clone.input(AppMsg::NavigatePrev); gtk::glib::Propagation::Stop }
                (Key::Tab, false) => { sender_clone.input(AppMsg::NavigateTab); gtk::glib::Propagation::Stop }
                (Key::l, true) => { sender_clone.input(AppMsg::FocusBookmarkSearch); gtk::glib::Propagation::Stop }
                (Key::h, true) => { sender_clone.input(AppMsg::FocusTagSearch); gtk::glib::Propagation::Stop }
                _ => gtk::glib::Propagation::Proceed
            }
        });
        model.window.add_controller(key_controller);

        model.bookmark_search_entry.grab_focus();

        // Load custom CSS for favicon styling
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_data(
            ".favicon-icon { border-radius: 8px; min-width: 32px; min-height: 32px; }"
        );
        gtk::style_context_add_provider_for_display(
            &adw::prelude::WidgetExt::display(&root),
            &css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
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

            AppMsg::TagToggled(tag_id) => {
                let mut focus_search = false;
                if let Some(focused) = gtk::prelude::RootExt::focus(&self.window) {
                    if let Some(row) = focused.ancestor(gtk::ListBoxRow::static_type()).and_downcast::<gtk::ListBoxRow>() {
                        let row_widget = row.upcast_ref::<gtk::Widget>();
                        if row_widget.is_ancestor(self.pinned_tags.widget().upcast_ref::<gtk::Widget>()) {
                            if self.pinned_tags.guard().len() == 1 { focus_search = true; }
                        } else if row_widget.is_ancestor(self.unpinned_tags.widget().upcast_ref::<gtk::Widget>()) {
                            if self.unpinned_tags.guard().len() == 1 { focus_search = true; }
                        }
                    }
                }

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

                if focus_search {
                    self.tag_search_entry.grab_focus();
                }
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

            AppMsg::CreateBookmark => {
                // Get all tags for autocomplete
                let all_tags = self.db.get_all_tags().unwrap_or_default();

                let init = BookmarkEditInit {
                    bookmark: None, // Create mode
                    current_tags: vec![],
                    all_tags,
                };

                // Create and show dialog
                let dialog = BookmarkEditDialog::builder().launch(init).forward(
                    _sender.input_sender(),
                    |output| match output {
                        BookmarkEditOutput::SaveCreate {
                            title,
                            url,
                            note,
                            tag_titles,
                        } => AppMsg::ConfirmCreateBookmark {
                            title,
                            url,
                            note,
                            tag_titles,
                        },
                        BookmarkEditOutput::ValidationError(msg) => AppMsg::ShowToast(msg),
                        _ => unreachable!(),
                    },
                );

                // Present the dialog
                dialog.widget().present(Some(&self.window));

                self.edit_dialog = Some(dialog);
            }

            AppMsg::EditBookmark(id) => {
                // Fetch bookmark data
                match self.db.get_bookmark_by_id(id) {
                    Ok(bookmark_with_tags) => {
                        // Get all tags for autocomplete
                        let all_tags = self.db.get_all_tags().unwrap_or_default();

                        let init = BookmarkEditInit {
                            bookmark: Some(bookmark_with_tags.bookmark),
                            current_tags: bookmark_with_tags.tags,
                            all_tags,
                        };

                        // Create and show dialog
                        let dialog = BookmarkEditDialog::builder().launch(init).forward(
                            _sender.input_sender(),
                            |output| match output {
                                BookmarkEditOutput::SaveEdit {
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
                                _ => unreachable!(),
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

                        // Spawn async favicon fetch AFTER dialog closed (non-blocking)
                        let url_clone = url.clone();
                        let sender_clone = _sender.clone();
                        tokio::spawn(async move {
                            // Run blocking favicon fetch in a blocking thread pool
                            let result = tokio::task::spawn_blocking(move || {
                                crate::fetch_metadata::fetch_favicon_sync(&url_clone)
                            })
                            .await
                            .ok()
                            .flatten();

                            if let Some((hash, favicon_data)) = result {
                                // Create new DB connection for async task
                                if let Ok(db) = crate::db::Database::new() {
                                    // Insert favicon if new (INSERT OR IGNORE handles hash collisions)
                                    if let Err(e) = db.insert_favicon_if_new(hash, &favicon_data) {
                                        eprintln!("Error saving favicon data: {}", e);
                                    }
                                    // Update bookmark's favicon_hash field
                                    if let Err(e) = db.update_bookmark_favicon_hash(id, hash) {
                                        eprintln!("Error updating bookmark favicon hash: {}", e);
                                    }
                                    // Refresh bookmarks to show new favicon
                                    sender_clone.input(AppMsg::RefreshBookmarks);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Error updating bookmark: {}", e);
                        let toast = adw::Toast::new("Failed to update bookmark");
                        self.toast_overlay.add_toast(toast);
                        // Don't close dialog on error - let user retry
                    }
                }
            }

            AppMsg::ConfirmCreateBookmark {
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

                // Insert new bookmark into database
                match self.db.insert_bookmark(&title, &url, note.as_deref()) {
                    Ok(bookmark_id) => {
                        // Update tags for the new bookmark
                        if let Err(e) = self.db.update_bookmark_tags(bookmark_id, &tag_titles) {
                            eprintln!("Error adding bookmark tags: {}", e);
                            let toast = adw::Toast::new("Failed to add tags");
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
                        let toast = adw::Toast::new("Bookmark created");
                        self.toast_overlay.add_toast(toast);

                        // Spawn async favicon fetch AFTER dialog closed (non-blocking)
                        let url_clone = url.clone();
                        let sender_clone = _sender.clone();
                        tokio::spawn(async move {
                            // Run blocking favicon fetch in a blocking thread pool
                            let result = tokio::task::spawn_blocking(move || {
                                crate::fetch_metadata::fetch_favicon_sync(&url_clone)
                            })
                            .await
                            .ok()
                            .flatten();

                            if let Some((hash, favicon_data)) = result {
                                // Create new DB connection for async task
                                if let Ok(db) = crate::db::Database::new() {
                                    // Insert favicon if new (INSERT OR IGNORE handles hash collisions)
                                    if let Err(e) = db.insert_favicon_if_new(hash, &favicon_data) {
                                        eprintln!("Error saving favicon data: {}", e);
                                    }
                                    // Update bookmark's favicon_hash field
                                    if let Err(e) = db.update_bookmark_favicon_hash(bookmark_id, hash) {
                                        eprintln!("Error updating bookmark favicon hash: {}", e);
                                    }
                                    // Refresh bookmarks to show new favicon
                                    sender_clone.input(AppMsg::RefreshBookmarks);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Error creating bookmark: {}", e);
                        // Check if it's a duplicate URL error
                        let error_msg = e.to_string();
                        let toast_message = if error_msg.contains("UNIQUE constraint failed") {
                            "A bookmark with this URL already exists"
                        } else {
                            "Failed to create bookmark"
                        };
                        let toast = adw::Toast::new(toast_message);
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

            AppMsg::FocusChanged => {
                if let Some(focused) = gtk::prelude::RootExt::focus(&self.window) {
                    let focused_widget = focused.upcast_ref::<gtk::Widget>();
                    let tag_search = self.tag_search_entry.upcast_ref::<gtk::Widget>();
                    let pinned = self.pinned_tags.widget();
                    let unpinned = self.unpinned_tags.widget();
                    let pinned_widget = pinned.upcast_ref::<gtk::Widget>();
                    let unpinned_widget = unpinned.upcast_ref::<gtk::Widget>();
                    
                    if focused_widget == tag_search || focused_widget.is_ancestor(tag_search)
                        || focused_widget == pinned_widget || focused_widget.is_ancestor(pinned_widget)
                        || focused_widget == unpinned_widget || focused_widget.is_ancestor(unpinned_widget) 
                    {
                        self.shortcut_label.set_accelerator("<Ctrl>l");
                        self.shortcut_label.set_disabled_text("Search bookmarks");
                    } else {
                        let bm_search = self.bookmark_search_entry.upcast_ref::<gtk::Widget>();
                        let bms = self.bookmarks.widget();
                        let bms_widget = bms.upcast_ref::<gtk::Widget>();
                        if focused_widget == bm_search || focused_widget.is_ancestor(bm_search)
                            || focused_widget == bms_widget || focused_widget.is_ancestor(bms_widget) 
                        {
                            self.shortcut_label.set_accelerator("<Ctrl>h");
                            self.shortcut_label.set_disabled_text("Search tags");
                        }
                    }
                }
            }

            AppMsg::FocusTagSearch => {
                self.tag_search_entry.grab_focus();
            }

            AppMsg::FocusBookmarkSearch => {
                self.bookmark_search_entry.grab_focus();
            }

            AppMsg::NavigateNext | AppMsg::NavigateTab => {
                if let Some(focused) = gtk::prelude::RootExt::focus(&self.window) {
                    let focused_widget = focused.upcast_ref::<gtk::Widget>();
                    let is_tag_search = focused_widget == self.tag_search_entry.upcast_ref::<gtk::Widget>() || focused_widget.is_ancestor(self.tag_search_entry.upcast_ref::<gtk::Widget>());
                    let is_bm_search = focused_widget == self.bookmark_search_entry.upcast_ref::<gtk::Widget>() || focused_widget.is_ancestor(self.bookmark_search_entry.upcast_ref::<gtk::Widget>());

                    if is_tag_search {
                        if let Some(first) = self.pinned_tags.widget().row_at_index(0).or_else(|| self.unpinned_tags.widget().row_at_index(0)) {
                            first.grab_focus();
                        }
                    } else if is_bm_search {
                        if let Some(first) = self.bookmarks.widget().row_at_index(0) {
                            first.grab_focus();
                        }
                    } else if let Some(row) = focused.ancestor(gtk::ListBoxRow::static_type()).and_downcast::<gtk::ListBoxRow>() {
                        let row_widget = row.upcast_ref::<gtk::Widget>();
                        if row_widget.is_ancestor(self.pinned_tags.widget().upcast_ref::<gtk::Widget>()) {
                            if let Some(next) = self.pinned_tags.widget().row_at_index(row.index() + 1) {
                                next.grab_focus();
                            } else if let Some(first) = self.unpinned_tags.widget().row_at_index(0) {
                                first.grab_focus();
                            } else if let Some(first) = self.pinned_tags.widget().row_at_index(0) {
                                first.grab_focus();
                            }
                        } else if row_widget.is_ancestor(self.unpinned_tags.widget().upcast_ref::<gtk::Widget>()) {
                            if let Some(next) = self.unpinned_tags.widget().row_at_index(row.index() + 1) {
                                next.grab_focus();
                            } else if let Some(first) = self.pinned_tags.widget().row_at_index(0).or_else(|| self.unpinned_tags.widget().row_at_index(0)) {
                                first.grab_focus();
                            }
                        } else if row_widget.is_ancestor(self.bookmarks.widget().upcast_ref::<gtk::Widget>()) {
                            if let Some(next) = self.bookmarks.widget().row_at_index(row.index() + 1) {
                                next.grab_focus();
                            } else if let Some(first) = self.bookmarks.widget().row_at_index(0) {
                                first.grab_focus();
                            }
                        }
                    }
                }
            }

            AppMsg::NavigatePrev => {
                if let Some(focused) = gtk::prelude::RootExt::focus(&self.window) {
                    if let Some(row) = focused.ancestor(gtk::ListBoxRow::static_type()).and_downcast::<gtk::ListBoxRow>() {
                        let row_widget = row.upcast_ref::<gtk::Widget>();
                        if row_widget.is_ancestor(self.pinned_tags.widget().upcast_ref::<gtk::Widget>()) {
                            if row.index() > 0 {
                                if let Some(prev) = self.pinned_tags.widget().row_at_index(row.index() - 1) {
                                    prev.grab_focus();
                                }
                            } else {
                                // at first pinned tag, wrap to last unpinned or last pinned
                                let last_unpinned_idx = self.unpinned_tags.guard().len() as i32 - 1;
                                if last_unpinned_idx >= 0 {
                                    if let Some(last) = self.unpinned_tags.widget().row_at_index(last_unpinned_idx) {
                                        last.grab_focus();
                                    }
                                } else {
                                    let last_pinned_idx = self.pinned_tags.guard().len() as i32 - 1;
                                    if let Some(last) = self.pinned_tags.widget().row_at_index(last_pinned_idx) {
                                        last.grab_focus();
                                    }
                                }
                            }
                        } else if row_widget.is_ancestor(self.unpinned_tags.widget().upcast_ref::<gtk::Widget>()) {
                            if row.index() > 0 {
                                if let Some(prev) = self.unpinned_tags.widget().row_at_index(row.index() - 1) {
                                    prev.grab_focus();
                                }
                            } else {
                                // at first unpinned tag, go to last pinned tag
                                let last_pinned_idx = self.pinned_tags.guard().len() as i32 - 1;
                                if last_pinned_idx >= 0 {
                                    if let Some(last) = self.pinned_tags.widget().row_at_index(last_pinned_idx) {
                                        last.grab_focus();
                                    }
                                } else {
                                    let last_unpinned_idx = self.unpinned_tags.guard().len() as i32 - 1;
                                    if let Some(last) = self.unpinned_tags.widget().row_at_index(last_unpinned_idx) {
                                        last.grab_focus();
                                    }
                                }
                            }
                        } else if row_widget.is_ancestor(self.bookmarks.widget().upcast_ref::<gtk::Widget>()) {
                            if row.index() > 0 {
                                if let Some(prev) = self.bookmarks.widget().row_at_index(row.index() - 1) {
                                    prev.grab_focus();
                                }
                            } else {
                                // at first bookmark, wrap to last
                                let last_idx = self.bookmarks.guard().len() as i32 - 1;
                                if last_idx >= 0 {
                                    if let Some(last) = self.bookmarks.widget().row_at_index(last_idx) {
                                        last.grab_focus();
                                    }
                                }
                            }
                        }
                    }
                }
            }

            AppMsg::OpenSettings => {
                // Create settings dialog if not exists
                if self.settings_dialog.is_none() {
                    let dialog = SettingsDialog::builder()
                        .launch(())
                        .forward(_sender.input_sender(), |output| match output {
                            SettingsOutput::RefreshBookmarks => AppMsg::RefreshBookmarks,
                            SettingsOutput::ShowToast(msg) => AppMsg::ShowToast(msg),
                        });
                    self.settings_dialog = Some(dialog);
                }

                // Present the dialog
                if let Some(ref dialog) = self.settings_dialog {
                    dialog.widget().present(Some(&self.window));
                }
            }
        }
    }
}
