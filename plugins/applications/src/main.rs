mod app_info;

use std::rc::Rc;
use std::{cell::RefCell, pin::Pin};

use app_info::AppInfo;
use gio::{prelude::*, SocketConnection};
use glib::clone;
#[allow(unused_imports)]
use log::*;
use unirun_if::{
    match_if::Match,
    socket::{connection, stream_read_future, stream_write_future},
};

fn handle_get_data<'a>(
    matches: Vec<Match>,
    connection: &'a SocketConnection,
    main_loop: &'a glib::MainLoop,
) -> Pin<Box<dyn std::future::Future<Output = ()> + 'a>> {
    Box::pin(async move {
        let answer = format!("ok: {}", matches.len());
        debug!("Sending {:?}", answer);
        stream_write_future(&connection.output_stream(), answer)
            .await
            .unwrap();

        let mut i = 0;
        while i < matches.len() {
            let m = matches.get(i).unwrap();

            // FIXME somehow removing this debug line breaks sending concatenating previous send with next
            debug!("Sending {}", m);
            stream_write_future(
                &connection.output_stream(),
                serde_json::to_string(&m).unwrap(),
            )
            .await
            .unwrap();

            let response = stream_read_future(&connection.input_stream())
                .await
                .unwrap_or_else(|e| {
                    error!("{}", e);
                    "".to_string()
                });
            debug!("Got response: {:?}", response);

            // FIXME
            // This is not ideal.
            if response.starts_with("abort") {
                warn!("ABORTING");
                connection.output_stream().clear_pending();
                return;
            }

            if response.as_str() != "ok" {
                continue;
            }
            if response.is_empty() {
                // FIXME workaround
                // stop it from connection stop
                main_loop.quit();
            }

            i += 1;
        }
    })
}

fn main() -> Result<(), glib::Error> {
    env_logger::init();

    let matches = Rc::new(RefCell::new(Vec::new()));

    let main_loop = Rc::new(glib::MainLoop::new(None, true));

    let conn = connection()?;
    glib::spawn_future_local(clone!(
        #[strong]
        main_loop,
        async move {
            loop {
                debug!("Waiting for command");

                let data = stream_read_future(&conn.input_stream())
                    .await
                    .unwrap_or_else(|e| {
                        error!("{}", e);
                        "".to_string()
                    });
                debug!("Received: {:?}", data);

                if data.starts_with("get_data") {
                    let text = data.trim_start_matches("get_data,").trim();

                    *matches.borrow_mut() = (if text.is_empty() {
                        AppInfo::all()
                    } else {
                        AppInfo::search(text)
                    })
                    .into_iter()
                    .map(|app_info| (app_info.clone(), Match::from(app_info)))
                    .collect();

                    let mt = matches
                        .borrow()
                        .clone()
                        .into_iter()
                        .map(|(_, m)| m)
                        .collect();
                    handle_get_data(mt, &conn, &main_loop.clone()).await;
                }
                if data.starts_with("activate") {
                    let id = data.trim_start_matches("activate,").trim();
                    if let Some(app_info) = matches.borrow().iter().find_map(|(a, m)| {
                        if m.get_id() == id {
                            Some(a)
                        } else {
                            None
                        }
                    }) {
                        if let Some(id) = &app_info.id {
                            info!("Launching: {}", id);
                            match gio::DesktopAppInfo::new(id)
                                .unwrap()
                                .launch(&[], gio::AppLaunchContext::NONE)
                            {
                                Ok(_) => stream_write_future(&conn.output_stream(), "ok")
                                    .await
                                    .unwrap(),
                                Err(_) => stream_write_future(&conn.output_stream(), "err")
                                    .await
                                    .unwrap(),
                            };
                        }
                    }
                }
                if data.is_empty() {
                    // FIXME workaround
                    // stop it from connection stop
                    main_loop.quit();
                }
            }
        }
    ));

    main_loop.run();
    Ok(())
}