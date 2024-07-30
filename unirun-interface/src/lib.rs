pub mod constants {
    pub const DOMAIN: &str = "com.bzglve";
    pub const MAIN_APP_ID: &str = "com.bzglve.unirun";
    // TODO need to coordinate socket buffer size between processes instead of using const default
    // 1024 is 1KiB
    pub const SOCKET_BUFFER_SIZE: usize = 1024 * 4;
}

pub mod path {
    use std::path::PathBuf;

    use crate::constants::{DOMAIN, MAIN_APP_ID};

    pub fn runtime() -> PathBuf {
        use std::fs;

        let path = glib::user_runtime_dir().join(DOMAIN);
        if !path.exists() {
            fs::create_dir_all(&path).unwrap_or_else(|_| {
                panic!(
                    "Failed to create runtime directory at {}",
                    path.to_string_lossy()
                )
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

    pub fn stream_read(stream: &impl InputStreamExt) -> Result<String, glib::Error> {
        Ok(bytes_to_string(
            &stream.read_bytes(SOCKET_BUFFER_SIZE, gio::Cancellable::NONE)?,
        ))
    }

    pub fn stream_read_async<F>(stream: &impl InputStreamExt, callback: F)
    where
        F: Fn(Result<&str, glib::Error>) + 'static,
    {
        stream.read_bytes_async(
            SOCKET_BUFFER_SIZE,
            glib::Priority::DEFAULT,
            gio::Cancellable::NONE,
            move |data| match data {
                Ok(bytes) => {
                    let data = bytes_to_string(&bytes);

                    callback(Ok(data.as_str()));
                }
                Err(e) => callback(Err(e)),
            },
        )
    }

    pub async fn stream_read_future(stream: &impl InputStreamExt) -> Result<String, glib::Error> {
        Ok(bytes_to_string(
            &stream
                .read_bytes_future(SOCKET_BUFFER_SIZE, glib::Priority::DEFAULT)
                .await?,
        ))
    }

    fn create_buffer(value: impl AsRef<[u8]>) -> [u8; SOCKET_BUFFER_SIZE] {
        let value_ref = value.as_ref();
        let mut buffer = [0; SOCKET_BUFFER_SIZE];
        let len = value_ref.len().min(SOCKET_BUFFER_SIZE);
        buffer[..len].copy_from_slice(&value_ref[..len]);
        buffer
    }

    /// # TODO
    /// - [ ] Need to accept parameter as Match
    /// - [ ] So Match have to be wrapped into something that will represent message or etc (like for handling `quit` message)
    pub fn stream_write(
        stream: &impl OutputStreamExt,
        value: impl AsRef<[u8]>,
    ) -> Result<isize, glib::Error> {
        let res = stream.write_bytes(
            &glib::Bytes::from(&create_buffer(value)),
            gio::Cancellable::NONE,
        )?;
        stream.flush(gio::Cancellable::NONE)?;
        Ok(res)
    }

    /// # TODO
    /// - [ ] Need to accept parameter as Match
    /// - [ ] So Match have to be wrapped into something that will represent message or etc (like for handling `quit` message)
    pub async fn stream_write_future(
        stream: &impl OutputStreamExt,
        value: impl AsRef<[u8]>,
    ) -> Result<isize, glib::Error> {
        let res = stream
            .write_bytes_future(
                &glib::Bytes::from(&create_buffer(value)),
                glib::Priority::DEFAULT,
            )
            .await?;
        stream.flush_future(glib::Priority::DEFAULT).await?;
        Ok(res)
    }

    pub fn connect_and_write(value: impl AsRef<[u8]> + std::fmt::Debug) -> Result<(), glib::Error> {
        use gio::prelude::IOStreamExt;

        let conn = connection()?;
        // debug!("Writing to output stream: {:?}", value);

        match stream_write(&conn.output_stream(), value.as_ref()) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub async fn connect_and_write_future(
        value: impl AsRef<[u8]> + std::fmt::Debug,
    ) -> Result<(), glib::Error> {
        use gio::prelude::IOStreamExt;

        let conn = connection_future().await?;
        // debug!("Writing to output stream: {:?}", value);

        match stream_write_future(&conn.output_stream(), value.as_ref()).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn connection() -> Result<SocketConnection, glib::Error> {
        use crate::path;
        use gio::prelude::SocketClientExt;

        let socket_path = path::socket();
        let conn = SocketClientExt::connect(
            &gio::SocketClient::new(),
            &gio::UnixSocketAddress::new(&socket_path),
            gio::Cancellable::NONE,
        )?;

        // debug!("Connected to socket: {}", socket_path.to_string_lossy());

        Ok(conn)
    }

    pub async fn connection_future() -> Result<SocketConnection, glib::Error> {
        use crate::path;
        use gio::prelude::SocketClientExt;

        let socket_path = path::socket();
        let conn = SocketClientExt::connect_future(
            &gio::SocketClient::new(),
            &gio::UnixSocketAddress::new(&socket_path),
        )
        .await?;

        // debug!("Connected to socket: {}", socket_path.to_string_lossy());

        Ok(conn)
    }
}

pub fn bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_matches(char::from(0))
        .trim()
        .to_string()
}

/// This module defines the `Match` struct and its associated methods and traits.
pub mod match_if {
    use serde::{Deserialize, Serialize};
    use std::fmt::Display;
    use uuid::Uuid;

    /// Represents a match with an ID, title, optional description, optional icon, and a flag indicating if Pango is used.
    #[derive(Default, Debug, Serialize, Deserialize, Clone)]
    pub struct Match {
        // TODO remove public to prevent building without `Self::new()`
        pub id: String,
        /// The title of the match.
        pub title: String,
        /// An optional description of the match.
        pub description: Option<String>,
        /// An optional icon associated with the match.
        pub icon: Option<String>,
        /// A flag indicating whether Pango is used.
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
                description: description.map(|v| v.to_owned()),
                icon: icon.map(|v| v.to_owned()),
                use_pango,
            }
        }

        /// Gets the ID of the match.
        ///
        /// # Returns
        ///
        /// The UUID of the match.
        pub fn get_id(&self) -> String {
            self.id.clone()
        }

        /// Generates a new ID for the match and updates the existing one.
        ///
        /// # Returns
        ///
        /// The new UUID of the match.
        pub fn update_id(&mut self) -> String {
            let val = Self::new_id();
            self.id.clone_from(&val);
            val
        }

        /// Generates a new UUID.
        ///
        /// # Returns
        ///
        /// A new UUID.
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
