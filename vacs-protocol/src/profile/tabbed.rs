use crate::profile::DirectAccessPage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    /// The label of the tab.
    pub label: String,

    /// The direct access page that opens when this tab is clicked.
    pub page: DirectAccessPage,
}
