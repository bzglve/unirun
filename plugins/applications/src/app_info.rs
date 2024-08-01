use std::{fmt::Display, path::PathBuf};

use gio::prelude::IsA;
use serde::Serialize;
use unirun_if::match_if::Match;

#[derive(Debug, Serialize, Clone)]
pub struct AppInfo {
    commandline: Option<PathBuf>,
    description: Option<String>,
    display_name: String,
    executable: PathBuf,
    icon: Option<String>,
    pub id: Option<String>,
    name: String,
    supported_types: Vec<String>,
}

impl AppInfo {
    pub fn all() -> Vec<Self> {
        gio::AppInfo::all().into_iter().map(Self::from).collect()
    }

    pub fn search(search_string: &str) -> Vec<Self> {
        gio::DesktopAppInfo::search(search_string)
            .iter()
            .flatten()
            .filter_map(|app_id| gio::DesktopAppInfo::new(app_id))
            .map(Self::from)
            .collect()
    }
}

impl Display for AppInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}, {})",
            self.display_name,
            self.executable.to_string_lossy(),
            self.name,
        )
    }
}

impl<T> From<T> for AppInfo
where
    T: IsA<gio::AppInfo>,
{
    fn from(value: T) -> Self {
        use gio::prelude::{AppInfoExt, IconExt};

        Self {
            commandline: value.commandline(),
            description: value.description().map(|s| s.to_string()),
            display_name: value.display_name().to_string(),
            executable: value.executable(),
            icon: match value.icon() {
                Some(icon) => icon.to_string().map(|i| i.to_string()),
                None => None,
            },
            id: value.id().map(|s| s.to_string()),
            name: value.name().to_string(),
            supported_types: value
                .supported_types()
                .iter()
                .map(|t| t.to_string())
                .collect(),
        }
    }
}

impl From<AppInfo> for Match {
    fn from(val: AppInfo) -> Self {
        Self::new(
            &val.display_name,
            val.description.as_deref(),
            val.icon.as_deref(),
            false,
        )
    }
}

impl From<&AppInfo> for Match {
    fn from(val: &AppInfo) -> Self {
        Self::new(
            &val.display_name,
            val.description.as_deref(),
            val.icon.as_deref(),
            false,
        )
    }
}
