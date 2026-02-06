use serde::{Deserialize, Serialize};

/// Unique identifier for a VATSIM client (CID).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ClientId(String);

/// Unique identifier for a VATSIM position.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PositionId(String);

/// Unique identifier for a vacs profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ProfileId(String);

/// Unique identifier for a VATSIM station.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct StationId(String);

/// Representation of a VACS profile.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// The unique identifier for this profile.
    pub id: ProfileId,
    /// The type of profile and its associated configuration.
    #[serde(flatten)]
    pub profile_type: ProfileType,
}

/// The specific configuration type of a profile.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProfileType {
    /// A GEO profile with a container-based layout.
    Geo(GeoPageContainer),
    /// A tabbed profile with pages accessible via tabs.
    ///
    /// The list of tabs will always be non-empty.
    Tabbed(Vec<Tab>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    /// The label of the tab.
    pub label: String,
    /// The direct access page that opens when this tab is clicked.
    pub page: DirectAccessPage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoPageContainer {
    /// The height of the container.
    ///
    /// Must either be defined as a percentage or a rem value (e.g. "100%", "5rem").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<String>,
    /// The width of the container.
    ///
    /// Must either be defined as a percentage or a rem value (e.g. "100%", "5rem").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
    /// The padding for all sides (in rem).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<f64>,
    /// The left padding (in rem).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<f64>,
    /// The right padding (in rem).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<f64>,
    /// The top padding (in rem).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<f64>,
    /// The bottom padding (in rem).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<f64>,
    /// The gap between children (in rem).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap: Option<f64>,
    /// The justification of the content along the main axis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justify_content: Option<JustifyContent>,
    /// The alignment of items along the cross-axis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align_items: Option<AlignItems>,
    /// The direction of the flex container.
    pub direction: FlexDirection,
    /// The children of this container.
    pub children: Vec<GeoNode>,
}

// TODO split file and place geo page container specific enums in submodule
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JustifyContent {
    /// The items are packed flush to each other toward the start edge of the alignment container in the main axis.
    Start,
    /// The items are packed flush to each other toward the end edge of the alignment container in the main axis.
    End,
    /// The items are evenly distributed within the alignment container along the main axis. The spacing between each pair of adjacent items is the same. The first item is flush with the main-start edge, and the last item is flush with the main-end edge.
    SpaceBetween,
    /// The items are evenly distributed within the alignment container along the main axis. The spacing between each pair of adjacent items is the same. The empty space before the first and after the last item equals half of the space between each pair of adjacent items. If there is only one item, it will be centered.
    SpaceAround,
    /// The items are evenly distributed within the alignment container along the main axis. The spacing between each pair of adjacent items, the main-start edge and the first item, and the main-end edge and the last item, are all exactly the same.
    SpaceEvenly,
    /// The items are packed flush to each other toward the center of the alignment container along the main axis.
    Center,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AlignItems {
    /// The items are packed flush to each other toward the start edge of the alignment container in the appropriate axis.
    Start,
    /// The items are packed flush to each other toward the end edge of the alignment container in the appropriate axis.
    End,
    /// The flex items' margin boxes are centered within the line on the cross-axis. If the cross-size of an item is larger than the flex container, it will overflow equally in both directions.
    Center,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlexDirection {
    /// The flex container's main axis is the same as the text direction.
    Row,
    /// The flex container's main axis is the same as the block axis.
    Col,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GeoNode {
    /// A recursive container for grouping other nodes.
    Container(GeoPageContainer),
    /// A clickable button with a label and action.
    Button(GeoPageButton),
    /// A visual divider between elements.
    Divider(GeoPageDivider),
}

/// A button on a GEO profile page.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoPageButton {
    /// The text label displayed on the button.
    ///
    /// Will always contain between 0 and 3 lines of text.
    pub label: Vec<String>,
    /// The size of the button (> 0, in rem).
    pub size: f64,
    /// The direct access page that opens when this button is clicked.
    /// If `None`, the button will be displayed and clickable on the UI, but will otherwise be non-functional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<DirectAccessPage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoPageDivider {
    /// The orientation of the divider.
    pub orientation: GeoPageDividerOrientation,
    /// The thickness of the divider (> 0, in px).
    pub thickness: f64,
    /// The color of the divider (CSS color).
    pub color: String,
    /// The oversize of the divider (> 0, in rem).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oversize: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeoPageDividerOrientation {
    /// The divider runs horizontally.
    Horizontal,
    /// The divider runs vertically.
    Vertical,
}

/// A page containing direct access keys.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DirectAccessPage {
    /// The list of keys on this page.
    ///
    /// Will always be non-empty.
    pub keys: Vec<DirectAccessKey>,
    /// The number of rows in the grid (> 0).
    ///
    /// The default layout is optimized for 6 rows. After a seventh row is added,
    /// the space in between the rows is slightly reduced and a scrollbar might
    /// appear automatically.
    pub rows: u8,
}

/// A single key on a direct access page.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectAccessKey {
    /// The text label displayed on the key.
    ///
    /// Will always contain between 0 and 3 lines of text.
    pub label: Vec<String>,
    /// The optional station ID associated with this key.
    ///
    /// If [`DirectAccessKey::station_id`] and [`DirectAccessKey::page`] are `None`, the DA key will be displayed on the UI, but will be non-functional.
    /// This field is mutually exclusive with [`DirectAccessKey::page`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub station_id: Option<StationId>,
    /// The optional subpage associated with this key.
    ///
    /// If [`DirectAccessKey::page`] and [`DirectAccessKey::station_id`] are `None`, the DA key will be displayed on the UI, but will be non-functional.
    /// This field is mutually exclusive with [`DirectAccessKey::station_id`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<DirectAccessPage>,
}

/// Trait alias for types that can be used as a profile reference in [`ActiveProfile`].
///
/// This trait is sealed, ensuring only the appropriate types can be passed.
pub trait ProfileReference: crate::sealed::Sealed {}

impl crate::sealed::Sealed for ProfileId {}
impl ProfileReference for ProfileId {}

impl crate::sealed::Sealed for Profile {}
impl ProfileReference for Profile {}

/// Represents the currently active profile for a user session.
///
/// The active profile determines which stations are considered "relevant" and thus which
/// status updates (online/offline/handoff) are sent to the client.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "profile")]
pub enum ActiveProfile<T: ProfileReference> {
    /// A specific, pre-defined profile is active.
    ///
    /// The client is restricted to the view defined by this profile, meaning only
    /// relevant stations and buttons configured in this profile are displayed and the
    /// appropriate station updates are sent.
    Specific(T),
    /// A custom, client-side profile selection is active.
    ///
    /// This typically corresponds to a "Show All" or "Custom" view where the set of
    /// relevant stations is determined dynamically by the client, or all stations are shown.
    Custom,
    /// No profile is currently active.
    ///
    /// In this state, the client will not receive any station updates, only general
    /// client information updates.
    #[default]
    None,
}

/// Represents a change in station status (online, offline, or handoff).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StationChange {
    /// A station has come online.
    #[serde(rename_all = "camelCase")]
    Online {
        /// The ID of the station that came online.
        station_id: StationId,
        /// The ID of the position that controls the station.
        position_id: PositionId,
    },
    /// A station has been handed off from one position to another.
    #[serde(rename_all = "camelCase")]
    Handoff {
        /// The ID of the station being handed off.
        station_id: StationId,
        /// The ID of the position handing off control over the station.
        from_position_id: PositionId,
        /// The ID of the position receiving control over the station.
        to_position_id: PositionId,
    },
    /// A station has gone offline.
    #[serde(rename_all = "camelCase")]
    Offline {
        /// The ID of the station that went offline.
        station_id: StationId,
    },
}

impl ClientId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for ClientId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for ClientId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl From<i32> for ClientId {
    fn from(id: i32) -> Self {
        Self(id.to_string())
    }
}

impl AsRef<str> for ClientId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for ClientId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<String> for ClientId {
    fn borrow(&self) -> &String {
        &self.0
    }
}

impl PositionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for PositionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for PositionId {
    fn from(id: String) -> Self {
        Self(id.to_ascii_uppercase())
    }
}

impl From<&str> for PositionId {
    fn from(id: &str) -> Self {
        Self(id.to_ascii_uppercase())
    }
}

impl AsRef<str> for PositionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for PositionId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<String> for PositionId {
    fn borrow(&self) -> &String {
        &self.0
    }
}

impl ProfileId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for ProfileId {
    fn from(id: String) -> Self {
        Self(id.to_ascii_uppercase())
    }
}

impl From<&str> for ProfileId {
    fn from(id: &str) -> Self {
        Self(id.to_ascii_uppercase())
    }
}

impl AsRef<str> for ProfileId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for ProfileId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<String> for ProfileId {
    fn borrow(&self) -> &String {
        &self.0
    }
}

impl StationId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for StationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for StationId {
    fn from(id: String) -> Self {
        Self(id.to_ascii_uppercase())
    }
}

impl From<&str> for StationId {
    fn from(id: &str) -> Self {
        Self(id.to_ascii_uppercase())
    }
}

impl AsRef<str> for StationId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for StationId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<String> for StationId {
    fn borrow(&self) -> &String {
        &self.0
    }
}

impl std::fmt::Debug for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Profile")
            .field("id", &self.id)
            .field("profile_type", &self.profile_type)
            .finish()
    }
}

impl PartialOrd for Profile {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl std::fmt::Debug for ProfileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Geo(container) => f.debug_tuple("Geo").field(container).finish(),
            Self::Tabbed(tabs) => f.debug_tuple("Tabbed").field(&tabs.len()).finish(),
        }
    }
}

// impl std::fmt::Debug for GeoPageContainer {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("GeoPageContainer")
//             .field("height", &self.height)
//             .field("width", &self.width)
//             .field("padding", &self.padding)
//             .field("padding_left", &self.padding_left)
//             .field("padding_right", &self.padding_right)
//             .field("padding_top", &self.padding_top)
//             .field("padding_bottom", &self.padding_bottom)
//             .field("gap", &self.gap)
//             .field("justify_content", &self.justify_content)
//             .field("align_items", &self.align_items)
//             .field("direction", &self.direction)
//             .field("children", &self.children.len())
//             .finish()
//     }
// }

impl std::fmt::Debug for GeoPageButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeoPageButton")
            .field("label", &self.label.len())
            .field("size", &self.size)
            .field("page", &self.page)
            .finish()
    }
}

impl std::fmt::Debug for DirectAccessPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectAccessPage")
            .field("keys", &self.keys.len())
            .field("rows", &self.rows)
            .finish()
    }
}

impl std::fmt::Debug for DirectAccessKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectAccessKey")
            .field("label", &self.label.len())
            .field("station_id", &self.station_id)
            .finish()
    }
}

impl<S, P> From<(S, Option<P>, Option<P>)> for StationChange
where
    S: Into<StationId>,
    P: Into<PositionId>,
{
    fn from((station_id, from, to): (S, Option<P>, Option<P>)) -> Self {
        match (from, to) {
            (None, Some(to)) => Self::Online {
                station_id: station_id.into(),
                position_id: to.into(),
            },
            (Some(_), None) => Self::Offline {
                station_id: station_id.into(),
            },
            (Some(from), Some(to)) => Self::Handoff {
                station_id: station_id.into(),
                from_position_id: from.into(),
                to_position_id: to.into(),
            },
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn station_id_creation() {
        let id = StationId::from("loww_twr");
        assert_eq!(id.as_str(), "LOWW_TWR");
        assert_eq!(id.to_string(), "LOWW_TWR");
        assert!(!id.is_empty());

        let empty = StationId::from("");
        assert!(empty.is_empty());
    }

    #[test]
    fn station_id_equality() {
        let id1 = StationId::from("LOWW_TWR");
        let id2 = StationId::from("loww_twr");
        assert_eq!(id1, id2);
    }

    #[test]
    fn position_id_creation() {
        let id = PositionId::from("loww_twr");
        assert_eq!(id.as_str(), "LOWW_TWR");
        assert_eq!(id.to_string(), "LOWW_TWR");
        assert!(!id.is_empty());

        let empty = PositionId::from("");
        assert!(empty.is_empty());
    }

    #[test]
    fn position_id_equality() {
        let id1 = PositionId::from("LOWW_TWR");
        let id2 = PositionId::from("loww_twr");
        assert_eq!(id1, id2);
    }
}
