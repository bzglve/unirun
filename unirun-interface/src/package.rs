use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[derive(Debug)]
pub enum Command {
    Quit,
    Activate(String),
    GetData(String),
    Abort,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Package {
    Command(Command),
    Result(Result<(), ()>),
    Match(match_if::Match),
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
