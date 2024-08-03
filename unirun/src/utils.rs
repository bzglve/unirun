use std::{
    cell::RefCell, env::current_exe, fs::read_dir, os::unix::fs::PermissionsExt, path::PathBuf,
    process, rc::Rc,
};

use gtk::{
    gio,
    glib::{self, clone},
    prelude::*,
};
#[allow(unused_imports)]
use log::*;
use unirun_if::{
    package::{Command, Hit, Package, Payload},
    path,
    socket::{connect_and_write_future, stream_read_future, stream_write_future},
};

use crate::types::{ghit::GHit, RuntimeData};

pub fn build_socket_service(
    runtime_data: Rc<RefCell<RuntimeData>>,
) -> Result<gio::SocketService, glib::Error> {
    fn handle_new_connection(
        connection: gio::SocketConnection,
        runtime_data: Rc<RefCell<RuntimeData>>,
    ) {
        fn handle_new_connection_from_self(
            connection: impl IOStreamExt,
            runtime_data: Rc<RefCell<RuntimeData>>,
        ) {
            fn handle_socket_data(data: &Payload, runtime_data: Rc<RefCell<RuntimeData>>) {
                if let Payload::Command(Command::Quit) = data {
                    runtime_data.borrow().application.quit()
                }
            }

            glib::spawn_future_local(async move {
                let data = stream_read_future(&connection.input_stream())
                    .await
                    .unwrap_or_else(|e| {
                        error!("{}", e);
                        panic!("{}", e);
                    });

                handle_socket_data(&data.payload, runtime_data);
            });
        }

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
    let mut image = gtk::Image::builder().pixel_size(32);
    let path = PathBuf::from(icon);

    image = if path.is_absolute() {
        image.file(path.to_string_lossy())
    } else {
        image.icon_name(icon)
    };
    image.build()
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
                    if let Err(e) = process::Command::new(&binary).spawn() {
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

pub fn on_entry_changed(text: &str, runtime_data: Rc<RefCell<RuntimeData>>) {
    fn clear_entry_pool(runtime_data: &mut RuntimeData) {
        let entry_pool = &runtime_data.entry_pool;

        if !entry_pool.is_empty() {
            warn!(
                "There is still {} running tasks. Aborting",
                entry_pool.len()
            );

            entry_pool.iter().for_each(|jh| jh.abort());
            runtime_data.entry_pool.clear();
        }
    }

    // fn filter_connections(runtime_data: &mut RuntimeData) {
    //     let connections = runtime_data.connections.clone();
    //     runtime_data.connections = connections
    //         .into_iter()
    //         .filter_map(|conn| {
    //             // TODO test
    //             if conn.is_connected() {
    //                 Some(conn)
    //             } else {
    //                 None
    //             }
    //         })
    //         .collect();
    // }

    let mut runtime_data = runtime_data.borrow_mut();

    clear_entry_pool(&mut runtime_data);
    // filter_connections(&mut runtime_data);

    let hit_store = runtime_data.hit_store.clone();
    hit_store.remove_all();

    let text = Rc::new(text.to_owned());

    for conn in runtime_data.connections.clone() {
        runtime_data
            .entry_pool
            .push(glib::spawn_future_local(clone!(
                #[strong]
                text,
                #[strong]
                hit_store,
                async move {
                    stream_write_future(
                        &conn.output_stream(),
                        Package::new(Payload::Command(Command::Abort)),
                    )
                    .await
                    .unwrap();

                    let request =
                        Package::new(Payload::Command(Command::GetData(text.to_string())));
                    stream_write_future(&conn.output_stream(), request.clone())
                        .await
                        .unwrap();

                    let request_id = request.get_id();
                    // FIXME is this workaround?
                    loop {
                        if let Payload::Result(Ok(response_id)) =
                            stream_read_future(&conn.input_stream())
                                .await
                                .unwrap()
                                .payload
                        {
                            if request_id == response_id {
                                break;
                            }
                        }
                    }

                    loop {
                        let response = stream_read_future(&conn.input_stream()).await.unwrap();
                        let response_id = response.get_id();
                        match response.payload {
                            Payload::Hit(h) => {
                                hit_store.append(&{
                                    let ghit = GHit::from(h);
                                    ghit.set_plugin_pid(
                                        conn.socket().credentials().unwrap().unix_pid().unwrap()
                                            as u64,
                                    );
                                    ghit
                                });

                                stream_write_future(
                                    &conn.output_stream(),
                                    Package::new(Payload::Result(Ok(response_id))),
                                )
                                .await
                                .unwrap();
                            }
                            Payload::Command(Command::Abort) => {
                                break;
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            )))
    }
}

pub fn handle_selection_activation(row_id: u32, runtime_data: Rc<RefCell<RuntimeData>>) {
    glib::spawn_future_local(async move {
        let ghit = runtime_data
            .borrow()
            .hit_store
            .item(row_id)
            .unwrap_or_else(|| panic!("Failed to get list_store item at {} position", row_id))
            .downcast::<GHit>()
            .expect("Failed to downcast Object to GHit");

        let plugin_pid = ghit.get_plugin_pid();

        let connections = runtime_data.borrow().connections.clone();
        let connection = connections
            .iter()
            .find(|conn| {
                conn.socket().credentials().unwrap().unix_pid().unwrap() as u64 == plugin_pid
            })
            .unwrap();

        let hit: Hit = ghit.clone().into();
        let request = Package::new(Payload::Command(Command::Activate(hit.id.to_owned())));
        stream_write_future(&connection.output_stream(), request.clone())
            .await
            .unwrap();
        let request_id = request.get_id();

        let response = stream_read_future(&connection.input_stream())
            .await
            .unwrap();

        if let Payload::Result(Ok(response_id)) = response.payload {
            if response_id == request_id {
                connect_and_write_future(Package::new(Payload::Command(Command::Quit)))
                    .await
                    .unwrap();
            }
        }
    });
}
