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
    use gio::{
        prelude::{InputStreamExt, OutputStreamExt},
        SocketConnection,
    };

    use crate::{bytes_to_string, constants::SOCKET_BUFFER_SIZE};

    fn create_buffer(value: &[u8]) -> [u8; SOCKET_BUFFER_SIZE] {
        let mut buffer = [0; SOCKET_BUFFER_SIZE];
        let len = value.len().min(SOCKET_BUFFER_SIZE);
        buffer[..len].copy_from_slice(&value[..len]);
        buffer
    }

    pub fn stream_read(stream: &impl InputStreamExt) -> Result<String, glib::Error> {
        let bytes = stream.read_bytes(SOCKET_BUFFER_SIZE, gio::Cancellable::NONE)?;
        Ok(bytes_to_string(&bytes))
    }

    pub async fn stream_read_future(stream: &impl InputStreamExt) -> Result<String, glib::Error> {
        let bytes = stream
            .read_bytes_future(SOCKET_BUFFER_SIZE, glib::Priority::DEFAULT)
            .await?;
        Ok(bytes_to_string(&bytes))
    }

    // TODO
    // - [ ] Need to accept parameter as Match
    // - [ ] So Match have to be wrapped into something that will represent message or etc (like for handling `quit` message)
    pub fn stream_write(
        stream: &impl OutputStreamExt,
        value: impl AsRef<[u8]>,
    ) -> Result<isize, glib::Error> {
        stream.write_bytes(
            &glib::Bytes::from(&create_buffer(value.as_ref())),
            gio::Cancellable::NONE,
        )
    }

    // TODO
    // - [ ] Need to accept parameter as Match
    // - [ ] So Match have to be wrapped into something that will represent message or etc (like for handling `quit` message)
    pub async fn stream_write_future(
        stream: &impl OutputStreamExt,
        value: impl AsRef<[u8]>,
    ) -> Result<isize, glib::Error> {
        stream
            .write_bytes_future(
                &glib::Bytes::from(&create_buffer(value.as_ref())),
                glib::Priority::DEFAULT,
            )
            .await
    }

    pub fn connect_and_write(value: impl AsRef<[u8]> + std::fmt::Debug) -> Result<(), glib::Error> {
        use gio::prelude::IOStreamExt;

        let conn = connection()?;
        stream_write(&conn.output_stream(), value.as_ref())?;
        Ok(())
    }

    pub async fn connect_and_write_future(
        value: impl AsRef<[u8]> + std::fmt::Debug,
    ) -> Result<(), glib::Error> {
        use gio::prelude::IOStreamExt;

        let conn = connection_future().await?;
        stream_write_future(&conn.output_stream(), value.as_ref()).await?;
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

pub mod match_if {
    use serde::{Deserialize, Serialize};
    use std::fmt::Display;
    use uuid::Uuid;

    #[derive(Default, Debug, Serialize, Deserialize, Clone)]
    pub struct Match {
        // TODO remove public to prevent building without `Self::new()`
        pub id: String,
        pub title: String,
        pub description: Option<String>,
        pub icon: Option<String>,
        pub use_pango: bool,
    }

    impl Match {
        /// Creates a new `Match` instance.
        ///
        /// # Parameters
        ///
        /// - `title`: The title of the match.
        /// - `description`: An optional description of the match.
        /// - `icon`: An optional icon associated with the match.
        /// - `use_pango`: A flag indicating whether Pango is used.
        ///
        /// # Returns
        ///
        /// A new `Match` instance.
        pub fn new(
            title: &str,
            description: Option<&str>,
            icon: Option<&str>,
            use_pango: bool,
        ) -> Self {
            Self {
                id: Self::new_id(),
                title: title.to_owned(),
                description: description.map(str::to_owned),
                icon: icon.map(str::to_owned),
                use_pango,
            }
        }

        pub fn get_id(&self) -> &str {
            &self.id
        }

        /// Generates a new ID for the match and updates the existing one.
        ///
        /// # Returns
        ///
        /// The new UUID of the match.
        pub fn update_id(&mut self) -> String {
            let new_id = Self::new_id();
            let _ = std::mem::replace(&mut self.id, new_id.clone());
            new_id
        }

        fn new_id() -> String {
            Uuid::new_v4().to_string()
        }
    }

    impl Display for Match {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "({}, {})", self.id, self.title)
        }
    }
}
