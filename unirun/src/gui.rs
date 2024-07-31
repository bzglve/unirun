use std::{
    cell::RefCell, env::current_exe, fs::read_dir, os::unix::fs::PermissionsExt, process::Command,
    rc::Rc,
};

use gtk::{
    gdk::Key,
    glib::{self, clone},
    prelude::*,
};
use gtk_layer_shell::{KeyboardMode, LayerShell};
#[allow(unused_imports)]
use log::*;
use unirun_if::{
    match_if::Match,
    socket::{
        connect_and_write, connect_and_write_future, stream_read_future, stream_write_future,
    },
};

use crate::{
    types::{gmatch::GMatch, RuntimeData},
    utils::build_socket_service,
    MAIN_WINDOW_TITLE,
};

pub fn init_layer_shell(window: &impl LayerShell) {
    use gtk_layer_shell::{Edge, Layer};

    window.init_layer_shell();

    window.set_layer(Layer::Overlay); // TODO move to config

    window.set_anchor(Edge::Top, true); // TODO move to config

    window.set_keyboard_mode(KeyboardMode::OnDemand); // TODO move to config
}

fn connect_key_press_events<F>(
    widget: Rc<impl WidgetExt>,
    event_controller_key: gtk::EventControllerKey,
    handler: F,
) where
    F: Fn(Key) -> glib::Propagation + 'static,
{
    widget.add_controller(event_controller_key.clone());
    event_controller_key.connect_key_pressed(move |_, keyval, _, _| handler(keyval));
}

fn connect_window_key_press_events(
    widget: Rc<impl WidgetExt>,
    event_controller_key: gtk::EventControllerKey,
) {
    connect_key_press_events(widget, event_controller_key, move |keyval| match keyval {
        Key::Escape => {
            // TODO this better be non-blocking
            connect_and_write("quit").unwrap();
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
}

fn connect_entry_key_press_events(
    widget: Rc<impl WidgetExt>,
    event_controller_key: gtk::EventControllerKey,
) {
    connect_key_press_events(
        widget.clone(),
        event_controller_key,
        move |keyval| match keyval {
            Key::Escape => {
                // TODO this better be non-blocking
                connect_and_write("quit").unwrap();
                glib::Propagation::Stop
            }
            Key::Down | Key::Up => {
                widget.emit_move_focus(if keyval == Key::Down {
                    gtk::DirectionType::TabForward
                } else {
                    gtk::DirectionType::TabBackward
                });
                glib::Propagation::Proceed
            }
            _ => glib::Propagation::Proceed,
        },
    );
}

pub fn build_ui(app: &impl IsA<gtk::Application>, runtime_data: Rc<RefCell<RuntimeData>>) {
    debug!("Bulding UI");

    let window = gtk::ApplicationWindow::new(app);
    window.set_title(Some(MAIN_WINDOW_TITLE));
    window.set_default_size(650, 500); // TODO move to config?

    init_layer_shell(&window);

    let window_eck = gtk::EventControllerKey::new();
    connect_window_key_press_events(window.clone().into(), window_eck);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 5);

    let entry = Rc::new(gtk::SearchEntry::new());
    entry.connect_search_changed(clone!(
        #[strong]
        runtime_data,
        move |entry| on_entry_changed(&entry.text(), runtime_data.clone())
    ));

    let entry_eck = gtk::EventControllerKey::new();
    connect_entry_key_press_events(entry.clone(), entry_eck);

    vbox.append(&*entry);

    let match_store = runtime_data.borrow().match_store.clone();

    let main_list = Rc::new(
        gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .build(),
    );
    main_list.bind_model(Some(&*match_store), move |match_row| {
        match_row
            .clone()
            .downcast::<GMatch>()
            .expect("Can't downcast glib::Object to GMatch")
            .into()
    });

    match_store.connect_items_changed(clone!(
        #[strong]
        main_list,
        move |_, _, _, _| main_list.select_row(main_list.row_at_index(0).as_ref())
    ));

    setup_activation(entry.clone(), main_list.clone(), runtime_data.clone());

    let scroll_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .min_content_width(340)
        .focusable(true)
        .build();
    scroll_window.set_child(Some(&*main_list));

    vbox.append(&scroll_window);

    window.set_child(Some(&vbox));

    entry.grab_focus();
    window.present();

    let socket_service = build_socket_service(runtime_data.clone());
    socket_service.start();

    let binding = current_exe().unwrap();
    let current_dir = binding.parent().unwrap();
    let binaries_to_launch = read_dir(current_dir)
        .unwrap()
        .filter_map(|dr| dr.ok())
        .map(|dr| dr.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("unirun-plugin")
        })
        .filter(|p| p.metadata().unwrap().permissions().mode() & 0o111 != 0)
        .collect::<Vec<_>>();

    binaries_to_launch.iter().for_each(|p| {
        Command::new(p).spawn().unwrap();
    });
}

pub fn setup_activation(
    entry: Rc<gtk::SearchEntry>,
    main_list: Rc<gtk::ListBox>,
    runtime_data: Rc<RefCell<RuntimeData>>,
) {
    entry.connect_activate(clone!(
        #[strong]
        main_list,
        #[weak]
        runtime_data,
        move |_| {
            if let Some(row) = main_list.selected_row() {
                handle_selection_activation(row.index().try_into().unwrap(), runtime_data.clone())
            }
        }
    ));

    main_list.connect_row_activated(clone!(
        #[weak]
        runtime_data,
        move |_, row| {
            handle_selection_activation(row.index().try_into().unwrap(), runtime_data.clone())
        }
    ));
}

pub fn on_entry_changed(text: &str, runtime_data: Rc<RefCell<RuntimeData>>) {
    fn clear_entry_pool(runtime_data: &mut RuntimeData) {
        let entry_pool = &runtime_data.entry_pool;

        if !entry_pool.is_empty() {
            debug!(
                "There is still {:?} running tasks. Aborting",
                entry_pool.len()
            );

            entry_pool
                .iter()
                .for_each(|jh: &glib::JoinHandle<_>| jh.abort());
            runtime_data.entry_pool.clear();
        }
    }

    fn filter_connections(runtime_data: &mut RuntimeData) {
        let connections = runtime_data.connections.clone();
        runtime_data.connections = connections
            .into_iter()
            .filter_map(|conn| {
                if conn.is_connected() {
                    Some(conn)
                } else {
                    None
                }
            })
            .collect();
    }

    debug!("Entry changed with: {:?}", text);

    let mut runtime_data = runtime_data.borrow_mut();

    clear_entry_pool(&mut runtime_data);
    filter_connections(&mut runtime_data);

    let match_store = runtime_data.match_store.clone();
    match_store.remove_all();

    let text = Rc::new(text.to_owned());

    for conn in runtime_data.connections.clone() {
        runtime_data
            .entry_pool
            .push(glib::spawn_future_local(clone!(
                #[strong]
                text,
                #[strong]
                match_store,
                async move {
                    debug!("Sending `abort`");
                    stream_write_future(&conn.output_stream(), "abort")
                        .await
                        .unwrap();

                    debug!(
                        "Sending `get_data,{}` to: {:?}",
                        text,
                        conn.socket().credentials().unwrap().to_str()
                    );
                    stream_write_future(&conn.output_stream(), format!("get_data,{}", text))
                        .await
                        .unwrap();

                    // FIXME is this workaround?
                    let mut response = String::new();
                    while !&response.starts_with("ok:") {
                        response = stream_read_future(&conn.input_stream()).await.unwrap();
                    }

                    let count = response
                        .trim_start_matches("ok:")
                        .trim()
                        .parse::<usize>()
                        .unwrap_or_else(|_| {
                            error!(
                                "Failed to read number of packages from {:?}. Using default",
                                response
                            );
                            0
                        });

                    debug!("Waiting for {} packages", count);

                    for i in 0..count {
                        let s = stream_read_future(&conn.input_stream()).await.unwrap();
                        let m = serde_json::from_str::<Match>(&s).unwrap_or_else(|e| {
                            error!("{}", e);
                            panic!()
                        });
                        debug!("[{}]: {:?}", i, m);

                        match_store.append(&{
                            let gmatch = GMatch::from(m);
                            gmatch.set_plugin_pid(
                                conn.socket().credentials().unwrap().unix_pid().unwrap() as u64,
                            );
                            gmatch
                        });

                        stream_write_future(&conn.output_stream(), "ok")
                            .await
                            .unwrap();
                    }
                }
            )))
    }
}

pub fn handle_selection_activation(row_id: usize, runtime_data: Rc<RefCell<RuntimeData>>) {
    glib::spawn_future_local(async move {
        let gmatch = runtime_data
            .borrow()
            .match_store
            .item(row_id.try_into().unwrap())
            .unwrap_or_else(|| panic!("Failed to get list_store item at {} position", row_id))
            .downcast::<GMatch>()
            .expect("Failed to downcast Object to MatchRow");

        let rmatch: Match = gmatch.clone().into();
        let plugin_pid = gmatch.get_plugin_pid();

        trace!("PLUGIN_ID: {:?}", plugin_pid);

        let connections = runtime_data.borrow().connections.clone();
        let connection = connections
            .iter()
            .find(|conn| {
                let creds = conn.socket().credentials().unwrap();
                trace!("CREDS: {:?}", creds.to_str());
                creds.unix_pid().unwrap() as u64 == plugin_pid
            })
            .unwrap();

        stream_write_future(
            &connection.output_stream(),
            format!("activate,{}", rmatch.get_id()),
        )
        .await
        .unwrap();

        let response = stream_read_future(&connection.input_stream())
            .await
            .unwrap();

        if &response == "ok" {
            connect_and_write_future("quit").await.unwrap();
        }
    });
}
