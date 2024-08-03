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
        pub id: MatchId,
        pub title: String,
        pub description: Option<String>,
        pub icon: Option<String>,
        pub use_pango: bool,
    }

    impl Match {
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
    }

    impl Display for Match {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "({}, {})", self.id, self.title)
        }
    }
}
