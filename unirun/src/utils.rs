use std::{
    cell::RefCell, env::current_exe, fs::read_dir, os::unix::fs::PermissionsExt, path::PathBuf,
    process::Command, rc::Rc,
};

use gtk::{gio, glib, prelude::*};
#[allow(unused_imports)]
use log::*;
use unirun_if::{path, socket::stream_read_future};

use crate::{gui::on_entry_changed, types::RuntimeData};

fn handle_socket_data(data: &str, runtime_data: Rc<RefCell<RuntimeData>>) {
    match data {
        "quit" => runtime_data.borrow().application.quit(),
        _ => warn!("Received unknown data: {:?}. Ignoring", data),
    }
}

pub fn build_socket_service(
    runtime_data: Rc<RefCell<RuntimeData>>,
) -> Result<gio::SocketService, glib::Error> {
    let socket_path = path::socket();
    let socket_service = gio::SocketService::new();

    socket_service.add_address(
        &gio::UnixSocketAddress::new(&socket_path),
        gio::SocketType::Stream,
        gio::SocketProtocol::Default,
        glib::Object::NONE,
    )?;

    socket_service.connect_incoming(move |_, connection, _| {
        handle_new_connection(connection.clone(), runtime_data.clone());
        true
    });

    Ok(socket_service)
}

fn handle_new_connection(
    connection: gio::SocketConnection,
    runtime_data: Rc<RefCell<RuntimeData>>,
) {
    let creds = connection.socket().credentials().unwrap_or_default();

    let pid = creds.unix_pid().expect("Failed to read process ID") as u64;
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

fn handle_new_connection_from_self(
    connection: impl IOStreamExt,
    runtime_data: Rc<RefCell<RuntimeData>>,
) {
    glib::spawn_future_local(async move {
        let data = stream_read_future(&connection.input_stream())
            .await
            .unwrap_or_else(|e| {
                error!("{}", e);
                panic!("{}", e);
            });

        handle_socket_data(&data, runtime_data);
    });
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

pub fn launch_plugins() {
    if let Ok(current_exe_path) = current_exe() {
        if let Some(current_dir) = current_exe_path.parent() {
            if let Ok(entries) = read_dir(current_dir) {
                let binaries_to_launch = entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| path.is_file())
                    .filter(|path| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .map_or(false, |name| name.starts_with("unirun-plugin"))
                    })
                    .filter(|path| {
                        path.metadata()
                            .map_or(false, |metadata| metadata.permissions().mode() & 0o111 != 0)
                    })
                    .collect::<Vec<_>>();

                for binary in binaries_to_launch {
                    if let Err(e) = Command::new(&binary).spawn() {
                        error!("Failed to launch {}: {}", binary.display(), e);
                    }
                }
            } else {
                error!("Failed to read directory: {}", current_dir.display());
            }
        } else {
            error!("Failed to get parent directory of current executable");
        }
    } else {
        error!("Failed to get current executable path");
    }
}
