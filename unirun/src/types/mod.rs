pub mod gmatch;

use std::rc::Rc;

use gmatch::GMatch;
use gtk::{gio, glib};

pub struct RuntimeData {
    pub application: Option<Rc<gtk::Application>>,
    pub connections: Vec<gio::SocketConnection>,
    pub entry_pool: Vec<glib::JoinHandle<()>>,
    pub match_store: Rc<gio::ListStore>,
}

impl Default for RuntimeData {
    fn default() -> Self {
        Self {
            application: Default::default(),
            connections: Default::default(),
            entry_pool: Default::default(),
            match_store: Rc::new(gio::ListStore::new::<GMatch>()),
        }
    }
}
