pub mod gmatch;

use crate::MAIN_APP_ID;

use gmatch::GMatch;
use gtk::{gio, glib, Application};

pub struct RuntimeData {
    pub application: gtk::Application,
    pub connections: Vec<gio::SocketConnection>,
    pub entry_pool: Vec<glib::JoinHandle<()>>,
    pub match_store: gio::ListStore,
}

impl Default for RuntimeData {
    fn default() -> Self {
        Self {
            application: Application::new(Some(MAIN_APP_ID), Default::default()),
            connections: Default::default(),
            entry_pool: Default::default(),
            match_store: gio::ListStore::new::<GMatch>(),
        }
    }
}
