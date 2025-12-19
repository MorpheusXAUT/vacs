use crate::FacilityType;
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

impl TryFrom<PositionRaw> for Position {
    type Error = CoverageError;
    fn try_from(value: PositionRaw) -> Result<Self, Self::Error> {
        value.validate()?;

        Ok(Self {
            id: value.id,
            prefixes: value.prefixes,
            frequency: value.frequency,
            facility_type: value.facility_type,
        })
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
