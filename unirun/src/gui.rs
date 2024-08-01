use std::{cell::RefCell, error::Error, rc::Rc};

use gtk::{
    gdk::Key,
    gio,
    glib::{self, clone},
    prelude::*,
};
use gtk_layer_shell::LayerShell;
#[allow(unused_imports)]
use log::*;
use unirun_if::{
    match_if::Match,
    socket::{connect_and_write_future, stream_read_future, stream_write_future},
};

use crate::{
    types::{gmatch::GMatch, RuntimeData},
    MAIN_WINDOW_TITLE,
};

fn init_layer_shell(window: impl LayerShell) {
    use gtk_layer_shell::{Edge, KeyboardMode, Layer};

    window.init_layer_shell();
    window.set_layer(Layer::Overlay); // TODO move to config
    window.set_anchor(Edge::Top, true); // TODO move to config
    window.set_keyboard_mode(KeyboardMode::OnDemand); // TODO move to config
}

fn connect_key_press_events<F>(
    widget: impl WidgetExt,
    event_controller_key: gtk::EventControllerKey,
    handler: F,
) where
    F: Fn(Key) -> glib::Propagation + 'static,
{
    widget.add_controller(event_controller_key.clone());
    event_controller_key.connect_key_pressed(move |_, keyval, _, _| handler(keyval));
}

fn connect_window_key_press_events(
    widget: impl WidgetExt,
    event_controller_key: gtk::EventControllerKey,
) {
    connect_key_press_events(widget, event_controller_key, move |keyval| match keyval {
        Key::Escape => {
            glib::spawn_future_local(
                async move { connect_and_write_future("quit").await.unwrap() },
            );
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
}

fn connect_entry_key_press_events(
    widget: impl WidgetExt,
    event_controller_key: gtk::EventControllerKey,
) {
    connect_key_press_events(
        widget.clone(),
        event_controller_key,
        move |keyval| match keyval {
            Key::Escape => {
                glib::spawn_future_local(
                    async move { connect_and_write_future("quit").await.unwrap() },
                );
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

fn build_window(app: impl IsA<gtk::Application>) -> gtk::ApplicationWindow {
    let window = gtk::ApplicationWindow::new(&app);
    window.set_title(Some(MAIN_WINDOW_TITLE));
    window.set_default_size(650, 500); // TODO move to config?
    init_layer_shell(window.clone());

    let window_eck = gtk::EventControllerKey::new();
    connect_window_key_press_events(window.clone(), window_eck);

    window
}

fn build_entry<C, A>(on_change: C, on_activate: A) -> gtk::SearchEntry
where
    C: Fn(&str) + 'static,
    A: Fn() + 'static,
{
    let entry = gtk::SearchEntry::new();

    entry.connect_search_changed(move |entry| on_change(&entry.text()));
    entry.connect_activate(move |_| on_activate());

    let entry_eck = gtk::EventControllerKey::new();
    connect_entry_key_press_events(entry.clone(), entry_eck);

    entry
}

fn build_main_list<A>(model: impl IsA<gio::ListModel>, on_activate: A) -> gtk::ListBox
where
    A: Fn(&gtk::ListBoxRow) + 'static,
{
    let main_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .build();
    main_list.bind_model(Some(&model), move |match_row| {
        match_row
            .clone()
            .downcast::<GMatch>()
            .expect("Can't downcast glib::Object to GMatch")
            .into()
    });

    main_list.connect_row_activated(move |_, row| on_activate(row));

    model.connect_items_changed(clone!(
        #[strong]
        main_list,
        move |_, _, _, _| main_list.select_row(main_list.row_at_index(0).as_ref())
    ));

    main_list
}

pub fn build_ui(
    app: impl IsA<gtk::Application>,
    runtime_data: Rc<RefCell<RuntimeData>>,
) -> Result<(), glib::Error> {
    let main_list = build_main_list(
        runtime_data.borrow().match_store.clone(),
        clone!(
            #[strong]
            runtime_data,
            move |row| handle_selection_activation(row.index() as u32, runtime_data.clone())
        ),
    );

    let entry = build_entry(
        clone!(
            #[strong]
            runtime_data,
            move |text| on_entry_changed(text, runtime_data.clone()),
        ),
        clone!(
            #[strong]
            main_list,
            #[strong]
            runtime_data,
            move || {
                if let Some(row) = main_list.selected_row() {
                    handle_selection_activation(row.index() as u32, runtime_data.clone())
                }
            }
        ),
    );
    entry.grab_focus();

    let scroll_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .min_content_width(340)
        .focusable(true)
        .child(&main_list)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
    vbox.append(&entry.clone());
    vbox.append(&scroll_window);

    let window = build_window(app);
    window.set_child(Some(&vbox));
    window.present();

    info!("UI built and presented");

    Ok(())
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

    async fn handle_stream_message(
        conn: &gio::SocketConnection,
        match_store: &gio::ListStore,
    ) -> Result<(), Box<dyn Error>> {
        let s = stream_read_future(&conn.input_stream()).await?;
        let m = serde_json::from_str::<Match>(&s)?;

        match_store.append(&{
            let gmatch = GMatch::from(m);
            gmatch.set_plugin_pid(conn.socket().credentials()?.unix_pid()? as u64);
            gmatch
        });

        Ok(())
    }

    let mut runtime_data = runtime_data.borrow_mut();

    clear_entry_pool(&mut runtime_data);
    // filter_connections(&mut runtime_data);

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
                    stream_write_future(&conn.output_stream(), "abort")
                        .await
                        .unwrap();

                    stream_write_future(&conn.output_stream(), format!("get_data,{}", text))
                        .await
                        .unwrap();

                    // FIXME is this workaround?
                    let mut response = String::new();
                    while !response.starts_with("ok:") {
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

                    for _ in 0..count {
                        match handle_stream_message(&conn, &match_store).await {
                            Ok(_) => stream_write_future(&conn.output_stream(), "ok")
                                .await
                                .unwrap(),
                            Err(e) => {
                                error!("Error handling stream message: {}", e);
                                stream_write_future(&conn.output_stream(), "err")
                                    .await
                                    .unwrap()
                            }
                        };
                    }
                }
            )))
    }
}

fn handle_selection_activation(row_id: u32, runtime_data: Rc<RefCell<RuntimeData>>) {
    glib::spawn_future_local(async move {
        let gmatch = runtime_data
            .borrow()
            .match_store
            .item(row_id)
            .unwrap_or_else(|| panic!("Failed to get list_store item at {} position", row_id))
            .downcast::<GMatch>()
            .expect("Failed to downcast Object to MatchRow");

        let plugin_pid = gmatch.get_plugin_pid();

        let connections = runtime_data.borrow().connections.clone();
        let connection = connections
            .iter()
            .find(|conn| {
                conn.socket().credentials().unwrap().unix_pid().unwrap() as u64 == plugin_pid
            })
            .unwrap();

        let rmatch: Match = gmatch.clone().into();
        stream_write_future(
            &connection.output_stream(),
            format!("activate,{}", rmatch.get_id()),
        )
        .await
        .unwrap();

        let response = stream_read_future(&connection.input_stream())
            .await
            .unwrap();

        if response == "ok" {
            connect_and_write_future("quit").await.unwrap();
        }
    });
}
