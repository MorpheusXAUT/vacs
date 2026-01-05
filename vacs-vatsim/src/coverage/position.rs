use crate::FacilityType;
use crate::coverage::flight_information_region::FlightInformationRegionId;
use crate::coverage::{CoverageError, ValidationError, Validator};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::LazyLock;
use vacs_protocol::vatsim::PositionId;

static FREQUENCY_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{3}\.\d{3}$").unwrap());

#[derive(Clone)]
pub struct Position {
    pub id: PositionId,
    pub prefixes: HashSet<String>,
    pub frequency: String,
    pub facility_type: FacilityType,
    pub fir_id: FlightInformationRegionId,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct PositionRaw {
    pub id: PositionId,
    pub prefixes: HashSet<String>,
    pub frequency: String,
    pub facility_type: FacilityType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PositionConfigFile {
    pub positions: Vec<PositionRaw>,
}

impl std::fmt::Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Position")
            .field("id", &self.id)
            .field("prefixes", &self.prefixes.len())
            .field("frequency", &self.frequency)
            .field("facility_type", &self.facility_type)
            .field("fir_id", &self.fir_id)
            .finish()
    }
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Position {
    pub(super) fn from_raw(
        position_raw: PositionRaw,
        fir_id: impl Into<FlightInformationRegionId>,
    ) -> Result<Self, CoverageError> {
        position_raw.validate()?;

        Ok(Self {
            id: position_raw.id,
            prefixes: position_raw.prefixes,
            frequency: position_raw.frequency,
            facility_type: position_raw.facility_type,
            fir_id: fir_id.into(),
        })
    }
}

impl std::fmt::Debug for PositionRaw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PositionRaw")
            .field("id", &self.id)
            .field("prefixes", &self.prefixes.len())
            .field("frequency", &self.frequency)
            .field("facility_type", &self.facility_type)
            .finish()
    }
}

impl Validator for PositionRaw {
    fn validate(&self) -> Result<(), CoverageError> {
        if self.id.is_empty() {
            return Err(ValidationError::Empty {
                field: "id".to_string(),
            }
            .into());
        }
        if self.prefixes.is_empty() || self.prefixes.iter().any(|p| p.is_empty()) {
            return Err(ValidationError::Empty {
                field: "prefixes".to_string(),
            }
            .into());
        }
        if self.frequency.is_empty() {
            return Err(ValidationError::Empty {
                field: "frequency".to_string(),
            }
            .into());
        } else if !FREQUENCY_REGEX.is_match(&self.frequency) {
            return Err(ValidationError::InvalidFormat {
                field: "frequency".to_string(),
                value: self.frequency.clone(),
                reason: "must match pattern XXX.XXX".to_string(),
            }
            .into());
        }
        if self.facility_type == FacilityType::Unknown {
            return Err(ValidationError::InvalidValue {
                field: "facility_type".to_string(),
                value: "Unknown".to_string(),
                reason: "must not be Unknown".to_string(),
            }
            .into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_matches, assert_ne};

    #[test]
    fn position_raw_valid() {
        let raw = PositionRaw {
            id: "LOWW_TWR".into(),
            prefixes: HashSet::from(["LOWW".to_string()]),
            frequency: "119.400".to_string(),
            facility_type: FacilityType::Tower,
        };
        assert!(raw.validate().is_ok());
    }

    #[test]
    fn position_raw_invalid_id() {
        let raw = PositionRaw {
            id: "".into(),
            prefixes: HashSet::from(["LOWW".to_string()]),
            frequency: "119.400".to_string(),
            facility_type: FacilityType::Tower,
        };
        assert_matches!(
            raw.validate(),
            Err(CoverageError::Validation(ValidationError::Empty { field })) if field == "id"
        );
    }

    #[test]
    fn position_raw_invalid_prefixes() {
        // Empty hashset
        let raw = PositionRaw {
            id: "LOWW_TWR".into(),
            prefixes: HashSet::new(),
            frequency: "119.400".to_string(),
            facility_type: FacilityType::Tower,
        };
        assert_matches!(
            raw.validate(),
            Err(CoverageError::Validation(ValidationError::Empty { field })) if field == "prefixes"
        );

        // Empty string in hashset
        let raw = PositionRaw {
            id: "LOWW_TWR".into(),
            prefixes: HashSet::from(["".to_string()]),
            frequency: "119.400".to_string(),
            facility_type: FacilityType::Tower,
        };
        assert_matches!(
            raw.validate(),
            Err(CoverageError::Validation(ValidationError::Empty { field })) if field == "prefixes"
        );
    }

    #[test]
    fn position_raw_invalid_frequency() {
        // Empty
        let raw = PositionRaw {
            id: "LOWW_TWR".into(),
            prefixes: HashSet::from(["LOWW".to_string()]),
            frequency: "".to_string(),
            facility_type: FacilityType::Tower,
        };
        assert_matches!(
            raw.validate(),
            Err(CoverageError::Validation(ValidationError::Empty { field })) if field == "frequency"
        );

        // Bad format
        let bad_freqs = vec![
            "119.4", "119.40", "119.4000", "119,400", "abc.def", "119.40a",
        ];
        for freq in bad_freqs {
            let raw = PositionRaw {
                id: "LOWW_TWR".into(),
                prefixes: HashSet::from(["LOWW".to_string()]),
                frequency: freq.to_string(),
                facility_type: FacilityType::Tower,
            };
            assert_matches!(
                raw.validate(),
                Err(CoverageError::Validation(ValidationError::InvalidFormat { field, .. })) if field == "frequency",
                "Should fail for frequency: {freq}"
            );
        }
    }

    #[test]
    fn position_raw_invalid_facility_type() {
        let raw = PositionRaw {
            id: "LOWW_TWR".into(),
            prefixes: HashSet::from(["LOWW".to_string()]),
            frequency: "119.400".to_string(),
            facility_type: FacilityType::Unknown,
        };
        assert_matches!(
            raw.validate(),
            Err(CoverageError::Validation(ValidationError::InvalidValue { field, value, .. }))
                if field == "facility_type" && value == "Unknown"
        );
    }

    #[test]
    fn position_conversion() {
        let raw = PositionRaw {
            id: "LOWW_TWR".into(),
            prefixes: HashSet::from(["LOWW".to_string()]),
            frequency: "119.400".to_string(),
            facility_type: FacilityType::Tower,
        };
        let pos = Position::from_raw(raw, "LOVV").unwrap();
        assert_eq!(pos.id.as_str(), "LOWW_TWR");
        assert_eq!(pos.fir_id.as_str(), "LOVV");
    }

    #[test]
    fn position_equality() {
        let p1 = Position {
            id: "LOWW_TWR".into(),
            prefixes: HashSet::from(["LOWW".to_string()]),
            frequency: "119.400".to_string(),
            facility_type: FacilityType::Tower,
            fir_id: FlightInformationRegionId::from("LOVV"),
        };
        let p2 = Position {
            id: "LOWW_TWR".into(),
            prefixes: HashSet::new(),            // Different content
            frequency: "119.000".to_string(),    // Different content
            facility_type: FacilityType::Ground, // Different content
            fir_id: FlightInformationRegionId::from("LOVV"),
        };
        assert_eq!(p1, p2); // Should be equal because IDs are equal

        let p3 = Position {
            id: "LOWW_GND".into(),
            prefixes: HashSet::from(["LOWW".to_string()]),
            frequency: "119.400".to_string(),
            facility_type: FacilityType::Tower,
            fir_id: FlightInformationRegionId::from("LOVV"),
        };
        assert_ne!(p1, p3);
    }
}
