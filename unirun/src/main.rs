mod gui;
mod types;
mod utils;

use std::{cell::RefCell, fs, rc::Rc};

use gtk::{
    glib::{self, clone},
    prelude::*,
};
#[allow(unused_imports)]
use log::*;
use types::RuntimeData;
use unirun_if::{
    constants::MAIN_APP_ID,
    package::{Command, Package, Payload},
    path,
    socket::{connect_and_write, stream_read, stream_write},
};
use utils::clear_entry_pool;

use crate::utils::{build_socket_service, launch_plugins};

pub const MAIN_WINDOW_TITLE: &str = "UniRun";

fn main() -> Result<(), glib::Error> {
    env_logger::init();

    ctrlc::set_handler(|| {
        info!("Ctrl-C shutdown");
        if let Err(e) = connect_and_write(Package::new(Payload::Command(Command::Quit))) {
            error!("Failed to send quit command: {}", e);
        }
    })
    .expect("Error setting Ctrl-C handler");

    let runtime_data = Rc::new(RefCell::new(RuntimeData::default()));

    let socket_service = build_socket_service(runtime_data.clone())?;
    socket_service.start();

    launch_plugins();

    let application = runtime_data.borrow().application.clone();

    application.connect_activate(clone!(
        #[strong]
        runtime_data,
        move |app| {
            info!("Application activate");

            if let Err(e) = gui::build_ui(app.clone(), runtime_data.clone()) {
                error!("Failed to build UI: {}", e);
                panic!("{}", e);
            }
        }
    ));

    application.connect_shutdown(move |_| {
        info!("Application shutdown");

        finalize_connections(runtime_data.clone());
        remove_socket_file();
    });

    application.run();

    Ok(())
}

fn finalize_connections(runtime_data: Rc<RefCell<RuntimeData>>) {
    clear_entry_pool(&mut runtime_data.borrow_mut());
    let connections = runtime_data.borrow().connections.clone();
    for connection in connections {
        trace!("SENDING QUIT");
        let _ = stream_write(
            &connection.output_stream(),
            Package::new(Payload::Command(Command::Quit)),
        );

        let _ = stream_read(&connection.input_stream());
    }
}

// FIXME
// spawn new unirun instance kills all instances
// this removes socket if there is another instance running
fn remove_socket_file() {
    let path = path::socket();
    if path.exists() {
        debug!("Removing socket file");
        if let Err(e) = fs::remove_file(&path) {
            error!("Failed to remove socket file: {}", e);
        }
    }
}
