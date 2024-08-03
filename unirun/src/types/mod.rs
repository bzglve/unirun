pub mod ghit;

use crate::MAIN_APP_ID;

use ghit::GHit;
use gtk::{gio, glib, Application};

pub struct RuntimeData {
    pub application: gtk::Application,
    pub connections: Vec<gio::SocketConnection>,
    pub entry_pool: Vec<glib::JoinHandle<()>>,
    pub hit_store: gio::ListStore,
}

impl Default for RuntimeData {
    fn default() -> Self {
        Self {
            application: Application::new(Some(MAIN_APP_ID), Default::default()),
            connections: Default::default(),
            entry_pool: Default::default(),
            hit_store: gio::ListStore::new::<GHit>(),
        }
    }
}
