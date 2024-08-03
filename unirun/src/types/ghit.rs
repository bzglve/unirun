/// link to the [source](https://gtk-rs.org/gtk-rs-core/stable/latest/docs/glib/subclass/index.html)
use gtk::{
    glib::{self, subclass::prelude::*},
    prelude::{ObjectExt, ToValue},
};
use std::cell::{Cell, RefCell};
use unirun_if::package::{Hit, HitId};

use crate::utils::{build_image, build_label};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct GHit {
        id: RefCell<String>,
        title: RefCell<String>,
        description: RefCell<Option<String>>,
        icon: RefCell<Option<String>>,
        use_pango: Cell<bool>,
        plugin_pid: Cell<u64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GHit {
        const NAME: &'static str = "GHit";

        type Type = super::GHit;
    }

    impl ObjectImpl for GHit {
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
                    self.id.replace(
                        value
                            .get()
                            .expect("type conformity checked by `Object::set_property`"),
                    );
                }
                "title" => {
                    self.title.replace(
                        value
                            .get()
                            .expect("type conformity checked by `Object::set_property`"),
                    );
                }
                "description" => {
                    self.description.replace(
                        value
                            .get()
                            .expect("type conformity checked by `Object::set_property`"),
                    );
                }
                "icon" => {
                    self.icon.replace(
                        value
                            .get()
                            .expect("type conformity checked by `Object::set_property`"),
                    );
                }
                "use-pango" => {
                    self.use_pango.replace(
                        value
                            .get()
                            .expect("type conformity checked by `Object::set_property`"),
                    );
                }
                "plugin-pid" => {
                    self.plugin_pid.replace(
                        value
                            .get()
                            .expect("type conformity checked by `Object::set_property`"),
                    );
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
            self.parent_constructed();
        }
    }
}

glib::wrapper! {
    pub struct GHit(ObjectSubclass<imp::GHit>);
}

// TODO does we need so much setters-getters? Is there any way to simplify this
impl GHit {
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

impl Default for GHit {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Hit> for GHit {
    fn from(value: Hit) -> Self {
        let item = Self::new();

        item.set_id(&value.id.to_string());
        item.set_title(&value.title);
        item.set_description(value.description.as_deref());
        item.set_icon(value.icon.as_deref());
        item.set_use_pango(value.use_pango);

        // TODO Handle plugin-pid if needed

        item
    }
}

impl From<GHit> for Hit {
    fn from(val: GHit) -> Self {
        Self {
            id: HitId::from(val.get_id().as_str()),
            title: val.get_title(),
            description: val.get_description(),
            icon: val.get_icon(),
            use_pango: val.get_use_pango(),
        }
    }
}

impl From<GHit> for gtk::Widget {
    fn from(value: GHit) -> Self {
        use gtk::prelude::BoxExt;

        let hbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .height_request(36)
            .spacing(4)
            .build();

        let hit_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .build();

        if let Some(icon) = value.get_icon() {
            hit_box.append(&build_image(&icon));
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

        hit_box.append(&vbox);
        hbox.append(&hit_box);

        hbox.into()
    }
}
