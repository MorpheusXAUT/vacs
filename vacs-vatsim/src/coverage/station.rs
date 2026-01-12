use crate::coverage::flight_information_region::FlightInformationRegionId;
use crate::coverage::{CoverageError, ReferenceValidator, ValidationError, Validator};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use vacs_protocol::vatsim::{PositionId, StationId};

#[derive(Clone)]
pub struct Station {
    pub id: StationId,
    pub parent_id: Option<StationId>,
    pub controlled_by: Vec<PositionId>,
    pub fir_id: FlightInformationRegionId,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct StationRaw {
    pub id: StationId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<StationId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controlled_by: Vec<PositionId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StationConfigFile {
    pub stations: Vec<StationRaw>,
}

impl std::fmt::Debug for Station {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Station")
            .field("id", &self.id)
            .field("parent_id", &self.parent_id)
            .field("controlled_by", &self.controlled_by.len())
            .field("fir_id", &self.fir_id)
            .finish()
    }
}

impl PartialEq for Station {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for Station {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Station {
    pub(super) fn from_raw(
        station_raw: StationRaw,
        fir_id: impl Into<FlightInformationRegionId>,
        all_stations: &HashMap<StationId, &StationRaw>,
    ) -> Result<Self, CoverageError> {
        station_raw.validate()?;

        let controlled_by = station_raw.resolve_controlled_by(all_stations);
        Ok(Self {
            id: station_raw.id,
            parent_id: station_raw.parent_id,
            controlled_by,
            fir_id: fir_id.into(),
        })
    }
}

impl std::fmt::Debug for StationRaw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StationRaw")
            .field("id", &self.id)
            .field("parent_id", &self.parent_id)
            .field("controlled_by", &self.controlled_by.len())
            .finish()
    }
}

impl Validator for StationRaw {
    fn validate(&self) -> Result<(), CoverageError> {
        if self.id.is_empty() {
            return Err(ValidationError::Empty {
                field: "id".to_string(),
            }
            .into());
        }
        Ok(())
    }
}

impl ReferenceValidator<PositionId> for StationRaw {
    fn validate_references(&self, positions: &HashSet<&PositionId>) -> Result<(), CoverageError> {
        if let Some(position_id) = self.controlled_by.iter().find(|p| !positions.contains(p)) {
            return Err(ValidationError::MissingReference {
                field: "position_id".to_string(),
                ref_id: position_id.to_string(),
            }
            .into());
        }
        Ok(())
    }
}

impl StationRaw {
    pub(super) fn resolve_controlled_by(
        &self,
        all_stations: &HashMap<StationId, &StationRaw>,
    ) -> Vec<PositionId> {
        let mut controlled_by = Vec::new();
        let mut visited_stations = HashSet::new();
        let mut seen_positions = HashSet::new();
        let mut current = Some(self);

        while let Some(station) = current {
            if !visited_stations.insert(&station.id) {
                tracing::warn!(
                    ?station,
                    "Cycle detected in station parent chain, stopping resolution"
                );
                break;
            }

            controlled_by.extend(
                station
                    .controlled_by
                    .iter()
                    .filter(|pos| seen_positions.insert((*pos).clone()))
                    .cloned(),
            );

            current = station.parent_id.as_ref().and_then(|id| {
                let parent = all_stations.get(id).copied();
                if parent.is_none() {
                    tracing::warn!(?id, ?self, "Parent station not found, stopping resolution");
                }
                parent
            });
        }

        controlled_by
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_matches};

    #[test]
    fn station_raw_valid() {
        let raw1 = StationRaw {
            id: "LOWW_TWR".into(),
            parent_id: None,
            controlled_by: vec!["LOWW_TWR".into()],
        };
        assert!(raw1.validate().is_ok());

        let raw2 = StationRaw {
            id: "LOWW_TWR".into(),
            parent_id: Some("LOWW_APP".into()),
            controlled_by: vec!["LOWW_TWR".into()],
        };
        assert!(raw2.validate().is_ok());
    }

    #[test]
    fn station_raw_invalid_id() {
        let raw = StationRaw {
            id: "".into(),
            parent_id: None,
            controlled_by: vec![],
        };
        assert_matches!(
            raw.validate(),
            Err(CoverageError::Validation(ValidationError::Empty { field })) if field == "id"
        );
    }

    #[test]
    fn station_equality() {
        let s1 = Station {
            id: "LOWW_TWR".into(),
            parent_id: None,
            controlled_by: vec![],
            fir_id: "LOVV".into(),
        };
        let s2 = Station {
            id: "LOWW_TWR".into(),
            parent_id: Some("LOWW_APP".into()),     // Different
            controlled_by: vec!["LOWW_TWR".into()], // Different
            fir_id: "LOVV".into(),
        };
        assert_eq!(s1, s2);
    }

    #[test]
    fn resolve_controlled_by_simple() {
        let station = StationRaw {
            id: "LOWW_TWR".into(),
            parent_id: None,
            controlled_by: vec!["LOWW_TWR".into(), "LOWW_APP".into()],
        };
        let all_stations = HashMap::from([("LOWW_TWR".into(), &station)]);

        let actual_ids = station.resolve_controlled_by(&all_stations);
        // Should contain explicit controlled_by
        let expected_ids: Vec<PositionId> = vec!["LOWW_TWR", "LOWW_APP"]
            .into_iter()
            .map(PositionId::from)
            .collect();

        assert_eq!(actual_ids, expected_ids);
    }

    #[test]
    fn resolve_controlled_by_inheritance() {
        let parent = StationRaw {
            id: "LOVV_CTR".into(),
            parent_id: None,
            controlled_by: vec!["LOVV_CTR".into()],
        };
        let child = StationRaw {
            id: "LOWW_TWR".into(),
            parent_id: Some("LOVV_CTR".into()),
            controlled_by: vec!["LOWW_TWR".into(), "LOWW_APP".into()],
        };

        let all_stations =
            HashMap::from([(parent.id.clone(), &parent), (child.id.clone(), &child)]);

        let actual_ids = child.resolve_controlled_by(&all_stations);
        let expected_ids: Vec<PositionId> = vec![
            "LOWW_TWR", // Self
            "LOWW_APP", // Self
            "LOVV_CTR", // Parent inherited
        ]
        .into_iter()
        .map(PositionId::from)
        .collect();

        assert_eq!(actual_ids, expected_ids);
    }

    #[test]
    fn resolve_controlled_by_complex_chain() {
        let root = StationRaw {
            id: "LOVV_CTR".into(),
            parent_id: None,
            controlled_by: vec!["LOVV_CTR".into()],
        };

        let intermediate1 = StationRaw {
            id: "LOWW_APP".into(),
            parent_id: Some("LOVV_CTR".into()),
            controlled_by: vec!["LOWW_APP".into(), "LOWW_B_APP".into(), "LOWW_P_APP".into()],
        };

        let intermediate2 = StationRaw {
            id: "LOWW_TWR".into(),
            parent_id: Some("LOWW_APP".into()),
            controlled_by: vec!["LOWW_TWR".into(), "LOWW_E_TWR".into()],
        };

        let intermediate3 = StationRaw {
            id: "LOWW_E_TWR".into(),
            parent_id: Some("LOWW_TWR".into()),
            controlled_by: vec!["LOWW_E_TWR".into(), "LOWW_TWR".into()],
        };

        let intermediate4 = StationRaw {
            id: "LOWW_GND".into(),
            parent_id: Some("LOWW_E_TWR".into()),
            controlled_by: vec!["LOWW_GND".into(), "LOWW_W_GND".into()],
        };

        let leaf = StationRaw {
            id: "LOWW_DEL".into(),
            parent_id: Some("LOWW_GND".into()),
            controlled_by: vec!["LOWW_DEL".into()],
        };

        let all_stations = HashMap::from([
            (root.id.clone(), &root),
            (intermediate1.id.clone(), &intermediate1),
            (intermediate2.id.clone(), &intermediate2),
            (intermediate3.id.clone(), &intermediate3),
            (intermediate4.id.clone(), &intermediate4),
            (leaf.id.clone(), &leaf),
        ]);

        let actual_ids = leaf.resolve_controlled_by(&all_stations);

        let expected_ids: Vec<PositionId> = vec![
            "LOWW_DEL",   // Self
            "LOWW_GND",   // Intermediate4
            "LOWW_W_GND", // Intermediate4
            "LOWW_E_TWR", // Intermediate3
            "LOWW_TWR",   // Intermediate2
            "LOWW_APP",   // Intermediate1
            "LOWW_B_APP", // Intermediate1
            "LOWW_P_APP", // Intermediate1
            "LOVV_CTR",   // Root
        ]
        .into_iter()
        .map(PositionId::from)
        .collect();

        assert_eq!(actual_ids, expected_ids);
    }

    #[test]
    fn resolve_controlled_by_unique_positions() {
        let parent = StationRaw {
            id: "LOWW_GND".into(),
            parent_id: None,
            controlled_by: vec!["LOWW_GND".into(), "LOWW_W_GND".into()],
        };
        let child = StationRaw {
            id: "LOWW_W_GND".into(),
            parent_id: Some("LOWW_GND".into()),
            controlled_by: vec!["LOWW_W_GND".into(), "LOWW_GND".into()],
        };

        let all_stations =
            HashMap::from([(parent.id.clone(), &parent), (child.id.clone(), &child)]);

        let actual_ids = child.resolve_controlled_by(&all_stations);

        let expected_ids: Vec<PositionId> = vec![
            "LOWW_W_GND", // Self
            "LOWW_GND",   // Self
        ]
        .into_iter()
        .map(PositionId::from)
        .collect();

        assert_eq!(actual_ids, expected_ids);
    }

    #[test]
    fn resolve_controlled_by_cycle() {
        let s1 = StationRaw {
            id: "A".into(),
            parent_id: Some("B".into()),
            controlled_by: vec!["POS_A".into()],
        };
        let s2 = StationRaw {
            id: "B".into(),
            parent_id: Some("A".into()), // Cycle back to A
            controlled_by: vec!["POS_B".into()],
        };

        let all_stations = HashMap::from([(s1.id.clone(), &s1), (s2.id.clone(), &s2)]);

        let actual_ids = s1.resolve_controlled_by(&all_stations);

        // Should resolve A, then go to B, then see A and stop.
        // Result: A's positions and B's positions.
        let expected_ids: Vec<PositionId> = vec![
            "POS_A", // Self
            "POS_B", // From B
        ]
        .into_iter()
        .map(PositionId::from)
        .collect();

        assert_eq!(actual_ids, expected_ids);
    }

    #[test]
    fn resolve_controlled_by_missing_parent() {
        let child = StationRaw {
            id: "LOWW_DEL".into(),
            parent_id: Some("LOWW_GND".into()),
            controlled_by: vec!["LOWW_DEL".into()],
        };

        // Explicitly omit the parent station from the map of all stations.
        let all_stations = HashMap::from([(child.id.clone(), &child)]);

        let actual_ids = child.resolve_controlled_by(&all_stations);

        let expected_ids: Vec<PositionId> = vec![
            "LOWW_DEL", // Self
        ]
        .into_iter()
        .map(PositionId::from)
        .collect();

        assert_eq!(actual_ids, expected_ids);
    }
}
