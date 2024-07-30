/// link to the [source](https://gtk-rs.org/gtk-rs-core/stable/latest/docs/glib/subclass/index.html)
use gtk::{
    gio::prelude::*,
    glib::{self, subclass::prelude::*},
};
use std::cell::{Cell, RefCell};
use unirun_if::match_if::Match;

use crate::utils::{build_image, build_label};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct GMatch {
        id: RefCell<String>,
        title: RefCell<String>,
        description: RefCell<Option<String>>,
        icon: RefCell<Option<String>>,
        use_pango: Cell<bool>,
        plugin_pid: Cell<u64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GMatch {
        const NAME: &'static str = "GMatch";

        type Type = super::GMatch;
    }

    impl ObjectImpl for GMatch {
        fn properties() -> &'static [glib::ParamSpec] {
            use std::sync::OnceLock;
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecString::builder("id").build(),
                    glib::ParamSpecString::builder("title").build(),
                    glib::ParamSpecString::builder("description").build(),
                    glib::ParamSpecString::builder("icon").build(),
                    glib::ParamSpecBoolean::builder("use-pango").build(),
                    glib::ParamSpecUInt64::builder("plugin-pid").build(),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "id" => {
                    let id = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.id.replace(id);
                }
                "title" => {
                    let title = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.title.replace(title);
                }
                "description" => {
                    let description = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.description.replace(description);
                }
                "icon" => {
                    let icon = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.icon.replace(icon);
                }
                "use-pango" => {
                    let use_pango = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.use_pango.replace(use_pango);
                }
                "plugin-pid" => {
                    let plugin_pid = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.plugin_pid.replace(plugin_pid);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "id" => self.id.borrow().to_value(),
                "title" => self.title.borrow().to_value(),
                "description" => self.description.borrow().to_value(),
                "icon" => self.icon.borrow().to_value(),
                "use-pango" => self.use_pango.get().to_value(),
                "plugin-pid" => self.plugin_pid.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed()
        }
    }
}

glib::wrapper! {
    pub struct GMatch(ObjectSubclass<imp::GMatch>);
}

// TODO does we need so much setters-getters? Is there any way to simplify this
impl GMatch {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn get_id(&self) -> String {
        self.property("id")
    }

    pub fn set_id(&self, value: &str) {
        self.set_property("id", value);
    }

    pub fn get_title(&self) -> String {
        self.property("title")
    }

    pub fn set_title(&self, value: &str) {
        self.set_property("title", value)
    }

    pub fn get_description(&self) -> Option<String> {
        self.property("description")
    }

    pub fn set_description(&self, value: Option<&str>) {
        self.set_property("description", value)
    }

    pub fn get_icon(&self) -> Option<String> {
        self.property("icon")
    }

    pub fn set_icon(&self, value: Option<&str>) {
        self.set_property("icon", value)
    }

    pub fn get_use_pango(&self) -> bool {
        self.property("use-pango")
    }

    pub fn set_use_pango(&self, value: bool) {
        self.set_property("use-pango", value)
    }

    pub fn get_plugin_pid(&self) -> u64 {
        self.property("plugin-pid")
    }

    pub fn set_plugin_pid(&self, value: u64) {
        self.set_property("plugin-pid", value)
    }
}

impl Default for GMatch {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Match> for GMatch {
    fn from(value: Match) -> Self {
        let item = Self::new();

        item.set_id(&value.get_id());
        item.set_title(&value.title);
        item.set_description(value.description.as_deref());
        item.set_icon(value.icon.as_deref());
        item.set_use_pango(value.use_pango);

        // TODO what to do with plugin-pid?

        item
    }
}

impl From<GMatch> for Match {
    fn from(val: GMatch) -> Self {
        Match {
            id: val.get_id(),
            title: val.get_title(),
            description: val.get_description(),
            icon: val.get_icon(),
            use_pango: val.get_use_pango(),
        }
    }
}

impl From<GMatch> for gtk::Widget {
    fn from(value: GMatch) -> Self {
        use gtk::prelude::BoxExt;

        let hbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .height_request(36)
            .spacing(4)
            .build();

        let match_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .build();

        if let Some(icon) = value.get_icon() {
            match_box.append(&build_image(&icon));
        }

        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .build();

        vbox.append(&build_label(value.get_use_pango(), &value.get_title()));

        if let Some(desc) = value.get_description() {
            vbox.append(&build_label(value.get_use_pango(), &desc));
        }

        match_box.append(&vbox);
        hbox.append(&match_box);

        hbox.into()
    }
}
