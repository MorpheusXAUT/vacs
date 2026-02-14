use crate::profile::DirectAccessPage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    /// The label of the tab.
    ///
    /// Will always contain between 1 and 3 lines of text.
    #[serde(deserialize_with = "crate::profile::string_or_vec")]
    pub label: Vec<String>,

    /// The direct access page that opens when this tab is clicked.
    pub page: DirectAccessPage,
}
