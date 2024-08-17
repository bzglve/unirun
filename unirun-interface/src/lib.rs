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

    use crate::{constants::SOCKET_BUFFER_SIZE, package::Package};

    fn bytes_to_string(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes)
            .trim_end_matches(char::from(0))
            .trim()
            .to_string()
    }

    fn create_buffer(value: &[u8]) -> [u8; SOCKET_BUFFER_SIZE] {
        let mut buffer = [0; SOCKET_BUFFER_SIZE];
        let len = value.len().min(SOCKET_BUFFER_SIZE);
        buffer[..len].copy_from_slice(&value[..len]);
        buffer
    }

    pub type Stream = GStream;

    #[derive(Clone)]
    pub struct GStream {
        inner: gio::SocketConnection,
        pub credentials: Option<Credentials>,
    }

    impl GStream {
        pub fn new() -> Result<Self, glib::Error> {
            use gio::prelude::{SocketClientExt, SocketConnectionExt};

            let socket_path = crate::path::socket();
            let inner = gio::SocketClient::new().connect(
                &gio::UnixSocketAddress::new(&socket_path),
                gio::Cancellable::NONE,
            )?;
            let credentals = Self::credentials(inner.socket());
            Ok(Self {
                inner,
                credentials: credentals,
            })
        }

        pub async fn new_future() -> Result<Self, glib::Error> {
            use gio::prelude::{SocketClientExt, SocketConnectionExt};

            let socket_path = crate::path::socket();
            let inner = gio::SocketClient::new()
                .connect_future(&gio::UnixSocketAddress::new(&socket_path))
                .await?;
            let credentals = Self::credentials(inner.socket());
            Ok(Self {
                inner,
                credentials: credentals,
            })
        }

        pub fn read(&self) -> Result<Package, Box<dyn Error>> {
            use gio::prelude::{IOStreamExt, InputStreamExt};

            let stream = self.inner.input_stream();

            let buffer = stream.read_bytes(SOCKET_BUFFER_SIZE, gio::Cancellable::NONE)?;
            let json = bytes_to_string(&buffer);
            let package = serde_json::from_str::<Package>(&json)?;
            Ok(package)
        }

        pub async fn read_future(&self) -> Result<Package, Box<dyn Error>> {
            use gio::prelude::{IOStreamExt, InputStreamExt};

            let stream = self.inner.input_stream();

            let buffer = stream
                .read_bytes_future(SOCKET_BUFFER_SIZE, glib::Priority::DEFAULT)
                .await?;
            let json = bytes_to_string(&buffer);
            let package = serde_json::from_str::<Package>(&json)?;
            Ok(package)
        }

        pub fn write(&self, package: Package) -> Result<(), Box<dyn Error>> {
            use gio::prelude::{IOStreamExt, OutputStreamExt};

            let stream = self.inner.output_stream();

            let json = serde_json::to_string(&package)?;
            let buffer = create_buffer(json.as_ref());
            stream.write_bytes(&glib::Bytes::from(&buffer), gio::Cancellable::NONE)?;
            Ok(())
        }

        pub async fn write_future(&self, package: Package) -> Result<(), Box<dyn Error>> {
            use gio::prelude::{IOStreamExt, OutputStreamExt};

            let stream = self.inner.output_stream();

            let json = serde_json::to_string(&package)?;
            let buffer = create_buffer(json.as_ref());
            stream
                .write_bytes_future(&glib::Bytes::from(&buffer), glib::Priority::DEFAULT)
                .await?;
            Ok(())
        }

        fn credentials(socket: gio::Socket) -> Option<Credentials> {
            use gio::prelude::SocketExt;

            match socket.credentials() {
                Ok(c) => Credentials::try_from(c).ok(),
                Err(_) => None,
            }
        }
    }

    impl From<gio::SocketConnection> for GStream {
        fn from(value: gio::SocketConnection) -> Self {
            use gio::prelude::SocketConnectionExt;

            Self {
                inner: value.clone(),
                credentials: Self::credentials(value.socket()),
            }
        }
    }

    #[derive(Debug, Clone, Copy, Default)]
    pub struct Credentials {
        pub uid: u32,
        pub gid: i32,
        pub pid: Option<u32>,
    }

    impl TryFrom<gio::Credentials> for Credentials {
        type Error = glib::Error;

        fn try_from(value: gio::Credentials) -> Result<Self, Self::Error> {
            Ok(Self {
                uid: value.unix_user()?,
                gid: value.unix_pid()?,
                pid: Some(value.unix_pid()? as u32),
            })
        }
    }
}
