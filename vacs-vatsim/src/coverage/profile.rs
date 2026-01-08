use crate::coverage::{CoverageError, ReferenceValidator, ValidationError, Validator};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use vacs_protocol::vatsim::{
    DirectAccessKey, DirectAccessPage, GeoPageButton, Profile as ProtocolProfile, ProfileId,
    ProfileType, StationId,
};

#[derive(Clone)]
pub struct Profile {
    pub id: ProfileId,
    pub profile_type: ProfileType,
    pub relevant_station_ids: HashSet<StationId>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct ProfileRaw {
    pub id: ProfileId,
    #[serde(flatten)]
    pub profile_type: ProfileTypeRaw,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(super) enum ProfileTypeRaw {
    Geo {
        buttons: Vec<GeoPageButtonRaw>,
    },
    Tabbed {
        tabs: HashMap<String, DirectAccessPageRaw>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct GeoPageButtonRaw {
    pub label: Vec<String>,
    pub x: u8,
    pub y: u8,
    pub size: u8,
    pub page: DirectAccessPageRaw,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct DirectAccessPageRaw {
    pub keys: Vec<DirectAccessKeyRaw>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct DirectAccessKeyRaw {
    pub label: Vec<String>,
    #[serde(default)]
    pub station_id: Option<StationId>,
}

pub(super) trait FromRaw<T> {
    fn from_raw(raw: T) -> Result<Self, CoverageError>
    where
        Self: Sized;
}

trait StationIdCollector {
    fn collect_station_ids(&self, ids: &mut HashSet<StationId>);
}

impl std::fmt::Debug for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Profile")
            .field("id", &self.id)
            .field("profile_type", &self.profile_type)
            .field("relevant_stations", &self.relevant_station_ids.len())
            .finish()
    }
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

impl FromRaw<ProfileRaw> for Profile {
    fn from_raw(profile_raw: ProfileRaw) -> Result<Self, CoverageError> {
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

        let mut relevant_station_ids = HashSet::new();
        profile_type.collect_station_ids(&mut relevant_station_ids);

        Ok(Self {
            id: profile_raw.id,
            profile_type,
            relevant_station_ids,
        })
    }
}

impl From<&Profile> for ProtocolProfile {
    fn from(profile: &Profile) -> Self {
        Self {
            id: profile.id.clone(),
            profile_type: profile.profile_type.clone(),
        }
    }
}

impl ReferenceValidator<StationId> for Profile {
    fn validate_references(&self, stations: &HashSet<&StationId>) -> Result<(), CoverageError> {
        self.profile_type.validate_references(stations)
    }
}

impl ReferenceValidator<StationId> for ProfileType {
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

impl StationIdCollector for ProfileType {
    fn collect_station_ids(&self, ids: &mut HashSet<StationId>) {
        match self {
            ProfileType::Geo(buttons) => {
                for button in buttons {
                    button.page.collect_station_ids(ids);
                }
            }
            ProfileType::Tabbed(tabs) => {
                for page in tabs.values() {
                    page.collect_station_ids(ids);
                }
            }
        }
    }
}

impl ReferenceValidator<StationId> for GeoPageButton {
    fn validate_references(&self, stations: &HashSet<&StationId>) -> Result<(), CoverageError> {
        self.page.validate_references(stations)
    }
}

impl ReferenceValidator<StationId> for DirectAccessPage {
    fn validate_references(&self, stations: &HashSet<&StationId>) -> Result<(), CoverageError> {
        for key in &self.keys {
            key.validate_references(stations)?;
        }
        Ok(())
    }
}

impl StationIdCollector for DirectAccessPage {
    fn collect_station_ids(&self, ids: &mut HashSet<StationId>) {
        for key in &self.keys {
            if let Some(station_id) = &key.station_id {
                ids.insert(station_id.clone());
            }
        }
    }
}

impl ReferenceValidator<StationId> for DirectAccessKey {
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

impl FromRaw<GeoPageButtonRaw> for GeoPageButton {
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

impl FromRaw<DirectAccessPageRaw> for DirectAccessPage {
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

impl std::fmt::Debug for ProfileRaw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfileRaw")
            .field("id", &self.id)
            .field("profile_type", &self.profile_type)
            .finish()
    }
}

impl std::fmt::Debug for ProfileTypeRaw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Geo { buttons } => f.debug_tuple("Geo").field(&buttons.len()).finish(),
            Self::Tabbed { tabs } => f.debug_tuple("Tabbed").field(&tabs.len()).finish(),
        }
    }
}

impl std::fmt::Debug for DirectAccessPageRaw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectAccessPageRaw")
            .field("keys", &self.keys.len())
            .field("rows", &self.rows)
            .field("columns", &self.columns)
            .finish()
    }
}

impl std::fmt::Debug for GeoPageButtonRaw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeoPageButtonRaw")
            .field("label", &self.label.len())
            .field("x", &self.x)
            .field("y", &self.y)
            .field("size", &self.size)
            .field("page", &self.page)
            .finish()
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

impl std::fmt::Debug for DirectAccessKeyRaw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectAccessKeyRaw")
            .field("label", &self.label.len())
            .field("station_id", &self.station_id)
            .finish()
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

    #[test]
    fn profile_relevant_stations() {
        let raw = ProfileRaw {
            id: ProfileId::from("test"),
            profile_type: ProfileTypeRaw::Geo {
                buttons: vec![
                    GeoPageButtonRaw {
                        label: vec!["B1".to_string()],
                        x: 0,
                        y: 0,
                        size: 10,
                        page: DirectAccessPageRaw {
                            keys: vec![DirectAccessKeyRaw {
                                label: vec!["K1".to_string()],
                                station_id: Some(StationId::from("S1")),
                            }],
                            rows: None,
                            columns: None,
                        },
                    },
                    GeoPageButtonRaw {
                        label: vec!["B2".to_string()],
                        x: 10,
                        y: 10,
                        size: 10,
                        page: DirectAccessPageRaw {
                            keys: vec![
                                DirectAccessKeyRaw {
                                    label: vec!["K2".to_string()],
                                    station_id: Some(StationId::from("S2")),
                                },
                                DirectAccessKeyRaw {
                                    label: vec!["K3".to_string()],
                                    station_id: Some(StationId::from("S1")), // Duplicate
                                },
                                DirectAccessKeyRaw {
                                    label: vec!["K4".to_string()],
                                    station_id: None,
                                },
                            ],
                            rows: None,
                            columns: None,
                        },
                    },
                ],
            },
        };

        let profile = Profile::from_raw(raw).expect("Should be valid");
        let expected = HashSet::from([StationId::from("S1"), StationId::from("S2")]);
        assert_eq!(profile.relevant_station_ids, expected);
    }

    #[test]
    fn validate_references() {
        let station_id = StationId::from("S1");
        let other_station_id = StationId::from("S2");
        let valid_stations = HashSet::from([&station_id, &other_station_id]);

        let raw = ProfileRaw {
            id: ProfileId::from("test"),
            profile_type: ProfileTypeRaw::Geo {
                buttons: vec![GeoPageButtonRaw {
                    label: vec!["L".to_string()],
                    x: 0,
                    y: 0,
                    size: 10,
                    page: DirectAccessPageRaw {
                        keys: vec![DirectAccessKeyRaw {
                            label: vec!["K1".to_string()],
                            station_id: Some(station_id.clone()),
                        }],
                        rows: None,
                        columns: None,
                    },
                }],
            },
        };
        let profile = Profile::from_raw(raw).expect("Should be valid");
        assert!(profile.validate_references(&valid_stations).is_ok());

        let raw_missing = ProfileRaw {
            id: ProfileId::from("test3"),
            profile_type: ProfileTypeRaw::Geo {
                buttons: vec![GeoPageButtonRaw {
                    label: vec!["L".to_string()],
                    x: 0,
                    y: 0,
                    size: 10,
                    page: DirectAccessPageRaw {
                        keys: vec![DirectAccessKeyRaw {
                            label: vec!["K3".to_string()],
                            station_id: Some(StationId::from("MISSING")),
                        }],
                        rows: None,
                        columns: None,
                    },
                }],
            },
        };
        let profile_missing = Profile::from_raw(raw_missing).expect("Should be valid");
        assert_matches!(
            profile_missing.validate_references(&valid_stations),
            Err(CoverageError::Validation(ValidationError::MissingReference { field, ref_id }))
            if field == "station_id" && ref_id == "MISSING"
        );

        let raw_none = ProfileRaw {
            id: ProfileId::from("test4"),
            profile_type: ProfileTypeRaw::Geo {
                buttons: vec![GeoPageButtonRaw {
                    label: vec!["L".to_string()],
                    x: 0,
                    y: 0,
                    size: 10,
                    page: DirectAccessPageRaw {
                        keys: vec![DirectAccessKeyRaw {
                            label: vec!["K4".to_string()],
                            station_id: None,
                        }],
                        rows: None,
                        columns: None,
                    },
                }],
            },
        };
        let profile_none = Profile::from_raw(raw_none).expect("Should be valid");
        assert!(profile_none.validate_references(&valid_stations).is_ok());
    }
}
