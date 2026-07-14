use adw::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub struct WelcomeDialog {
    root: adw::Dialog,
}

#[relm4::component(pub)]
impl SimpleComponent for WelcomeDialog {
    type Init = ();
    type Input = ();
    type Output = ();

    view! {
        #[root]
        adw::Dialog {
            set_title: "Welcome to Marca",
            set_content_width: 500,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,
                    set_margin_all: 24,

                    adw::StatusPage {
                        set_title: "Welcome to Marca!",
                        set_description: Some("A minimal bookmarking utility"),
                        set_icon_name: Some(crate::icon_names::custom::MARCA),
                    },

                    adw::PreferencesGroup {
                        set_title: "Keyboard Shortcuts",

                        gtk::Box {
                            add_css_class: "card",

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 8,
                                set_margin_all: 16,

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 0,

                                    gtk::Label {
                                        set_label: "Navigate Bookmarks",
                                        set_halign: gtk::Align::Start,
                                        set_valign: gtk::Align::Center,
                                        set_hexpand: true,
                                        add_css_class: "dim-label",
                                    },
                                    gtk::Label {
                                        set_label: "Ctrl+J/K  or  ↑/↓",
                                        set_halign: gtk::Align::End,
                                        set_valign: gtk::Align::Center,
                                        add_css_class: "keycap",
                                    },
                                },

                                gtk::Separator {},

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 0,

                                    gtk::Label {
                                        set_label: "Switch Sections",
                                        set_halign: gtk::Align::Start,
                                        set_valign: gtk::Align::Center,
                                        set_hexpand: true,
                                        add_css_class: "dim-label",
                                    },
                                    gtk::Label {
                                        set_label: "Tab / S-Tab",
                                        set_halign: gtk::Align::End,
                                        set_valign: gtk::Align::Center,
                                        add_css_class: "keycap",
                                    },
                                },
                            },
                        },
                    },

                    adw::PreferencesGroup {
                        set_title: "Add from highlighted text",
                        set_description: Some("(requires wl-clipboard for Wayland, xsel/xclip for X11)"),

                        adw::ActionRow {
                            set_title: "Set a keybind to:",
                            set_subtitle: "flatpak run io.github.simoonas.marca --add-selection",

                            add_suffix = &gtk::Button {
                                set_icon_name: "edit-copy-symbolic",
                                add_css_class: "flat",
                                set_valign: gtk::Align::Center,
                                set_tooltip_text: Some("Copy to clipboard"),

                                connect_clicked[text = "flatpak run io.github.simoonas.marca --add-selection"] => move |_| {
                                    if let Some(display) = gtk::gdk::Display::default() {
                                        display.clipboard().set_text(text);
                                    }
                                }
                            }
                        },
                    },

                    gtk::Button {
                        set_label: "Get Started",
                        add_css_class: "suggested-action",
                        set_margin_top: 12,
                        set_halign: gtk::Align::Center,
                        connect_clicked[sender] => move |_| {
                            sender.input_sender().send(()).unwrap();
                        }
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
        let model = WelcomeDialog { root: root.clone() };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _msg: Self::Input, _sender: ComponentSender<Self>) {
        self.root.close();
    }
}
