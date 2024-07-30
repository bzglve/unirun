use std::{cell::RefCell, path::PathBuf, rc::Rc};

use gtk::{
    gio,
    glib::{self, clone},
    prelude::IOStreamExt,
};
#[allow(unused_imports)]
use log::*;
use unirun_if::{path, socket::stream_read_async};

use crate::{gui::on_entry_changed, types::RuntimeData};

pub fn handle_socket_data(data: &str, runtime_data: Rc<RefCell<RuntimeData>>) {
    use gtk::prelude::ApplicationExt;

    match data {
        "quit" => {
            if let Some(app) = &runtime_data.borrow().application {
                app.quit()
            }
        }
        _ => warn!("Received unknown data: ({:?}). Ignoring", data),
    }
}

pub fn build_socket_service(runtime_data: Rc<RefCell<RuntimeData>>) -> gio::SocketService {
    use gio::prelude::{SocketListenerExt, SocketServiceExt};

    debug!("Building socket service");

    let socket_path = path::socket();
    debug!("socket_path={}", socket_path.to_string_lossy());

    let socket_service = gio::SocketService::new();

    socket_service
        .add_address(
            &gio::UnixSocketAddress::new(&socket_path),
            gio::SocketType::Stream,
            gio::SocketProtocol::Default,
            glib::Object::NONE,
        )
        .unwrap_or_else(|e| {
            error!("{}", e);
            panic!()
        });

    socket_service.connect_incoming(move |_service, connection, _obj| {
        debug!("Got new connection");
        handle_new_connection(connection, runtime_data.clone());
        true
    });

    socket_service
}

pub fn handle_new_connection(
    connection: &gio::SocketConnection,
    runtime_data: Rc<RefCell<RuntimeData>>,
) {
    use gio::prelude::{SocketConnectionExt, SocketExt};

    let creds = connection.socket().credentials().unwrap_or_default();
    debug!("Credentials: {:#?}", creds.to_str());

    let pid = creds.unix_pid().expect("Failed to read proceess Id") as u64;
    if std::process::id() as u64 == pid {
        handle_new_connection_from_self(connection, runtime_data.clone());
    } else {
        runtime_data
            .borrow_mut()
            .connections
            .push(connection.clone());
        on_entry_changed("", runtime_data.clone());
    }
}

pub fn handle_new_connection_from_self(
    connection: &impl IOStreamExt,
    runtime_data: Rc<RefCell<RuntimeData>>,
) {
    // TODO test with `glib::spawn_future_local()` and blocking
    stream_read_async(
        &connection.input_stream(),
        clone!(
            #[strong]
            runtime_data,
            move |data| {
                let data = data.unwrap_or_else(|d| {
                    error!("{}", d);
                    panic!()
                });
                debug!("self> {:?}", data);

                handle_socket_data(data, runtime_data.clone());
            }
        ),
    );
}

pub fn build_label(use_markup: bool, label: &str) -> gtk::Label {
    gtk::Label::builder()
        .wrap_mode(gtk::pango::WrapMode::Char)
        .wrap(true)
        .xalign(0.0)
        .use_markup(use_markup)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .label(label)
        .build()
}

pub fn build_image(icon: &str) -> gtk::Image {
    let mut match_image = gtk::Image::builder().pixel_size(32);

    let path = PathBuf::from(icon);

    match_image = if path.is_absolute() {
        match_image.file(path.to_string_lossy())
    } else {
        match_image.icon_name(icon)
    };
    match_image.build()
}
