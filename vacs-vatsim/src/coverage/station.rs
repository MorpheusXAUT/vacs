use crate::coverage::flight_information_region::FlightInformationRegionId;
use crate::coverage::position::PositionId;
use crate::coverage::{CoverageError, ValidationError, Validator};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct StationId(String);

#[derive(Debug, Clone)]
pub struct Station {
    pub id: StationId,
    pub parent_id: Option<StationId>,
    pub controlled_by: Vec<PositionId>,
    pub fir_id: FlightInformationRegionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StationRaw {
    pub id: StationId,
    #[serde(default)]
    pub parent_id: Option<StationId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controlled_by: Vec<PositionId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StationConfigFile {
    pub stations: Vec<StationRaw>,
}

impl PartialEq for Station {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Validator for StationRaw {
    fn validate(&self) -> Result<(), CoverageError> {
        if self.id.is_empty() {
            return Err(ValidationError::MissingField("id".to_string()).into());
        }
        Ok(())
    }
}

impl StationRaw {
    pub fn resolve_controlled_by(
        &self,
        fir_id: FlightInformationRegionId,
        all_stations: &HashMap<StationId, &StationRaw>,
    ) -> Station {
        let mut controlled_by = Vec::new();
        let mut visited_stations = HashSet::new();
        let mut seen_positions = HashSet::new();
        let mut current = Some(self);

        // Positions matching the station itself are always included as explicit matches, even if
        // they are not explicitly listed in the station's coverage.
        controlled_by.push(PositionId::from(self.id.as_str()));
        seen_positions.insert(PositionId::from(self.id.as_str()));

        while let Some(station) = current {
            if !visited_stations.insert(&station.id) {
                tracing::warn!(
                    ?station,
                    "Cycle detected in station parent chain, stopping resolution"
                );
                break;
            }

            // Insert position matching the parent station exactly, same as with the leaf station above.
            if seen_positions.insert(PositionId::from(station.id.as_str())) {
                controlled_by.push(PositionId::from(station.id.as_str()));
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

        Station {
            id: self.id.clone(),
            parent_id: self.parent_id.clone(),
            controlled_by,
            fir_id,
        }
    }
}

impl StationId {
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
    fn from(value: String) -> Self {
        StationId(value.to_ascii_uppercase())
    }
}

impl From<&str> for StationId {
    fn from(value: &str) -> Self {
        StationId(value.to_ascii_uppercase())
    }
}

impl std::borrow::Borrow<str> for StationId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_matches};

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
    fn station_raw_valid() {
        let raw1 = StationRaw {
            id: "LOWW_TWR".into(),
            parent_id: None,
            controlled_by: vec![],
        };
        assert!(raw1.validate().is_ok());

        let raw2 = StationRaw {
            id: "LOWW_TWR".into(),
            parent_id: Some("LOWW_APP".into()),
            controlled_by: vec![],
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
            Err(CoverageError::Validation(ValidationError::MissingField(f))) if f == "id"
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
        let fir_id = FlightInformationRegionId::from("LOVV");
        let station = StationRaw {
            id: "LOWW_TWR".into(),
            parent_id: None,
            controlled_by: vec!["LOWW_APP".into()],
        };
        let all_stations = HashMap::from([("LOWW_TWR".into(), &station)]);

        let resolved = station.resolve_controlled_by(fir_id.clone(), &all_stations);

        // Should contain itself (implicit) + explicit controlled_by
        let expected_ids: Vec<PositionId> = vec!["LOWW_TWR", "LOWW_APP"]
            .into_iter()
            .map(PositionId::from)
            .collect();
        let actual_ids: Vec<PositionId> = resolved.controlled_by.into_iter().collect();

        assert_eq!(actual_ids, expected_ids);
        assert_eq!(resolved.id, station.id);
        assert_eq!(resolved.fir_id, fir_id);
    }

    #[test]
    fn resolve_controlled_by_inheritance() {
        let fir_id = FlightInformationRegionId::from("LOVV");

        let parent = StationRaw {
            id: "LOVV_CTR".into(),
            parent_id: None,
            controlled_by: vec!["LOVV_CTR".into()],
        };
        let child = StationRaw {
            id: "LOWW_TWR".into(),
            parent_id: Some("LOVV_CTR".into()),
            controlled_by: vec!["LOWW_APP".into()],
        };

        let map = HashMap::from([(parent.id.clone(), &parent), (child.id.clone(), &child)]);

        let resolved = child.resolve_controlled_by(fir_id, &map);

        let expected_ids: Vec<PositionId> = vec![
            "LOWW_TWR", // Self
            "LOWW_APP", // Self explicit
            "LOVV_CTR", // Parent inherited
        ]
        .into_iter()
        .map(PositionId::from)
        .collect();
        let actual_ids: Vec<PositionId> = resolved.controlled_by.into_iter().collect();

        assert_eq!(actual_ids, expected_ids);
    }

    #[test]
    fn resolve_controlled_by_complex_chain() {
        let fir_id = FlightInformationRegionId::from("LOVV");

        let root = StationRaw {
            id: "LOVV_CTR".into(),
            parent_id: None,
            controlled_by: vec![],
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
            controlled_by: vec![],
        };

        let map = HashMap::from([
            (root.id.clone(), &root),
            (intermediate1.id.clone(), &intermediate1),
            (intermediate2.id.clone(), &intermediate2),
            (intermediate3.id.clone(), &intermediate3),
            (intermediate4.id.clone(), &intermediate4),
            (leaf.id.clone(), &leaf),
        ]);

        let resolved = leaf.resolve_controlled_by(fir_id, &map);

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
        let actual_ids: Vec<PositionId> = resolved.controlled_by.into_iter().collect();

        assert_eq!(actual_ids, expected_ids);
    }

    #[test]
    fn resolve_controlled_by_unique_positions() {
        let fir_id = FlightInformationRegionId::from("LOVV");

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

        let map = HashMap::from([(parent.id.clone(), &parent), (child.id.clone(), &child)]);

        let resolved = child.resolve_controlled_by(fir_id, &map);

        let expected_ids: Vec<PositionId> = vec![
            "LOWW_W_GND", // Self
            "LOWW_GND",   // Self explicit
        ]
        .into_iter()
        .map(PositionId::from)
        .collect();
        let actual_ids: Vec<PositionId> = resolved.controlled_by.into_iter().collect();

        assert_eq!(actual_ids, expected_ids);
    }

    #[test]
    fn resolve_controlled_by_cycle() {
        let fir_id = FlightInformationRegionId::from("LOVV");

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

        let map = HashMap::from([(s1.id.clone(), &s1), (s2.id.clone(), &s2)]);

        let resolved = s1.resolve_controlled_by(fir_id, &map);

        // Should resolve A, then go to B, then see A and stop.
        // Result: A's positions and B's positions.
        let expected_ids: Vec<PositionId> = vec![
            "A",     // Self
            "POS_A", // Self explicit
            "B",     // From B
            "POS_B", // From B explicit
        ]
        .into_iter()
        .map(PositionId::from)
        .collect();
        let actual_ids: Vec<PositionId> = resolved.controlled_by.into_iter().collect();

        assert_eq!(actual_ids, expected_ids);
    }

    #[test]
    fn resolve_controlled_by_missing_parent() {
        let fir_id = FlightInformationRegionId::from("LOVV");

        let child = StationRaw {
            id: "LOWW_DEL".into(),
            parent_id: Some("LOWW_GND".into()),
            controlled_by: vec![],
        };

        // Explicitly omit the parent station from the map of all stations.
        let map = HashMap::from([(child.id.clone(), &child)]);

        let resolved = child.resolve_controlled_by(fir_id, &map);

        let expected_ids: Vec<PositionId> = vec![
            "LOWW_DEL", // Self
        ]
        .into_iter()
        .map(PositionId::from)
        .collect();
        let actual_ids: Vec<PositionId> = resolved.controlled_by.into_iter().collect();

        assert_eq!(actual_ids, expected_ids);
    }
}
