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
