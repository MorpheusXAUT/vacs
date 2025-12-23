use crate::FacilityType;
use crate::coverage::flight_information_region::FlightInformationRegionId;
use crate::coverage::{CoverageError, ValidationError, Validator};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::LazyLock;

static FREQUENCY_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{3}\.\d{3}$").unwrap());

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PositionId(String);

#[derive(Debug, Clone)]
pub struct Position {
    pub id: PositionId,
    pub prefixes: HashSet<String>,
    pub frequency: String,
    pub facility_type: FacilityType,
    pub fir_id: FlightInformationRegionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

        Ok(Position {
            id: position_raw.id,
            prefixes: position_raw.prefixes,
            frequency: position_raw.frequency,
            facility_type: position_raw.facility_type,
            fir_id: fir_id.into(),
        })
    }
}

impl Validator for PositionRaw {
    fn validate(&self) -> Result<(), CoverageError> {
        if self.id.is_empty() {
            return Err(ValidationError::MissingField("id".to_string()).into());
        }
        if self.prefixes.is_empty() || self.prefixes.iter().any(|p| p.is_empty()) {
            return Err(ValidationError::MissingField("prefixes".to_string()).into());
        }
        if self.frequency.is_empty() {
            return Err(ValidationError::MissingField("frequency".to_string()).into());
        } else if !FREQUENCY_REGEX.is_match(&self.frequency) {
            return Err(ValidationError::InvalidFormat {
                field: "frequency".to_string(),
                value: self.frequency.clone(),
                reason: "must match pattern XXX.XXX".to_string(),
            }
            .into());
        }
        if self.facility_type == FacilityType::Unknown {
            return Err(
                ValidationError::Custom("Facility type must not be Unknown".to_string()).into(),
            );
        }
        Ok(())
    }
}

impl PositionId {
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
    fn from(value: String) -> Self {
        PositionId(value.to_ascii_uppercase())
    }
}

impl From<&str> for PositionId {
    fn from(value: &str) -> Self {
        PositionId(value.to_ascii_uppercase())
    }
}

impl std::borrow::Borrow<str> for PositionId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_matches, assert_ne};

    #[test]
    fn position_id_creation() {
        let pos_id = PositionId::from("loww_twr");
        assert_eq!(pos_id.as_str(), "LOWW_TWR");
        assert_eq!(pos_id.to_string(), "LOWW_TWR");
        assert!(!pos_id.is_empty());

        let empty_id = PositionId::from("");
        assert!(empty_id.is_empty());
    }

    #[test]
    fn position_id_equality() {
        let id1 = PositionId::from("LOWW_TWR");
        let id2 = PositionId::from("loww_twr");
        assert_eq!(id1, id2);
    }

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
            Err(CoverageError::Validation(ValidationError::MissingField(f))) if f == "id"
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
            Err(CoverageError::Validation(ValidationError::MissingField(f))) if f == "prefixes"
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
            Err(CoverageError::Validation(ValidationError::MissingField(f))) if f == "prefixes"
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
            Err(CoverageError::Validation(ValidationError::MissingField(f))) if f == "frequency"
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
            Err(CoverageError::Validation(ValidationError::Custom(msg))) if msg == "Facility type must not be Unknown"
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
