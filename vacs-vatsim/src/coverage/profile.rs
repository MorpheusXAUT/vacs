use crate::coverage::{CoverageError, ValidationError, Validator};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use vacs_protocol::vatsim::{ProfileId, StationId};

#[derive(Debug, Clone)]
pub struct Profile {
    pub id: ProfileId,
    pub profile_type: ProfileType,
}

#[derive(Debug, Clone)]
pub enum ProfileType {
    Geo(Vec<GeoPageButton>),                   // len > 0
    Tabbed(HashMap<String, DirectAccessPage>), // len > 0
}

#[derive(Debug, Clone)]
pub struct GeoPageButton {
    pub label: Vec<String>, // len 0..=3
    pub x: u8,              // 0..=100
    pub y: u8,              // 0..=100
    pub size: u8,           // 0..=100
    pub page: DirectAccessPage,
}

#[derive(Debug, Clone)]
pub struct DirectAccessPage {
    pub keys: Vec<DirectAccessKey>, // len > 0
    pub rows: Option<u8>,           // XOR columns
    pub columns: Option<u8>,        // XOR rows
}

#[derive(Debug, Clone)]
pub struct DirectAccessKey {
    pub label: Vec<String>, // len 0..=3
    pub station_id: Option<StationId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ProfileRaw {
    pub id: ProfileId,
    #[serde(flatten)]
    pub profile_type: ProfileTypeRaw,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(super) enum ProfileTypeRaw {
    Geo {
        buttons: Vec<GeoPageButtonRaw>,
    },
    Tabbed {
        tabs: HashMap<String, DirectAccessPageRaw>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct GeoPageButtonRaw {
    pub label: Vec<String>,
    pub x: u8,
    pub y: u8,
    pub size: u8,
    pub page: DirectAccessPageRaw,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct DirectAccessPageRaw {
    pub keys: Vec<DirectAccessKeyRaw>,
    #[serde(default)]
    pub rows: Option<u8>,
    #[serde(default)]
    pub columns: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct DirectAccessKeyRaw {
    pub label: Vec<String>,
    #[serde(default)]
    pub station_id: Option<StationId>,
}

impl PartialEq for Profile {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for Profile {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Profile {
    pub(super) fn from_raw(profile_raw: ProfileRaw) -> Result<Self, CoverageError> {
        profile_raw.validate()?;

        let profile_type = match profile_raw.profile_type {
            ProfileTypeRaw::Geo { buttons } => ProfileType::Geo(
                buttons
                    .into_iter()
                    .map(GeoPageButton::from_raw)
                    .collect::<Result<_, _>>()?,
            ),
            ProfileTypeRaw::Tabbed { tabs } => ProfileType::Tabbed(
                tabs.into_iter()
                    .map(|(k, v)| Ok((k, DirectAccessPage::from_raw(v)?)))
                    .collect::<Result<HashMap<_, _>, CoverageError>>()?,
            ),
        };

        Ok(Self {
            id: profile_raw.id,
            profile_type,
        })
    }

    pub fn validate_references(&self, stations: &HashSet<&StationId>) -> Result<(), CoverageError> {
        self.profile_type.validate_references(stations)
    }
}

impl ProfileType {
    fn validate_references(&self, stations: &HashSet<&StationId>) -> Result<(), CoverageError> {
        match self {
            ProfileType::Geo(buttons) => {
                for button in buttons {
                    button.validate_references(stations)?;
                }
            }
            ProfileType::Tabbed(tabs) => {
                for tab in tabs.values() {
                    tab.validate_references(stations)?;
                }
            }
        }
        Ok(())
    }
}

impl GeoPageButton {
    fn validate_references(&self, stations: &HashSet<&StationId>) -> Result<(), CoverageError> {
        self.page.validate_references(stations)
    }
}

impl DirectAccessPage {
    fn validate_references(&self, stations: &HashSet<&StationId>) -> Result<(), CoverageError> {
        for key in &self.keys {
            key.validate_references(stations)?;
        }
        Ok(())
    }
}

impl DirectAccessKey {
    fn validate_references(&self, stations: &HashSet<&StationId>) -> Result<(), CoverageError> {
        if let Some(station_id) = &self.station_id
            && !stations.contains(station_id)
        {
            return Err(ValidationError::MissingReference {
                field: "station_id".to_string(),
                ref_id: station_id.to_string(),
            }
            .into());
        }
        Ok(())
    }
}

impl GeoPageButton {
    fn from_raw(raw: GeoPageButtonRaw) -> Result<Self, CoverageError> {
        raw.validate()?;
        Ok(Self {
            label: raw.label,
            x: raw.x,
            y: raw.y,
            size: raw.size,
            page: DirectAccessPage::from_raw(raw.page)?,
        })
    }
}

impl DirectAccessPage {
    fn from_raw(raw: DirectAccessPageRaw) -> Result<Self, CoverageError> {
        raw.validate()?;
        Ok(Self {
            keys: raw
                .keys
                .into_iter()
                .map(DirectAccessKey::try_from)
                .collect::<Result<_, _>>()?,
            rows: raw.rows,
            columns: raw.columns,
        })
    }
}

impl TryFrom<DirectAccessKeyRaw> for DirectAccessKey {
    type Error = CoverageError;
    fn try_from(raw: DirectAccessKeyRaw) -> Result<Self, Self::Error> {
        raw.validate()?;
        Ok(Self {
            label: raw.label,
            station_id: raw.station_id,
        })
    }
}

impl Validator for ProfileRaw {
    fn validate(&self) -> Result<(), CoverageError> {
        if self.id.is_empty() {
            return Err(ValidationError::Empty {
                field: "id".to_string(),
            }
            .into());
        }
        self.profile_type.validate()?;
        Ok(())
    }
}

impl Validator for ProfileTypeRaw {
    fn validate(&self) -> Result<(), CoverageError> {
        match self {
            ProfileTypeRaw::Geo { buttons } => {
                if buttons.is_empty() {
                    return Err(ValidationError::Empty {
                        field: "buttons".to_string(),
                    }
                    .into());
                }
                for button in buttons {
                    button.validate()?;
                }
            }
            ProfileTypeRaw::Tabbed { tabs } => {
                if tabs.is_empty() {
                    return Err(ValidationError::Empty {
                        field: "tabs".to_string(),
                    }
                    .into());
                }
                for tab in tabs.values() {
                    tab.validate()?;
                }
            }
        }
        Ok(())
    }
}

impl Validator for GeoPageButtonRaw {
    fn validate(&self) -> Result<(), CoverageError> {
        if self.label.is_empty() {
            return Err(ValidationError::Empty {
                field: "label".to_string(),
            }
            .into());
        }
        if self.label.len() > 3 {
            return Err(ValidationError::InvalidValue {
                field: "label".to_string(),
                value: format!("{:?}", self.label),
                reason: "cannot have more than 3 lines".to_string(),
            }
            .into());
        }
        if self.x > 100 {
            return Err(ValidationError::OutOfRange {
                field: "x".to_string(),
                value: self.x.to_string(),
                min: "0".to_string(),
                max: "100".to_string(),
            }
            .into());
        }
        if self.y > 100 {
            return Err(ValidationError::OutOfRange {
                field: "y".to_string(),
                value: self.y.to_string(),
                min: "0".to_string(),
                max: "100".to_string(),
            }
            .into());
        }
        if self.size == 0 || self.size > 100 {
            return Err(ValidationError::OutOfRange {
                field: "size".to_string(),
                value: self.size.to_string(),
                min: "1".to_string(),
                max: "100".to_string(),
            }
            .into());
        }
        self.page.validate()?;
        Ok(())
    }
}

impl Validator for DirectAccessPageRaw {
    fn validate(&self) -> Result<(), CoverageError> {
        if self.columns.is_some() && self.rows.is_some() {
            return Err(ValidationError::Custom(
                "cannot specify both `rows` and `columns`".to_string(),
            )
            .into());
        }
        for key in &self.keys {
            key.validate()?;
        }
        Ok(())
    }
}

impl Validator for DirectAccessKeyRaw {
    fn validate(&self) -> Result<(), CoverageError> {
        if self.label.is_empty() {
            return Err(ValidationError::Empty {
                field: "label".to_string(),
            }
            .into());
        }
        if self.label.len() > 3 {
            return Err(ValidationError::InvalidValue {
                field: "label".to_string(),
                value: format!("{:?}", self.label),
                reason: "cannot have more than 3 lines".to_string(),
            }
            .into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coverage::{CoverageError, ValidationError};
    use pretty_assertions::assert_matches;

    #[test]
    fn profile_raw_validation() {
        let valid_geo = ProfileRaw {
            id: ProfileId::from("geo"),
            profile_type: ProfileTypeRaw::Geo {
                buttons: vec![GeoPageButtonRaw {
                    label: vec!["L".to_string()],
                    x: 0,
                    y: 0,
                    size: 10,
                    page: DirectAccessPageRaw {
                        keys: vec![],
                        rows: None,
                        columns: None,
                    },
                }],
            },
        };
        assert!(valid_geo.validate().is_ok());

        let empty_id = ProfileRaw {
            id: ProfileId::from(""),
            profile_type: ProfileTypeRaw::Geo {
                buttons: vec![GeoPageButtonRaw {
                    label: vec!["L".to_string()],
                    x: 0,
                    y: 0,
                    size: 10,
                    page: DirectAccessPageRaw {
                        keys: vec![],
                        rows: None,
                        columns: None,
                    },
                }],
            },
        };
        assert_matches!(
            empty_id.validate(),
            Err(CoverageError::Validation(ValidationError::Empty { field })) if field == "id"
        );
    }

    #[test]
    fn profile_type_geo_validation() {
        let valid = ProfileTypeRaw::Geo {
            buttons: vec![GeoPageButtonRaw {
                label: vec!["L".to_string()],
                x: 0,
                y: 0,
                size: 10,
                page: DirectAccessPageRaw {
                    keys: vec![],
                    rows: None,
                    columns: None,
                },
            }],
        };
        assert!(valid.validate().is_ok());

        let empty = ProfileTypeRaw::Geo { buttons: vec![] };
        assert_matches!(
            empty.validate(),
            Err(CoverageError::Validation(ValidationError::Empty { field })) if field == "buttons"
        );
    }

    #[test]
    fn profile_type_tabbed_validation() {
        let valid = ProfileTypeRaw::Tabbed {
            tabs: HashMap::from([(
                "tab1".to_string(),
                DirectAccessPageRaw {
                    keys: vec![],
                    rows: None,
                    columns: None,
                },
            )]),
        };
        assert!(valid.validate().is_ok());

        let empty = ProfileTypeRaw::Tabbed {
            tabs: HashMap::new(),
        };
        assert_matches!(
            empty.validate(),
            Err(CoverageError::Validation(ValidationError::Empty { field })) if field == "tabs"
        );
    }

    #[test]
    fn geo_page_button_validation() {
        let valid = GeoPageButtonRaw {
            label: vec!["L".to_string()],
            x: 0,
            y: 0,
            size: 10,
            page: DirectAccessPageRaw {
                keys: vec![],
                rows: None,
                columns: None,
            },
        };
        assert!(valid.validate().is_ok());

        let empty_label = GeoPageButtonRaw {
            label: vec![],
            x: 0,
            y: 0,
            size: 10,
            page: DirectAccessPageRaw {
                keys: vec![],
                rows: None,
                columns: None,
            },
        };
        assert_matches!(
            empty_label.validate(),
            Err(CoverageError::Validation(ValidationError::Empty { field })) if field == "label"
        );

        let long_label = GeoPageButtonRaw {
            label: vec![
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
            ],
            x: 0,
            y: 0,
            size: 10,
            page: DirectAccessPageRaw {
                keys: vec![],
                rows: None,
                columns: None,
            },
        };
        assert_matches!(
            long_label.validate(),
            Err(CoverageError::Validation(ValidationError::InvalidValue { field, .. })) if field == "label"
        );

        let invalid_x = GeoPageButtonRaw {
            label: vec!["L".to_string()],
            x: 101,
            y: 0,
            size: 10,
            page: DirectAccessPageRaw {
                keys: vec![],
                rows: None,
                columns: None,
            },
        };
        assert_matches!(
            invalid_x.validate(),
            Err(CoverageError::Validation(ValidationError::OutOfRange { field, .. })) if field == "x"
        );

        let invalid_y = GeoPageButtonRaw {
            label: vec!["L".to_string()],
            x: 0,
            y: 101,
            size: 10,
            page: DirectAccessPageRaw {
                keys: vec![],
                rows: None,
                columns: None,
            },
        };
        assert_matches!(
            invalid_y.validate(),
            Err(CoverageError::Validation(ValidationError::OutOfRange { field, .. })) if field == "y"
        );

        let zero_size = GeoPageButtonRaw {
            label: vec!["L".to_string()],
            x: 0,
            y: 0,
            size: 0,
            page: DirectAccessPageRaw {
                keys: vec![],
                rows: None,
                columns: None,
            },
        };
        assert_matches!(
            zero_size.validate(),
            Err(CoverageError::Validation(ValidationError::OutOfRange { field, .. })) if field == "size"
        );

        let large_size = GeoPageButtonRaw {
            label: vec!["L".to_string()],
            x: 0,
            y: 0,
            size: 101,
            page: DirectAccessPageRaw {
                keys: vec![],
                rows: None,
                columns: None,
            },
        };
        assert_matches!(
            large_size.validate(),
            Err(CoverageError::Validation(ValidationError::OutOfRange { field, .. })) if field == "size"
        );
    }

    #[test]
    fn direct_access_page_validation() {
        let valid_none = DirectAccessPageRaw {
            keys: vec![],
            rows: None,
            columns: None,
        };
        assert!(valid_none.validate().is_ok());

        let valid_rows = DirectAccessPageRaw {
            keys: vec![],
            rows: Some(1),
            columns: None,
        };
        assert!(valid_rows.validate().is_ok());

        let valid_columns = DirectAccessPageRaw {
            keys: vec![],
            rows: None,
            columns: Some(1),
        };
        assert!(valid_columns.validate().is_ok());

        let invalid = DirectAccessPageRaw {
            keys: vec![],
            rows: Some(1),
            columns: Some(1),
        };
        assert_matches!(
            invalid.validate(),
            Err(CoverageError::Validation(ValidationError::Custom(msg))) if msg == "cannot specify both `rows` and `columns`"
        );
    }

    #[test]
    fn direct_access_key_validation() {
        let valid = DirectAccessKeyRaw {
            label: vec!["L".to_string()],
            station_id: None,
        };
        assert!(valid.validate().is_ok());

        let empty_label = DirectAccessKeyRaw {
            label: vec![],
            station_id: None,
        };
        assert_matches!(
            empty_label.validate(),
            Err(CoverageError::Validation(ValidationError::Empty { field })) if field == "label"
        );

        let long_label = DirectAccessKeyRaw {
            label: vec![
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
            ],
            station_id: None,
        };
        assert_matches!(
            long_label.validate(),
            Err(CoverageError::Validation(ValidationError::InvalidValue { field, .. })) if field == "label"
        );
    }
}
