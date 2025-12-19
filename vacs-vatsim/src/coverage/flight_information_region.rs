use crate::coverage::position::{PositionConfigFile, PositionId, PositionRaw};
use crate::coverage::station::{StationConfigFile, StationId, StationRaw};
use crate::coverage::{CoverageError, IoError, ValidationError, Validator};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct FlightInformationRegionId(String);

#[derive(Debug, Clone)]
pub struct FlightInformationRegion {
    pub id: FlightInformationRegionId,
    pub stations: HashSet<StationId>,
    pub positions: HashSet<PositionId>,
}

#[derive(Debug, Clone)]
pub(super) struct FlightInformationRegionRaw {
    pub id: FlightInformationRegionId,
    pub stations: Vec<StationRaw>,
    pub positions: Vec<PositionRaw>,
}

impl PartialEq for FlightInformationRegion {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Validator for FlightInformationRegionRaw {
    fn validate(&self) -> Result<(), CoverageError> {
        if self.id.is_empty() {
            return Err(ValidationError::MissingField("id".to_string()).into());
        }
        if self.stations.is_empty() {
            return Err(ValidationError::MissingField("stations".to_string()).into());
        }
        if self.positions.is_empty() {
            return Err(ValidationError::MissingField("positions".to_string()).into());
        }
        Ok(())
    }
}

impl FlightInformationRegionRaw {
    pub fn load_from_dir(dir: impl AsRef<std::path::Path>) -> Result<Self, CoverageError> {
        let path = dir.as_ref();
        let Some(dir_name) = path.file_name() else {
            return Err(ValidationError::Custom(format!("missing dir name: {path:?}")).into());
        };
        let Some(dir_name) = dir_name.to_str() else {
            return Err(ValidationError::Custom(format!("invalid dir name: {path:?}")).into());
        };

        Ok(Self {
            id: FlightInformationRegionId::from(dir_name),
            stations: Self::read_file::<StationConfigFile>(path, "stations")?.stations,
            positions: Self::read_file::<PositionConfigFile>(path, "positions")?.positions,
        })
    }

    const FILE_EXTENSIONS: &'static [&'static str] = &["toml", "json"];
    fn read_file<T: for<'de> Deserialize<'de>>(
        dir: &std::path::Path,
        kind: &'static str,
    ) -> Result<T, CoverageError> {
        let (path, ext) = Self::FILE_EXTENSIONS
            .iter()
            .find_map(|ext| {
                let path = dir.join(std::path::Path::new(kind).with_extension(ext));
                if path.is_file() {
                    Some((path.clone(), ext))
                } else {
                    None
                }
            })
            .ok_or_else(|| IoError::Read {
                path: dir.into(),
                reason: format!("No {kind} file found"),
            })?;

        let bytes = std::fs::read(&path).map_err(|err| IoError::Read {
            path: path.clone(),
            reason: err.to_string(),
        })?;

        match *ext {
            "toml" => toml::from_slice(&bytes).map_err(|err| IoError::Parse {
                path: path.clone(),
                reason: err.to_string(),
            }),
            "json" => serde_json::from_slice(&bytes).map_err(|err| IoError::Parse {
                path: path.clone(),
                reason: err.to_string(),
            }),
            _ => Err(IoError::Read {
                path: path.clone(),
                reason: format!("unsupported file extension: {ext}"),
            }),
        }
        .map_err(Into::into)
    }
}

impl TryFrom<FlightInformationRegionRaw> for FlightInformationRegion {
    type Error = CoverageError;
    fn try_from(value: FlightInformationRegionRaw) -> Result<Self, Self::Error> {
        value.validate()?;

        Ok(Self {
            id: value.id,
            stations: value.stations.iter().map(|s| s.id.clone()).collect(),
            positions: value.positions.iter().map(|p| p.id.clone()).collect(),
        })
    }
}

impl FlightInformationRegionId {
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for FlightInformationRegionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for FlightInformationRegionId {
    fn from(value: String) -> Self {
        FlightInformationRegionId(value.to_ascii_uppercase())
    }
}

impl From<&str> for FlightInformationRegionId {
    fn from(value: &str) -> Self {
        FlightInformationRegionId(value.to_ascii_uppercase())
    }
}

impl std::borrow::Borrow<str> for FlightInformationRegionId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_matches};

    #[test]
    fn fir_id_creation() {
        let id = FlightInformationRegionId::from("lovv");
        assert_eq!(id.as_str(), "LOVV");
        assert_eq!(id.to_string(), "LOVV");
        assert!(!id.is_empty());

        let empty = FlightInformationRegionId::from("");
        assert!(empty.is_empty());
    }

    #[test]
    fn fir_id_equality() {
        let id1 = FlightInformationRegionId::from("LOVV");
        let id2 = FlightInformationRegionId::from("lovv");
        assert_eq!(id1, id2);
    }

    #[test]
    fn fir_raw_valid() {
        let raw = FlightInformationRegionRaw {
            id: "LOVV".into(),
            stations: vec![StationRaw {
                id: "LOWW_TWR".into(),
                parent_id: None,
                controlled_by: vec![],
            }],
            positions: vec![PositionRaw {
                id: "LOWW_TWR".into(),
                prefixes: HashSet::from(["LOWW".to_string()]),
                frequency: "119.400".to_string(),
                facility_type: crate::FacilityType::Tower,
            }],
        };
        assert!(raw.validate().is_ok());
    }

    #[test]
    fn fir_raw_invalid_id() {
        let raw = FlightInformationRegionRaw {
            id: "".into(),
            stations: vec![StationRaw {
                id: "LOWW_TWR".into(),
                parent_id: None,
                controlled_by: vec![],
            }],
            positions: vec![PositionRaw {
                id: "LOWW_TWR".into(),
                prefixes: HashSet::from(["LOWW".to_string()]),
                frequency: "119.400".to_string(),
                facility_type: crate::FacilityType::Tower,
            }],
        };
        assert_matches!(
            raw.validate(),
            Err(CoverageError::Validation(ValidationError::MissingField(f))) if f == "id"
        );
    }

    #[test]
    fn fir_raw_invalid_stations() {
        let raw = FlightInformationRegionRaw {
            id: "LOVV".into(),
            stations: vec![],
            positions: vec![PositionRaw {
                id: "LOWW_TWR".into(),
                prefixes: HashSet::from(["LOWW".to_string()]),
                frequency: "119.400".to_string(),
                facility_type: crate::FacilityType::Tower,
            }],
        };
        assert_matches!(
            raw.validate(),
            Err(CoverageError::Validation(ValidationError::MissingField(f))) if f == "stations"
        );
    }

    #[test]
    fn fir_raw_invalid_positions() {
        let raw = FlightInformationRegionRaw {
            id: "LOVV".into(),
            stations: vec![StationRaw {
                id: "LOWW_TWR".into(),
                parent_id: None,
                controlled_by: vec![],
            }],
            positions: vec![],
        };
        assert_matches!(
            raw.validate(),
            Err(CoverageError::Validation(ValidationError::MissingField(f))) if f == "positions"
        );
    }

    #[test]
    fn fir_conversion() {
        let raw = FlightInformationRegionRaw {
            id: "LOVV".into(),
            stations: vec![StationRaw {
                id: "LOWW_TWR".into(),
                parent_id: None,
                controlled_by: vec![],
            }],
            positions: vec![PositionRaw {
                id: "LOWW_TWR".into(),
                prefixes: HashSet::from(["LOWW".to_string()]),
                frequency: "119.400".to_string(),
                facility_type: crate::FacilityType::Tower,
            }],
        };
        let fir = FlightInformationRegion::try_from(raw).unwrap();
        assert_eq!(fir.id.as_str(), "LOVV");
        assert!(fir.stations.contains(&StationId::from("LOWW_TWR")));
        assert!(fir.positions.contains(&PositionId::from("LOWW_TWR")));
    }

    #[test]
    fn fir_equality() {
        let f1 = FlightInformationRegion {
            id: "LOVV".into(),
            stations: HashSet::new(),
            positions: HashSet::new(),
        };
        let f2 = FlightInformationRegion {
            id: "LOVV".into(),
            stations: HashSet::from(["LOWW_TWR".into()]),
            positions: HashSet::from(["LOWW_TWR".into()]),
        };
        assert_eq!(f1, f2); // Should be equal because only IDs check
    }
}
