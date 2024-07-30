mod gui;
mod types;
mod utils;

use std::{cell::RefCell, fs, rc::Rc};

use gtk::{prelude::*, Application};
#[allow(unused_imports)]
use log::*;
use types::RuntimeData;
use unirun_if::{constants::MAIN_APP_ID, path, socket::connect_and_write};

pub const MAIN_WINDOW_TITLE: &str = "UniRun";

fn main() {
    env_logger::init();

    ctrlc::set_handler(move || {
        debug!("Ctrl-C shutdown");
        connect_and_write("quit").unwrap();
    })
    .unwrap_or_else(|e| {
        error!("{}", e);
        panic!()
    });

    debug!("Starting");

    let runtime_data = Rc::new(RefCell::new(RuntimeData::default()));

    let application = Rc::new(Application::new(Some(MAIN_APP_ID), Default::default()));

    runtime_data
        .borrow_mut()
        .application
        .replace(application.clone());

    application.connect_activate(move |app| {
        debug!("Application activate");

        gui::build_ui(app, runtime_data.clone());
    });

    application.connect_shutdown(|_app| {
        debug!("Application shutdown");

        // FIXME
        // spawn new unuirun instance kills all instances
        // this removes socket if there is another instance running
        let path = path::socket();
        if path.exists() {
            debug!("Removing socket file");
            fs::remove_file(path).unwrap();
        }
    });

    debug!("Application run");
    application.run();

    debug!("Ending");
}
