use serde::{Deserialize, Serialize};

use crate::FacilityType;
use crate::coverage::flight_information_region::{
    FlightInformationRegion, FlightInformationRegionId, FlightInformationRegionRaw,
};
use crate::coverage::position::{Position, PositionId};
use crate::coverage::station::{Station, StationId};
use crate::coverage::{CoverageError, IoError, StructureError, Validator};
use std::collections::{HashMap, HashSet};
use std::fs;

#[derive(Debug, Clone, Default)]
pub struct Network {
    #[allow(dead_code)] // TODO remove if not needed
    firs: HashMap<FlightInformationRegionId, FlightInformationRegion>,
    positions: HashMap<PositionId, Position>,
    stations: HashMap<StationId, Station>,
}

impl Network {
    pub fn load_from_dir(dir: impl AsRef<std::path::Path>) -> Result<Self, CoverageError> {
        Self::parse_dir(dir, true).map(|(net, _)| net)
    }

    pub fn validate_dir(
        dir: impl AsRef<std::path::Path>,
    ) -> Result<(Self, Vec<CoverageError>), CoverageError> {
        Self::parse_dir(dir, false)
    }

    #[tracing::instrument(level = "trace", skip_all, fields(callsign = tracing::field::Empty, frequency = tracing::field::Empty, facility_type = tracing::field::Empty))]
    pub fn find_positions(
        &self,
        callsign: impl AsRef<str>,
        frequency: impl AsRef<str>,
        facility_type: impl Into<FacilityType>,
    ) -> Vec<&Position> {
        let callsign = callsign.as_ref();
        let frequency = frequency.as_ref();
        let facility_type = facility_type.into();
        tracing::Span::current()
            .record("callsign", callsign)
            .record("frequency", frequency)
            .record("facility_type", tracing::field::debug(&facility_type));

        let frequency_matches: Vec<&Position> = self
            .positions
            .values()
            .filter(|p| p.frequency == frequency && p.facility_type == facility_type)
            .collect();

        if frequency_matches.len() == 1 {
            tracing::trace!(position = ?frequency_matches[0], "Found exact match for frequency and station type");
            return frequency_matches;
        }

        let clean_callsign = callsign.replace("__", "_").to_ascii_uppercase();

        if let Some(position) = self.positions.get(clean_callsign.as_str()) {
            tracing::trace!(?position, ?clean_callsign, "Found exact match for callsign");
            return vec![position];
        }

        let positions = frequency_matches
            .into_iter()
            .filter(|p| {
                p.prefixes
                    .iter()
                    .any(|prefix| clean_callsign.starts_with(prefix))
            })
            .collect::<Vec<_>>();

        if positions.len() == 1 {
            tracing::trace!(position = ?positions[0], ?clean_callsign, "Found exact match using prefixes");
            return positions;
        }

        tracing::trace!(
            positions = positions.len(),
            ?clean_callsign,
            "Found multiple matches"
        );
        positions
    }

    #[tracing::instrument(level = "trace", skip(self, online_positions), fields(online_positions = online_positions.len()))]
    pub fn covered_stations(
        &'_ self,
        client_position_id: Option<&PositionId>,
        online_positions: &HashSet<PositionId>,
    ) -> Vec<CoveredStation<'_>> {
        self.stations
            .values()
            .filter_map(|station| {
                self.controlling_position(&station.id, online_positions)
                    .map(|position| {
                        let is_self_controlled = client_position_id == Some(&position.id);
                        CoveredStation {
                            station,
                            is_self_controlled,
                        }
                    })
            })
            .collect()
    }

    #[tracing::instrument(level = "trace", skip(self, online_positions), fields(online_positions = online_positions.len()))]
    pub fn controlling_position(
        &self,
        station_id: &StationId,
        online_positions: &HashSet<PositionId>,
    ) -> Option<&Position> {
        let station = self.stations.get(station_id)?;

        station.controlled_by.iter().find_map(|pos_id| {
            if online_positions.contains(pos_id) {
                let position = self.positions.get(pos_id.as_str())?;
                tracing::trace!(?position, "Found position with matching coverage");
                Some(position)
            } else {
                None
            }
        })
    }

    #[tracing::instrument(level = "trace", skip(self, online_positions), fields(online_positions = online_positions.len()))]
    pub fn coverage_diff(
        &self,
        position_id: &PositionId,
        disconnected: bool,
        online_positions: &HashSet<PositionId>,
    ) -> HashMap<PositionId, Vec<StationDiff>> {
        let mut updated_positions = online_positions.clone();
        if disconnected {
            updated_positions.remove(position_id);
        } else {
            updated_positions.insert(position_id.clone());
        }

        let mut diffs: HashMap<PositionId, Vec<StationDiff>> = HashMap::new();

        for station in self.stations.values() {
            let before = self
                .controlling_position(&station.id, online_positions)
                .map(|p| p.id.clone());
            let after = self
                .controlling_position(&station.id, &updated_positions)
                .map(|p| p.id.clone());

            if before == after {
                continue;
            }

            match (before, after) {
                (None, Some(new_pos)) => {
                    tracing::trace!(?new_pos, "Station is now online");
                    diffs.entry(new_pos).or_default().push(StationDiff {
                        station_id: station.id.clone(),
                        diff_type: DiffType::Online,
                    });
                }
                (Some(old_pos), None) => {
                    tracing::trace!(?old_pos, "Station is now offline");
                    diffs.entry(old_pos).or_default().push(StationDiff {
                        station_id: station.id.clone(),
                        diff_type: DiffType::Offline,
                    });
                }
                (Some(old_pos), Some(new_pos)) if old_pos != new_pos => {
                    tracing::trace!(?old_pos, ?new_pos, "Station coverage changed");
                    diffs.entry(old_pos).or_default().push(StationDiff {
                        station_id: station.id.clone(),
                        diff_type: DiffType::LostControl,
                    });
                    diffs.entry(new_pos).or_default().push(StationDiff {
                        station_id: station.id.clone(),
                        diff_type: DiffType::GainedControl,
                    });
                }
                _ => {}
            }
        }

        diffs
    }

    #[tracing::instrument(level = "trace", skip(dir), err, fields(dir = tracing::field::Empty))]
    fn parse_dir(
        dir: impl AsRef<std::path::Path>,
        strict: bool,
    ) -> Result<(Self, Vec<CoverageError>), CoverageError> {
        let dir = dir.as_ref();
        tracing::Span::current().record("dir", tracing::field::debug(dir));

        let entries = fs::read_dir(dir).map_err(|err| IoError::Read {
            path: dir.to_path_buf(),
            reason: err.to_string(),
        })?;

        let mut errors = Vec::new();
        let mut raw_firs = Vec::new();

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    let err: CoverageError = IoError::ReadEntry(err.to_string()).into();
                    if strict {
                        return Err(err);
                    }
                    tracing::warn!(?err, "Failed to read directory entry");
                    errors.push(err);
                    continue;
                }
            };
            let path = entry.path();
            if !path.is_dir() {
                tracing::trace!(?path, "Skipping non-directory entry");
                continue;
            }

            let fir = match FlightInformationRegionRaw::load_from_dir(&path) {
                Ok(fir) => fir,
                Err(err) => {
                    let err: CoverageError = StructureError::Load {
                        entity: "FIR".to_string(),
                        id: path.display().to_string(),
                        reason: err.to_string(),
                    }
                    .into();
                    if strict {
                        return Err(err);
                    }
                    tracing::warn!(?err, ?path, "Failed to load FIR");
                    errors.push(err);
                    continue;
                }
            };

            raw_firs.push(fir);
        }

        let mut firs = HashMap::new();
        let mut stations = HashMap::new();
        let mut positions = HashMap::new();

        let all_stations = raw_firs
            .iter()
            .flat_map(|fir| fir.stations.iter().map(|s| (s.id.clone(), s)))
            .collect::<HashMap<_, _>>();

        for fir_raw in &raw_firs {
            match FlightInformationRegion::try_from(fir_raw.clone()) {
                Ok(fir) => {
                    if firs.contains_key(&fir.id) {
                        let err: CoverageError = StructureError::Duplicate {
                            entity: "FIR".to_string(),
                            id: fir.id.to_string(),
                        }
                        .into();
                        if strict {
                            return Err(err);
                        }
                        tracing::warn!(?fir, "Duplicate FIR ID");
                        errors.push(err);
                        continue;
                    }
                    firs.insert(fir.id.clone(), fir);
                }
                Err(err) => {
                    let err: CoverageError = StructureError::Load {
                        entity: "FIR".to_string(),
                        id: fir_raw.id.to_string(),
                        reason: err.to_string(),
                    }
                    .into();
                    if strict {
                        return Err(err);
                    }
                    tracing::warn!(?err, ?fir_raw, "Failed to parse FIR");
                    errors.push(err);
                    continue;
                }
            };

            for station_raw in &fir_raw.stations {
                if let Err(err) = station_raw.validate() {
                    let err: CoverageError = StructureError::Load {
                        entity: "Station".to_string(),
                        id: station_raw.id.to_string(),
                        reason: err.to_string(),
                    }
                    .into();
                    if strict {
                        return Err(err);
                    }
                    tracing::warn!(?err, ?station_raw, "Failed to validate station");
                    errors.push(err);
                    continue;
                }

                if stations.contains_key(&station_raw.id) {
                    let err: CoverageError = StructureError::Duplicate {
                        entity: "Station".to_string(),
                        id: station_raw.id.to_string(),
                    }
                    .into();
                    if strict {
                        return Err(err);
                    }
                    tracing::warn!(?station_raw, "Duplicate station ID");
                    errors.push(err);
                    continue;
                }

                let station = station_raw.resolve_controlled_by(fir_raw.id.clone(), &all_stations);
                if station.controlled_by.is_empty() {
                    let err: CoverageError =
                        StructureError::EmptyCoverage(station.id.to_string()).into();
                    if strict {
                        return Err(err);
                    }
                    tracing::warn!(?station, "Station has no coverage");
                    errors.push(err);
                }
                stations.insert(station.id.clone(), station);
            }

            for position_raw in &fir_raw.positions {
                match Position::try_from(position_raw.clone()) {
                    Ok(position) => {
                        if positions.contains_key(&position.id) {
                            let err: CoverageError = StructureError::Duplicate {
                                entity: "Position".to_string(),
                                id: position.id.to_string(),
                            }
                            .into();
                            if strict {
                                return Err(err);
                            }
                            tracing::warn!(?position, "Duplicate position ID");
                            errors.push(err);
                            continue;
                        }
                        positions.insert(position.id.clone(), position);
                    }
                    Err(err) => {
                        let err: CoverageError = StructureError::Load {
                            entity: "Position".to_string(),
                            id: position_raw.id.to_string(),
                            reason: err.to_string(),
                        }
                        .into();
                        if strict {
                            return Err(err);
                        }
                        tracing::warn!(?err, ?position_raw, "Failed to parse position");
                        errors.push(err);
                    }
                }
            }
        }

        Ok((
            Self {
                firs,
                positions,
                stations,
            },
            errors,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub enum DiffType {
    #[default]
    Offline,
    LostControl,
    Online,
    GainedControl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationDiff {
    pub station_id: StationId,
    pub diff_type: DiffType,
}

#[derive(Debug, Clone)]
pub struct CoveredStation<'a> {
    pub station: &'a Station,
    pub is_self_controlled: bool,
}
