use crate::db::models::{Bookmark, BookmarkWithTags, Tag};
use adw::prelude::*;
use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender};
use relm4::prelude::*;

#[derive(Debug)]
pub struct BookmarkRow {
    bookmark: Bookmark,
    tags: Vec<Tag>,
}

#[derive(Debug)]
pub enum BookmarkRowMsg {
    Clicked,
}

#[derive(Debug)]
pub enum BookmarkRowOutput {
    Open(String),
    Delete(DynamicIndex),
}

#[relm4::factory(pub)]
impl FactoryComponent for BookmarkRow {
    type Init = BookmarkWithTags;
    type Input = BookmarkRowMsg;
    type Output = BookmarkRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        root = gtk::ListBoxRow {
            set_activatable: true,
            set_selectable: false,
            add_css_class: "card",
            set_margin_top: 6,
            set_margin_bottom: 6,
            set_margin_start: 0,
            set_margin_end: 0,

            connect_activate[sender] => move |_| {
                sender.input(BookmarkRowMsg::Clicked);
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_margin_all: 12,

                // Title and URL
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,

                    gtk::Label {
                        set_label: &self.bookmark.title,
                        set_halign: gtk::Align::Start,
                        set_xalign: 0.0,
                        add_css_class: "title-4",
                        set_wrap: true,
                        set_wrap_mode: gtk::pango::WrapMode::WordChar,
                    },

                    gtk::Label {
                        set_label: &self.bookmark.url,
                        set_halign: gtk::Align::Start,
                        set_xalign: 0.0,
                        add_css_class: "dim-label",
                        add_css_class: "caption",
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                    },
                },

                // Tags row
                #[name = "tags_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,
                    set_visible: !self.tags.is_empty(),
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            bookmark: init.bookmark,
            tags: init.tags,
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        // Add tag buttons
        for tag in &self.tags {
            let btn = gtk::Button::builder()
                .label(&tag.title)
                .css_classes(vec!["pill".to_string()])
                .sensitive(false)
                .build();
            widgets.tags_box.append(&btn);
        }

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            BookmarkRowMsg::Clicked => {
                sender
                    .output(BookmarkRowOutput::Open(self.bookmark.url.clone()))
                    .unwrap();
            }
        }
    }
}
