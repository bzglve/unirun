mod app_info;

use std::{cell::RefCell, pin::Pin, rc::Rc};

use app_info::AppInfo;
use gio::{prelude::*, SocketConnection};
use glib::{self, clone};
#[allow(unused_imports)]
use log::*;
use unirun_if::{
    package::{match_if::Match, Command, Package},
    socket::{connection, stream_read_future, stream_write_future},
};

fn handle_get_data<'a>(
    matches: Vec<Match>,
    connection: &'a SocketConnection,
    main_loop: &'a glib::MainLoop,
) -> Pin<Box<dyn std::future::Future<Output = ()> + 'a>> {
    Box::pin(async move {
        let pack = Package::Result(Ok(()));
        debug!("Sending {:?}", pack);
        stream_write_future(&connection.output_stream(), pack)
            .await
            .unwrap();

        let mut i = 0;
        while i < matches.len() {
            let m = matches.get(i).unwrap();
            let pack = Package::Match(m.clone());

            debug!("Sending {}", m);
            stream_write_future(&connection.output_stream(), pack)
                .await
                .unwrap();

            let response = stream_read_future(&connection.input_stream()).await;

            if response.is_err() {
                main_loop.quit();
            }
            let response = response.unwrap();

            debug!("Got response: {:?}", response);

            match response {
                Package::Command(Command::Abort) => {
                    // FIXME workaround
                    warn!("ABORTING");
                    connection.output_stream().clear_pending();
                    return;
                }
                Package::Result(Err(_)) => {
                    continue;
                }
                Package::Result(Ok(_)) => {}
                _ => unreachable!(),
            };

            i += 1;
        }

        stream_write_future(
            &connection.output_stream(),
            Package::Command(Command::Abort),
        )
        .await
        .unwrap();
    })
}

async fn handle_command(
    command: Command,
    matches: Rc<RefCell<Vec<(AppInfo, Match)>>>,
    connection: &SocketConnection,
    main_loop: &glib::MainLoop,
) {
    match command {
        Command::GetData(text) => {
            *matches.borrow_mut() = (if text.is_empty() {
                AppInfo::all()
            } else {
                AppInfo::search(&text)
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
            handle_get_data(mt, connection, &main_loop.clone()).await;
        }
        Command::Activate(id) => {
            if let Some(app_info) =
                matches
                    .borrow()
                    .iter()
                    .find_map(|(a, m)| if m.get_id() == id { Some(a) } else { None })
            {
                if let Some(id) = &app_info.id {
                    info!("Launching: {}", id);
                    match gio::DesktopAppInfo::new(id)
                        .unwrap()
                        .launch(&[], gio::AppLaunchContext::NONE)
                    {
                        Ok(_) => stream_write_future(
                            &connection.output_stream(),
                            Package::Result(Ok(())),
                        )
                        .await
                        .unwrap(),
                        Err(_) => stream_write_future(
                            &connection.output_stream(),
                            Package::Result(Err(())),
                        )
                        .await
                        .unwrap(),
                    };
                }
            }
        }
        Command::Abort => {}
        _ => unreachable!(),
    }
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

                let data = match stream_read_future(&conn.input_stream()).await {
                    Ok(d) => d,
                    Err(_) => {
                        // error!("Failed to read data: {}", e);
                        main_loop.quit();
                        continue;
                    }
                };
                debug!("Received: {:?}", data);

                match data {
                    Package::Command(command) => {
                        handle_command(command, matches.clone(), &conn, &main_loop).await
                    }
                    _ => unreachable!(),
                }
            }
        }
    ));

    main_loop.run();
    Ok(())
}
