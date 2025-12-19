pub mod data_feed;
pub mod slurper;

use std::str::FromStr;
use thiserror::Error;

/// User-Agent string used for all HTTP requests.
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unknown facility type: {0}")]
    UnknownFacilityType(String),
    #[error(transparent)]
    Slurper(#[from] crate::slurper::SlurperError),
    #[error(transparent)]
    DataFeed(#[from] crate::data_feed::DataFeedError),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ControllerInfo {
    pub cid: String,
    pub callsign: String,
    pub frequency: String,
    pub facility_type: FacilityType,
}

/// Enum representing the different VATSIM facility types as parsed from their respective callsign suffixes
/// (in accordance with the [VATSIM GCAP](https://vatsim.net/docs/policy/global-controller-administration-policy).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum FacilityType {
    #[default]
    Unknown,
    Ramp,
    Delivery,
    Ground,
    Tower,
    Approach,
    Departure,
    Enroute,
    FlightServiceStation,
    Radio,
    TrafficFlow,
}

impl FromStr for FacilityType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let s = s.to_ascii_uppercase();
        let facility_suffix = s.split('_').next_back().unwrap_or_default();
        match facility_suffix {
            "RMP" => Ok(FacilityType::Ramp),
            "DEL" => Ok(FacilityType::Delivery),
            "GND" => Ok(FacilityType::Ground),
            "TWR" => Ok(FacilityType::Tower),
            "APP" => Ok(FacilityType::Approach),
            "DEP" => Ok(FacilityType::Departure),
            "CTR" => Ok(FacilityType::Enroute),
            "FSS" => Ok(FacilityType::FlightServiceStation),
            "RDO" => Ok(FacilityType::Radio),
            "TMU" | "FMP" => Ok(FacilityType::TrafficFlow),
            other => Err(Error::UnknownFacilityType(other.to_string())),
        }
    }
}

impl From<&str> for FacilityType {
    fn from(value: &str) -> Self {
        value.parse().unwrap_or_default()
    }
}

impl From<String> for FacilityType {
    fn from(value: String) -> Self {
        value.as_str().parse().unwrap_or_default()
    }
}
