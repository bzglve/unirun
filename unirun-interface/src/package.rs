use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
pub struct Uuid(String);

impl Uuid {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Display for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// FIXME why?
impl From<&str> for Uuid {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

pub type PackageId = Uuid;
pub type MatchId = Uuid;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum Command {
    #[serde(rename = "quit")]
    Quit,

    #[serde(rename = "activate")]
    Activate(MatchId),

    #[serde(rename = "get_data")]
    GetData(String),

    #[serde(rename = "abort")]
    Abort,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum Payload {
    #[serde(rename = "command")]
    Command(Command),

    #[serde(rename = "result")]
    Result(Result<PackageId, PackageId>),

    #[serde(rename = "match")]
    Match(match_if::Match),
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Package {
    id: PackageId,

    #[serde(flatten)]
    pub payload: Payload,
}

impl Package {
    pub fn new(payload: Payload) -> Self {
        Self {
            id: PackageId::new(),
            payload,
        }
    }

    pub fn get_id(&self) -> PackageId {
        self.id.clone()
    }
}

pub mod match_if {
    use serde::{Deserialize, Serialize};
    use std::fmt::Display;

    use super::MatchId;

    #[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
    pub struct Match {
        // TODO remove public to prevent building without `Self::new()`
        pub id: MatchId,
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
                id: MatchId::new(),
                title: title.to_owned(),
                description: description.map(str::to_owned),
                icon: icon.map(str::to_owned),
                use_pango,
            }
        }

        pub fn get_id(&self) -> MatchId {
            self.id.clone()
        }

        /// Generates a new ID for the match and updates the existing one.
        ///
        /// # Returns
        ///
        /// The new UUID of the match.
        pub fn update_id(&mut self) -> MatchId {
            let new_id = MatchId::new();
            self.id = new_id.clone();
            new_id
        }
    }

    impl Display for Match {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "({}, {})", self.id, self.title)
        }
    }
}
