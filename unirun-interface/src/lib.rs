pub mod package;

pub mod constants {
    pub const DOMAIN: &str = "com.bzglve";
    pub const MAIN_APP_ID: &str = "com.bzglve.unirun";
    // TODO need to coordinate socket buffer size between processes instead of using const default
    // 1024 is 1KiB
    pub const SOCKET_BUFFER_SIZE: usize = 1024 * 4;
}

pub mod path {
    use std::{fs, path::PathBuf};

    use crate::constants::{DOMAIN, MAIN_APP_ID};

    pub fn runtime() -> PathBuf {
        let path = glib::user_runtime_dir().join(DOMAIN);
        if !path.exists() {
            fs::create_dir_all(&path).unwrap_or_else(|_| {
                panic!("Failed to create runtime directory at {}", path.display())
            });
        }
        path
    }

    pub fn socket() -> PathBuf {
        runtime().join(format!("{}.sock", MAIN_APP_ID))
    }
}

pub mod socket {
    use std::error::Error;

    use gio::{
        prelude::{InputStreamExt, OutputStreamExt},
        SocketConnection,
    };

    use crate::{bytes_to_string, constants::SOCKET_BUFFER_SIZE, package::Package};

    fn create_buffer(value: &[u8]) -> [u8; SOCKET_BUFFER_SIZE] {
        let mut buffer = [0; SOCKET_BUFFER_SIZE];
        let len = value.len().min(SOCKET_BUFFER_SIZE);
        buffer[..len].copy_from_slice(&value[..len]);
        buffer
    }

    // TODO DRY `stream_read_future`
    pub fn stream_read(stream: &impl InputStreamExt) -> Result<Package, Box<dyn Error>> {
        let buffer = stream.read_bytes(SOCKET_BUFFER_SIZE, gio::Cancellable::NONE)?;
        let json = bytes_to_string(&buffer);
        let package = serde_json::from_str::<Package>(&json)?;
        Ok(package)
    }

    // TODO DRY `stream_read`
    pub async fn stream_read_future(
        stream: &impl InputStreamExt,
    ) -> Result<Package, Box<dyn Error>> {
        let buffer = stream
            .read_bytes_future(SOCKET_BUFFER_SIZE, glib::Priority::DEFAULT)
            .await?;
        let json = bytes_to_string(&buffer);
        let package = serde_json::from_str::<Package>(&json)?;
        Ok(package)
    }

    pub fn stream_write(
        stream: &impl OutputStreamExt,
        value: Package,
    ) -> Result<isize, Box<dyn Error>> {
        let json = serde_json::to_string(&value)?;
        let buffer = create_buffer(json.as_ref());
        Ok(stream.write_bytes(&glib::Bytes::from(&buffer), gio::Cancellable::NONE)?)
    }

    pub async fn stream_write_future(
        stream: &impl OutputStreamExt,
        value: Package,
    ) -> Result<isize, Box<dyn Error>> {
        let json = serde_json::to_string(&value)?;
        let buffer = create_buffer(json.as_ref());
        Ok(stream
            .write_bytes_future(&glib::Bytes::from(&buffer), glib::Priority::DEFAULT)
            .await?)
    }

    pub fn connect_and_write(value: Package) -> Result<(), Box<dyn Error>> {
        use gio::prelude::IOStreamExt;

        let conn = connection()?;
        stream_write(&conn.output_stream(), value)?;
        Ok(())
    }

    pub async fn connect_and_write_future(value: Package) -> Result<(), Box<dyn Error>> {
        use gio::prelude::IOStreamExt;

        let conn = connection_future().await?;
        stream_write_future(&conn.output_stream(), value).await?;
        Ok(())
    }

    pub fn connection() -> Result<SocketConnection, glib::Error> {
        use gio::prelude::SocketClientExt;

        let socket_path = crate::path::socket();
        gio::SocketClient::new().connect(
            &gio::UnixSocketAddress::new(&socket_path),
            gio::Cancellable::NONE,
        )
    }

    pub async fn connection_future() -> Result<SocketConnection, glib::Error> {
        use gio::prelude::SocketClientExt;

        let socket_path = crate::path::socket();
        gio::SocketClient::new()
            .connect_future(&gio::UnixSocketAddress::new(&socket_path))
            .await
    }
}

pub fn bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches(char::from(0))
        .trim()
        .to_string()
}
