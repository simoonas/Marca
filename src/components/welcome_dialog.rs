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

                        adw::ActionRow {
                            set_title: "Navigate Bookmarks",
                            set_subtitle: "Use Ctrl+J/N / ↓ or Ctrl+K/P / ↑",
                        },

                        adw::ActionRow {
                            set_title: "Switch Sections",
                            set_subtitle: "Use Tab/S-Tab to switch between search and lists",
                        },
                    },

                    adw::PreferencesGroup {
                        set_title: "Add from highlighted text via CLI",

                        adw::ActionRow {
                            set_title: "Quickly Add from highlighted text",
                            set_subtitle: "Set a keyboard shortcut to:",
                            set_activatable: false,

                            #[wrap(Some)]
                            set_child = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 6,
                                set_valign: gtk::Align::Center,

                                gtk::Label {
                                    set_label: "io.github.simoonas.marca --add \"$(wl-paste -p)\"",
                                    set_selectable: true,
                                    add_css_class: "code-label",
                                    set_xalign: 0.0,
                                },
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
