mod app_info;

use std::{cell::RefCell, pin::Pin, rc::Rc};

use app_info::AppInfo;
use gio::{prelude::*, SocketConnection};
use glib::{self, clone};
#[allow(unused_imports)]
use log::*;
use unirun_if::{
    package::{Command, Hit, Package, PackageId, Payload},
    socket::{connection, stream_read_future, stream_write_future},
};

fn handle_get_data<'a>(
    hits: Vec<Hit>,
    pack_id: PackageId,
    connection: &'a SocketConnection,
    main_loop: &'a glib::MainLoop,
) -> Pin<Box<dyn std::future::Future<Output = ()> + 'a>> {
    Box::pin(async move {
        let pack = Package::new(Payload::Result(Ok(pack_id)));
        debug!("Sending {:?}", pack);
        stream_write_future(&connection.output_stream(), pack)
            .await
            .unwrap();

        let mut i = 0;
        while i < hits.len() {
            let h = hits.get(i).unwrap();
            let pack = Package::new(Payload::Hit(h.clone()));

            debug!("Sending {}", h);
            stream_write_future(&connection.output_stream(), pack)
                .await
                .unwrap();

            let response = stream_read_future(&connection.input_stream()).await;

            if response.is_err() {
                main_loop.quit();
            }
            let response = response.unwrap();

            debug!("Got response: {:?}", response);

            match response.payload {
                Payload::Command(Command::Abort) => {
                    // FIXME workaround
                    warn!("ABORTING");
                    connection.output_stream().clear_pending();
                    return;
                }
                Payload::Result(Err(_)) => {
                    continue;
                }
                Payload::Result(Ok(_)) => {}
                _ => unreachable!(),
            };

            i += 1;
        }

        stream_write_future(
            &connection.output_stream(),
            Package::new(Payload::Command(Command::Abort)),
        )
        .await
        .unwrap();
    })
}

async fn handle_command(
    command: Command,
    pack_id: PackageId,
    hits: Rc<RefCell<Vec<(AppInfo, Hit)>>>,
    connection: &SocketConnection,
    main_loop: &glib::MainLoop,
) {
    match command {
        Command::GetData(text) => {
            *hits.borrow_mut() = (if text.is_empty() {
                AppInfo::all()
            } else {
                AppInfo::search(&text)
            })
            .into_iter()
            .map(|app_info| (app_info.clone(), Hit::from(app_info)))
            .collect();

            let hits_right = hits.borrow().clone().into_iter().map(|(_, h)| h).collect();
            handle_get_data(hits_right, pack_id, connection, &main_loop.clone()).await;
        }
        Command::Activate(id) => {
            if let Some(app_info) =
                hits.borrow()
                    .iter()
                    .find_map(|(a, h)| if h.id == id { Some(a) } else { None })
            {
                if let Some(id) = &app_info.id {
                    info!("Launching: {}", id);
                    match gio::DesktopAppInfo::new(id)
                        .unwrap()
                        .launch(&[], gio::AppLaunchContext::NONE)
                    {
                        Ok(_) => stream_write_future(
                            &connection.output_stream(),
                            Package::new(Payload::Result(Ok(pack_id))),
                        )
                        .await
                        .unwrap(),
                        Err(_) => stream_write_future(
                            &connection.output_stream(),
                            Package::new(Payload::Result(Err(pack_id))),
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

    let hits = Rc::new(RefCell::new(Vec::new()));
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

                match &data.payload {
                    Payload::Command(command) => {
                        handle_command(
                            command.clone(),
                            data.get_id(),
                            hits.clone(),
                            &conn,
                            &main_loop,
                        )
                        .await
                    }
                    _ => unreachable!(),
                }
            }
        }
    ));

    main_loop.run();
    Ok(())
}
