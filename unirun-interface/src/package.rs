pub use hit::{Hit, HitId};
pub use package::{Command, Package, PackageId, Payload};
use serde::{Deserialize, Serialize};
pub use uuid::Uuid;

mod uuid {
    use super::*;

    #[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
    pub struct Uuid(String);

    impl Uuid {
        pub fn new() -> Self {
            Self(uuid_crate::Uuid::new_v4().to_string())
        }
    }

    impl Default for Uuid {
        fn default() -> Self {
            Self::new()
        }
    }

    impl std::fmt::Display for Uuid {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl From<&str> for Uuid {
        fn from(value: &str) -> Self {
            Self(value.to_owned())
        }
    }
}

#[allow(clippy::module_inception)]
mod package {
    use hit::{Hit, HitId};

    use super::*;

    #[doc(alias = "Uuid")]
    pub type PackageId = Uuid;

    #[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
    pub enum Command {
        #[serde(rename = "quit")]
        Quit,

        #[serde(rename = "activate")]
        Activate(HitId),

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
        Result((PackageId, Result<(), String>)),

        #[serde(rename = "hit")]
        Hit(Hit),
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
}

mod hit {
    use uuid::Uuid;

    use super::*;

    #[doc(alias = "Uuid")]
    pub type HitId = Uuid;

    #[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
    pub struct Hit {
        pub id: HitId,
        pub title: String,
        pub description: Option<String>,
        pub icon: Option<String>,
        pub use_pango: bool,
    }

    impl Hit {
        pub fn new(
            title: &str,
            description: Option<&str>,
            icon: Option<&str>,
            use_pango: bool,
        ) -> Self {
            Self {
                id: HitId::new(),
                title: title.to_owned(),
                description: description.map(str::to_owned),
                icon: icon.map(str::to_owned),
                use_pango,
            }
        }
    }

    impl std::fmt::Display for Hit {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "({}, {})", self.id, self.title)
        }
    }
}
