use std::{cell::RefCell, rc::Rc};

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
    package::{Command, Package, Payload},
    socket::connect_and_write_future,
};

use crate::{
    types::{gmatch::GMatch, RuntimeData},
    utils::{handle_selection_activation, on_entry_changed},
    MAIN_WINDOW_TITLE,
};

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

fn build_window(app: impl IsA<gtk::Application>) -> gtk::ApplicationWindow {
    fn init_layer_shell(window: impl LayerShell) {
        use gtk_layer_shell::{Edge, KeyboardMode, Layer};

        window.init_layer_shell();
        window.set_layer(Layer::Overlay); // TODO move to config
        window.set_anchor(Edge::Top, true); // TODO move to config
        window.set_keyboard_mode(KeyboardMode::OnDemand); // TODO move to config
    }

    fn connect_window_key_press_events(
        widget: impl WidgetExt,
        event_controller_key: gtk::EventControllerKey,
    ) {
        connect_key_press_events(widget, event_controller_key, move |keyval| match keyval {
            Key::Escape => {
                glib::spawn_future_local(async move {
                    connect_and_write_future(Package::new(Payload::Command(Command::Quit)))
                        .await
                        .unwrap()
                });
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        });
    }

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
    fn connect_entry_key_press_events(
        widget: impl WidgetExt,
        event_controller_key: gtk::EventControllerKey,
    ) {
        connect_key_press_events(
            widget.clone(),
            event_controller_key,
            move |keyval| match keyval {
                Key::Escape => {
                    glib::spawn_future_local(async move {
                        connect_and_write_future(Package::new(Payload::Command(Command::Quit)))
                            .await
                            .unwrap()
                    });
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
