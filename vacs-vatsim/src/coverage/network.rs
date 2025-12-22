use serde::{Deserialize, Serialize};

use crate::FacilityType;
use crate::coverage::flight_information_region::{
    FlightInformationRegion, FlightInformationRegionId, FlightInformationRegionRaw,
};
use crate::coverage::position::{Position, PositionId};
use crate::coverage::station::{Station, StationId};
use crate::coverage::{CoverageError, IoError, StructureError};
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
        // Normalize callsign (standard relief pattern) and ensure uppercase for matching
        let callsign = callsign.as_ref().replace("__", "_").to_ascii_uppercase();
        let frequency = frequency.as_ref();
        let facility_type = facility_type.into();
        tracing::Span::current()
            .record("callsign", &callsign)
            .record("frequency", frequency)
            .record("facility_type", tracing::field::debug(&facility_type));

        // Check if a position with the exact callsign exists and the frequency and facility type match
        if let Some(position) = self.positions.get(callsign.as_str())
            && position.frequency == frequency
            && position.facility_type == facility_type
        {
            tracing::trace!(?position, "Found exact match for callsign");
            return vec![position];
        }

        // Find all positions with the same frequency and facility type that have a prefix matching the callsign
        let mut positions = self
            .positions
            .values()
            .filter(|p| {
                p.frequency == frequency
                    && p.facility_type == facility_type
                    && p.prefixes.iter().any(|pre| callsign.starts_with(pre))
            })
            .collect::<Vec<_>>();

        if positions.len() == 1 {
            // Non-standard relief/COO callsign, but only one matching position found --> successful match
            tracing::trace!(position = ?positions[0], "Found exact match for frequency and station type");
        } else if positions.is_empty() {
            // No matches found at all (frequency and facility type might yield results, but callsign
            // didn't match any defined prefixes and FIR from callsign doesn't match) --> no match
            tracing::trace!("No matches found");
        } else {
            // Multiple matches found, no automatic selection possible --> user has to select the correct one
            tracing::trace!(positions = positions.len(), "Found multiple matches");
        }

        positions.sort_by_key(|p| p.id.as_str());
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

                let station =
                    match Station::from_raw(station_raw.clone(), fir_raw.id.clone(), &all_stations)
                    {
                        Ok(station) => station,
                        Err(err) => {
                            let err: CoverageError = StructureError::Load {
                                entity: "Station".to_string(),
                                id: station_raw.id.to_string(),
                                reason: err.to_string(),
                            }
                            .into();
                            if strict {
                                return Err(err);
                            }
                            tracing::warn!(?err, ?station_raw, "Failed to parse station");
                            errors.push(err);
                            continue;
                        }
                    };

                if station.controlled_by.is_empty() {
                    let err: CoverageError =
                        StructureError::EmptyCoverage(station.id.to_string()).into();
                    if strict {
                        return Err(err);
                    }
                    tracing::warn!(?station, "Station has no coverage");
                    errors.push(err);
                    continue;
                }
                stations.insert(station.id.clone(), station);
            }

            for position_raw in &fir_raw.positions {
                match Position::from_raw(position_raw.clone(), fir_raw.id.clone()) {
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
                        continue;
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_matches};

    fn create_minimal_valid_fir(dir: &std::path::Path, name: &str) {
        let fir_path = dir.join(name);
        fs::create_dir(&fir_path).unwrap();

        let stations_toml = format!(
            r#"
            [[stations]]
            id = "{name}_CTR"
            controlled_by = ["{name}_CTR"]
        "#
        );
        fs::write(fir_path.join("stations.toml"), stations_toml).unwrap();

        let positions_toml = format!(
            r#"
            [[positions]]
            id = "{name}_CTR"
            prefixes = ["{name}"]
            frequency = "199.998"
            facility_type = "Enroute"
        "#
        );
        fs::write(fir_path.join("positions.toml"), positions_toml).unwrap();
    }

    #[test]
    fn parse_dir_valid_single() {
        let dir = tempfile::tempdir().unwrap();
        create_minimal_valid_fir(dir.path(), "LOVV");

        let (network, errors) = Network::parse_dir(dir.path(), false).unwrap();
        assert_eq!(network.firs.len(), 1);
        assert!(
            network
                .firs
                .contains_key(&FlightInformationRegionId::from("LOVV"))
        );
        assert_eq!(network.stations.len(), 1);
        assert_eq!(network.positions.len(), 1);
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_dir_valid_multiple() {
        let dir = tempfile::tempdir().unwrap();
        create_minimal_valid_fir(dir.path(), "LOVV");
        create_minimal_valid_fir(dir.path(), "EDMM");

        let (network, errors) = Network::parse_dir(dir.path(), false).unwrap();
        assert_eq!(network.firs.len(), 2);
        assert!(
            network
                .firs
                .contains_key(&FlightInformationRegionId::from("LOVV"))
        );
        assert!(
            network
                .firs
                .contains_key(&FlightInformationRegionId::from("EDMM"))
        );
        assert_eq!(network.stations.len(), 2);
        assert_eq!(network.positions.len(), 2);
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_dir_duplicate_fir_id() {
        let dir = tempfile::tempdir().unwrap();
        create_minimal_valid_fir(dir.path(), "LOVV");
        create_minimal_valid_fir(dir.path(), "lovv");

        let (network, errors) = Network::parse_dir(dir.path(), false).unwrap();
        assert_eq!(network.firs.len(), 1);
        assert_eq!(network.stations.len(), 1);
        assert_eq!(network.positions.len(), 1);
        assert_eq!(errors.len(), 1);
        assert_matches!(&errors[0], CoverageError::Structure(StructureError::Duplicate { entity, .. }) if entity == "FIR");
    }

    #[test]
    fn parse_dir_duplicate_station_id_same_fir() {
        let dir = tempfile::tempdir().unwrap();
        let fir = dir.path().join("LOVV");
        fs::create_dir(&fir).unwrap();
        fs::write(
            fir.join("stations.toml"),
            r#"
            [[stations]]
            id = "LOWW_TWR"
            controlled_by = ["LOWW_TWR"]

            [[stations]]
            id = "LOWW_TWR"
            controlled_by = ["LOWW_TWR"]
        "#,
        )
        .unwrap();
        fs::write(
            fir.join("positions.toml"),
            r#"
            [[positions]]
            id = "LOWW_TWR"
            prefixes = ["LOWW"]
            frequency = "119.400"
            facility_type = "Tower"
        "#,
        )
        .unwrap();

        let (network, errors) = Network::parse_dir(dir.path(), false).unwrap();
        assert_eq!(network.firs.len(), 1);
        assert_eq!(network.stations.len(), 1);
        assert_eq!(network.positions.len(), 1);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| matches!(e, CoverageError::Structure(StructureError::Duplicate { entity, .. }) if entity == "Station")));
    }

    #[test]
    fn parse_dir_duplicate_station_id_different_fir() {
        let dir = tempfile::tempdir().unwrap();
        let fir1 = dir.path().join("LOVV");
        fs::create_dir(&fir1).unwrap();
        fs::write(
            fir1.join("stations.toml"),
            r#"
            [[stations]]
            id = "LOWW_TWR"
            controlled_by = ["LOWW_TWR"]
        "#,
        )
        .unwrap();
        fs::write(
            fir1.join("positions.toml"),
            r#"
            [[positions]]
            id = "LOWW_TWR"
            prefixes = ["LOWW"]
            frequency = "119.400"
            facility_type = "Tower"
        "#,
        )
        .unwrap();

        let fir2 = dir.path().join("EDMM");
        fs::create_dir(&fir2).unwrap();
        fs::write(
            fir2.join("stations.toml"),
            r#"
            [[stations]]
            id = "LOWW_TWR"
            controlled_by = ["EDDM_S_TWR"]
        "#,
        )
        .unwrap();
        fs::write(
            fir2.join("positions.toml"),
            r#"
            [[positions]]
            id = "EDDM_S_TWR"
            prefixes = ["EDDM"]
            frequency = "120.505"
            facility_type = "Tower"
        "#,
        )
        .unwrap();

        let (network, errors) = Network::parse_dir(dir.path(), false).unwrap();
        assert_eq!(network.firs.len(), 2);
        assert_eq!(network.stations.len(), 1);
        assert_eq!(network.positions.len(), 2);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| matches!(e, CoverageError::Structure(StructureError::Duplicate { entity, .. }) if entity == "Station")));
    }

    #[test]
    fn parse_dir_duplicate_position_id_same_fir() {
        let dir = tempfile::tempdir().unwrap();
        let fir = dir.path().join("LOVV");
        fs::create_dir(&fir).unwrap();
        fs::write(
            fir.join("stations.toml"),
            r#"
            [[stations]]
            id = "LOWW_TWR"
            controlled_by = ["LOWW_TWR"]
        "#,
        )
        .unwrap();
        fs::write(
            fir.join("positions.toml"),
            r#"
            [[positions]]
            id = "LOWW_TWR"
            prefixes = ["LOWW"]
            frequency = "119.400"
            facility_type = "Tower"

            [[positions]]
            id = "LOWW_TWR"
            prefixes = ["LOWW"]
            frequency = "119.400"
            facility_type = "Tower"
        "#,
        )
        .unwrap();

        let (network, errors) = Network::parse_dir(dir.path(), false).unwrap();
        assert_eq!(network.firs.len(), 1);
        assert_eq!(network.stations.len(), 1);
        assert_eq!(network.positions.len(), 1);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| matches!(e, CoverageError::Structure(StructureError::Duplicate { entity, .. }) if entity == "Position")));
    }

    #[test]
    fn parse_dir_duplicate_position_id_different_fir() {
        let dir = tempfile::tempdir().unwrap();
        let fir1 = dir.path().join("LOVV");
        fs::create_dir(&fir1).unwrap();
        fs::write(
            fir1.join("stations.toml"),
            r#"
            [[stations]]
            id = "LOWW_TWR"
            controlled_by = ["LOWW_TWR"]
        "#,
        )
        .unwrap();
        fs::write(
            fir1.join("positions.toml"),
            r#"
            [[positions]]
            id = "LOWW_TWR"
            prefixes = ["LOWW"]
            frequency = "119.400"
            facility_type = "Tower"
        "#,
        )
        .unwrap();

        let fir2 = dir.path().join("EDMM");
        fs::create_dir(&fir2).unwrap();
        fs::write(
            fir2.join("stations.toml"),
            r#"
            [[stations]]
            id = "EDDM_S_TWR"
            controlled_by = ["EDDM_S_TWR"]
        "#,
        )
        .unwrap();
        fs::write(
            fir2.join("positions.toml"),
            r#"
            [[positions]]
            id = "LOWW_TWR"
            prefixes = ["LOWW"]
            frequency = "119.400"
            facility_type = "Tower"
        "#,
        )
        .unwrap();

        let (network, errors) = Network::parse_dir(dir.path(), false).unwrap();
        assert_eq!(network.firs.len(), 2);
        assert_eq!(network.stations.len(), 2);
        assert_eq!(network.positions.len(), 1);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| matches!(e, CoverageError::Structure(StructureError::Duplicate { entity, .. }) if entity == "Position")));
    }

    #[test]
    fn parse_dir_empty_coverage() {
        let dir = tempfile::tempdir().unwrap();
        let fir_path = dir.path().join("LOVV");
        fs::create_dir(&fir_path).unwrap();

        fs::write(
            fir_path.join("stations.toml"),
            r#"
             [[stations]]
             id = "LOWW_TWR"
             controlled_by = []
         "#,
        )
        .unwrap();
        fs::write(
            fir_path.join("positions.toml"),
            r#"
            [[positions]]
            id = "LOWW_TWR"
            prefixes = ["LOWW"]
            frequency = "119.400"
            facility_type = "Tower"
         "#,
        )
        .unwrap();

        let (network, errors) = Network::parse_dir(dir.path(), false).unwrap();
        assert_eq!(network.firs.len(), 1);
        assert_eq!(network.stations.len(), 0);
        assert_eq!(network.positions.len(), 1);
        assert_eq!(errors.len(), 1);
        assert_matches!(&errors[0], CoverageError::Structure(StructureError::EmptyCoverage(station)) if station == "LOWW_TWR");
    }

    #[test]
    fn parse_dir_multiple_errors() {
        let dir = tempfile::tempdir().unwrap();

        // FIR 1: Malformed TOML
        let fir1 = dir.path().join("FIR1");
        fs::create_dir(&fir1).unwrap();
        fs::write(fir1.join("stations.toml"), "invalid").unwrap();
        fs::write(fir1.join("positions.toml"), "").unwrap();

        // FIR 2: Duplicate station within same FIR file
        let fir2 = dir.path().join("FIR2");
        fs::create_dir(&fir2).unwrap();
        fs::write(
            fir2.join("stations.toml"),
            r#"
            [[stations]]
            id = "A"
            controlled_by = ["A"]
            
            [[stations]]
            id = "A"
            controlled_by = ["A"]
        "#,
        )
        .unwrap();
        fs::write(
            fir2.join("positions.toml"),
            r#"
            [[positions]]
            id = "B"
            prefixes = ["B"]
            frequency = "120.000"
            facility_type = "Center"
        "#,
        )
        .unwrap();

        let (network, errors) = Network::parse_dir(dir.path(), false).unwrap();
        assert_eq!(network.firs.len(), 1);
        assert_eq!(network.stations.len(), 1);
        assert_eq!(network.positions.len(), 1);
        assert!(errors.len() >= 2);
        assert!(errors.iter().any(|e| matches!(e, CoverageError::Structure(StructureError::Load { entity, id, .. }) if entity == "FIR" && id.contains("FIR1"))));
        assert!(errors.iter().any(|e| matches!(e, CoverageError::Structure(StructureError::Duplicate { entity, id }) if entity == "Station" && id == "A")));
    }

    #[test]
    fn validate_dir() {
        let dir = tempfile::tempdir().unwrap();
        create_minimal_valid_fir(dir.path(), "LOVV");
        create_minimal_valid_fir(dir.path(), "EDMM");

        let (network, errors) = Network::validate_dir(dir.path()).unwrap();
        assert_eq!(network.firs.len(), 2);
        assert!(
            network
                .firs
                .contains_key(&FlightInformationRegionId::from("LOVV"))
        );
        assert!(
            network
                .firs
                .contains_key(&FlightInformationRegionId::from("EDMM"))
        );
        assert_eq!(network.stations.len(), 2);
        assert_eq!(network.positions.len(), 2);
        assert!(errors.is_empty());
    }

    #[test]
    fn validate_dir_error() {
        let dir = tempfile::tempdir().unwrap();

        // FIR 1: Malformed TOML
        let fir1 = dir.path().join("FIR1");
        fs::create_dir(&fir1).unwrap();
        fs::write(fir1.join("stations.toml"), "invalid").unwrap();
        fs::write(fir1.join("positions.toml"), "").unwrap();

        // FIR 2: Duplicate station within same FIR file
        let fir2 = dir.path().join("FIR2");
        fs::create_dir(&fir2).unwrap();
        fs::write(
            fir2.join("stations.toml"),
            r#"
            [[stations]]
            id = "A"
            controlled_by = ["A"]

            [[stations]]
            id = "A"
            controlled_by = ["A"]
        "#,
        )
        .unwrap();
        fs::write(
            fir2.join("positions.toml"),
            r#"
            [[positions]]
            id = "B"
            prefixes = ["B"]
            frequency = "120.000"
            facility_type = "Center"
        "#,
        )
        .unwrap();

        let (network, errors) = Network::validate_dir(dir.path()).unwrap();
        assert_eq!(network.firs.len(), 1);
        assert_eq!(network.stations.len(), 1);
        assert_eq!(network.positions.len(), 1);
        assert!(errors.len() >= 2);
        assert!(errors.iter().any(|e| matches!(e, CoverageError::Structure(StructureError::Load { entity, id, .. }) if entity == "FIR" && id.contains("FIR1"))));
        assert!(errors.iter().any(|e| matches!(e, CoverageError::Structure(StructureError::Duplicate { entity, id }) if entity == "Station" && id == "A")));
    }

    #[test]
    fn load_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        create_minimal_valid_fir(dir.path(), "LOVV");
        create_minimal_valid_fir(dir.path(), "EDMM");

        let network = Network::load_from_dir(dir.path()).expect("should load from dir");
        assert_eq!(network.firs.len(), 2);
        assert!(
            network
                .firs
                .contains_key(&FlightInformationRegionId::from("LOVV"))
        );
        assert!(
            network
                .firs
                .contains_key(&FlightInformationRegionId::from("EDMM"))
        );
        assert_eq!(network.stations.len(), 2);
        assert_eq!(network.positions.len(), 2);
    }

    #[test]
    fn load_from_dir_error() {
        let dir = tempfile::tempdir().unwrap();

        // FIR 1: Malformed TOML
        let fir1 = dir.path().join("FIR1");
        fs::create_dir(&fir1).unwrap();
        fs::write(fir1.join("stations.toml"), "invalid").unwrap();
        fs::write(fir1.join("positions.toml"), "").unwrap();

        // FIR 2: Duplicate station within same FIR file
        let fir2 = dir.path().join("FIR2");
        fs::create_dir(&fir2).unwrap();
        fs::write(
            fir2.join("stations.toml"),
            r#"
            [[stations]]
            id = "A"
            controlled_by = ["A"]

            [[stations]]
            id = "A"
            controlled_by = ["A"]
        "#,
        )
        .unwrap();
        fs::write(
            fir2.join("positions.toml"),
            r#"
            [[positions]]
            id = "B"
            prefixes = ["B"]
            frequency = "120.000"
            facility_type = "Center"
        "#,
        )
        .unwrap();

        let err = Network::load_from_dir(dir.path()).expect_err("should not load from dir");
        assert_matches!(err, CoverageError::Structure(StructureError::Load { entity, id, .. }) if entity == "FIR" && id.contains("FIR1"));
    }

    #[test]
    fn find_positions_callsign_match() {
        let dir = tempfile::tempdir().unwrap();
        create_minimal_valid_fir(dir.path(), "LOVV");
        create_minimal_valid_fir(dir.path(), "EDMM");
        let network = Network::load_from_dir(dir.path()).unwrap();

        let positions = network.find_positions("LOVV_CTR", "199.998", FacilityType::Enroute);
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].id.as_str(), "LOVV_CTR");
    }

    #[test]
    fn find_positions_relief_callsign_match() {
        let dir = tempfile::tempdir().unwrap();
        create_minimal_valid_fir(dir.path(), "LOVV");
        create_minimal_valid_fir(dir.path(), "EDMM");
        let network = Network::load_from_dir(dir.path()).unwrap();

        let positions = network.find_positions("LOVV__CTR", "199.998", FacilityType::Enroute);
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].id.as_str(), "LOVV_CTR");
    }

    #[test]
    fn find_positions_prefix_match() {
        let dir = tempfile::tempdir().unwrap();
        let fir1 = dir.path().join("ENOR");
        fs::create_dir(&fir1).unwrap();
        fs::write(
            fir1.join("stations.toml"),
            r#"
            [[stations]]
            id = "ENCN_TWR"
            controlled_by = ["ENCN_TWR"]

            [[stations]]
            id = "ENDU_TWR"
            controlled_by = ["ENDU_TWR"]

            [[stations]]
            id = "ENAL_TWR"
            controlled_by = ["ENAL_TWR"]

            [[stations]]
            id = "ENBO_TWR"
            controlled_by = ["ENBO_TWR"]

            [[stations]]
            id = "ENKR_TWR"
            controlled_by = ["ENKR_TWR"]
        "#,
        )
        .unwrap();
        fs::write(
            fir1.join("positions.toml"),
            r#"
            [[positions]]
            id = "ENKR_TWR"
            prefixes = ["ENKR"]
            frequency = "118.105"
            facility_type = "TWR"

            [[positions]]
            id = "ENCN_TWR"
            prefixes = ["ENCN"]
            frequency = "118.105"
            facility_type = "TWR"

            [[positions]]
            id = "ENAL_TWR"
            prefixes = ["ENAL"]
            frequency = "118.105"
            facility_type = "TWR"

            [[positions]]
            id = "ENBO_TWR"
            prefixes = ["ENBO"]
            frequency = "118.105"
            facility_type = "TWR"

            [[positions]]
            id = "ENDU_TWR"
            prefixes = ["ENDU"]
            frequency = "118.105"
            facility_type = "TWR"
        "#,
        )
        .unwrap();

        let fir2 = dir.path().join("EBBU");
        fs::create_dir(&fir2).unwrap();
        fs::write(
            fir2.join("stations.toml"),
            r#"
            [[stations]]
            id = "ELLX_TWR"
            controlled_by = ["ELLX_TWR"]
        "#,
        )
        .unwrap();
        fs::write(
            fir2.join("positions.toml"),
            r#"
            [[positions]]
            id = "ELLX_TWR"
            prefixes = ["ELLX"]
            frequency = "118.105"
            facility_type = "TWR"
        "#,
        )
        .unwrap();

        let network = Network::load_from_dir(dir.path()).unwrap();

        let positions = network.find_positions("ENBO_X_TWR", "118.105", FacilityType::Tower);
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].id.as_str(), "ENBO_TWR");
    }

    #[test]
    fn find_positions_different_frequency() {
        let dir = tempfile::tempdir().unwrap();
        create_minimal_valid_fir(dir.path(), "LOVV");
        create_minimal_valid_fir(dir.path(), "EDMM");
        let network = Network::load_from_dir(dir.path()).unwrap();

        let positions = network.find_positions("LOVV_CTR", "121.500", FacilityType::Enroute);
        assert_eq!(positions.len(), 0);
    }

    #[test]
    fn find_positions_different_facility_type() {
        let dir = tempfile::tempdir().unwrap();
        create_minimal_valid_fir(dir.path(), "LOVV");
        create_minimal_valid_fir(dir.path(), "EDMM");
        let network = Network::load_from_dir(dir.path()).unwrap();

        let positions = network.find_positions("LOVV_CTR", "199.998", FacilityType::TrafficFlow);
        assert_eq!(positions.len(), 0);
    }

    #[test]
    fn find_positions_different_prefixes() {
        let dir = tempfile::tempdir().unwrap();
        create_minimal_valid_fir(dir.path(), "EDMM");
        let network = Network::load_from_dir(dir.path()).unwrap();

        let positions = network.find_positions("LOVV_CTR", "199.998", FacilityType::Enroute);
        assert_eq!(positions.len(), 0);
    }

    #[test]
    fn find_positions_multiple_matches() {
        let dir = tempfile::tempdir().unwrap();
        let fir = dir.path().join("LOVV");
        fs::create_dir(&fir).unwrap();
        fs::write(
            fir.join("stations.toml"),
            r#"
            [[stations]]
            id = "LOWI_E_APP"
            controlled_by = ["LOWI_E_APP"]

            [[stations]]
            id = "LOWI_S_APP"
            controlled_by = ["LOWI_S_APP"]
        "#,
        )
        .unwrap();
        fs::write(
            fir.join("positions.toml"),
            r#"
            [[positions]]
            id = "LOWI_S_APP"
            prefixes = ["LOWI"]
            frequency = "128.975"
            facility_type = "APP"

            [[positions]]
            id = "LOWI_E_APP"
            prefixes = ["LOWI"]
            frequency = "128.975"
            facility_type = "Approach"
        "#,
        )
        .unwrap();
        let network = Network::load_from_dir(dir.path()).unwrap();

        let positions = network.find_positions("LOWI_X_APP", "128.975", FacilityType::Approach);
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].id.as_str(), "LOWI_E_APP");
        assert_eq!(positions[1].id.as_str(), "LOWI_S_APP");
    }
}
