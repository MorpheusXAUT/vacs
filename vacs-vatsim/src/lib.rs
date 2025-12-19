#[cfg(feature = "coverage")]
pub mod coverage;
#[cfg(feature = "data-feed")]
pub mod data_feed;
#[cfg(feature = "slurper")]
pub mod slurper;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;
use thiserror::Error;

#[cfg(any(feature = "data-feed", feature = "slurper"))]
/// User-Agent string used for all HTTP requests.
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unknown facility type: {0}")]
    UnknownFacilityType(String),
    #[error(transparent)]
    #[cfg(feature = "coverage")]
    Coverage(#[from] crate::coverage::CoverageError),
    #[error(transparent)]
    #[cfg(feature = "slurper")]
    Slurper(#[from] crate::slurper::SlurperError),
    #[error(transparent)]
    #[cfg(feature = "data-feed")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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

impl FacilityType {
    pub const fn as_str(&self) -> &str {
        match self {
            FacilityType::Ramp => "RMP",
            FacilityType::Delivery => "DEL",
            FacilityType::Ground => "GND",
            FacilityType::Tower => "TWR",
            FacilityType::Approach => "APP",
            FacilityType::Departure => "DEP",
            FacilityType::Enroute => "CTR",
            FacilityType::FlightServiceStation => "FSS",
            FacilityType::Radio => "RDO",
            FacilityType::TrafficFlow => "FMP",
            FacilityType::Unknown => "UNKNOWN",
        }
    }

    pub fn from_vatsim_facility(facility: u8) -> Self {
        FacilityType::try_from(facility).unwrap_or_default()
    }
}

impl FromStr for FacilityType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let s = s.to_ascii_uppercase();
        let facility_suffix = s.split('_').next_back().unwrap_or_default();
        match facility_suffix {
            "RMP" | "RAMP" => Ok(FacilityType::Ramp),
            "DEL" | "DELIVERY" => Ok(FacilityType::Delivery),
            "GND" | "GROUND" => Ok(FacilityType::Ground),
            "TWR" | "TOWER" => Ok(FacilityType::Tower),
            "APP" | "APPROACH" => Ok(FacilityType::Approach),
            "DEP" | "DEPARTURE" => Ok(FacilityType::Departure),
            "CTR" | "CENTER" | "ENROUTE" => Ok(FacilityType::Enroute),
            "FSS" | "FLIGHTSERVICESTATION" => Ok(FacilityType::FlightServiceStation),
            "RDO" | "RADIO" => Ok(FacilityType::Radio),
            "TMU" | "TRAFFICMANAGEMENTUNIT" | "FMP" | "FLOWMANAGEMENTPOSITION" | "TRAFFICFLOW" => {
                Ok(FacilityType::TrafficFlow)
            }
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

impl TryFrom<u8> for FacilityType {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(FacilityType::FlightServiceStation),
            2 => Ok(FacilityType::Delivery),
            3 => Ok(FacilityType::Ground),
            4 => Ok(FacilityType::Tower),
            5 => Ok(FacilityType::Approach),
            6 => Ok(FacilityType::Enroute),
            other => Err(Error::UnknownFacilityType(other.to_string())),
        }
    }
}

impl Serialize for FacilityType {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for FacilityType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FacilityType::from_str(&s).map_err(serde::de::Error::custom)
    }
}
