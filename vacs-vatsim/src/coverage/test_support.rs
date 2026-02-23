use super::network::Network;

/// Builder for creating test FIR directories with stations, positions, and profiles.
///
/// Writes TOML files to a temporary directory that can be loaded via [`Network::load_from_dir`].
pub struct TestFirBuilder {
    name: String,
    stations: Vec<String>,
    positions: Vec<String>,
    profiles: Vec<(String, String)>,
}

impl TestFirBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            stations: Vec::new(),
            positions: Vec::new(),
            profiles: Vec::new(),
        }
    }

    pub fn station(mut self, id: &str, controlled_by: &[&str]) -> Self {
        self.stations.push(format!(
            r#"
[[stations]]
id = "{id}"
controlled_by = {controlled_by:?}
"#
        ));
        self
    }

    pub fn station_with_parent(
        mut self,
        id: &str,
        parent_id: &str,
        controlled_by: &[&str],
    ) -> Self {
        self.stations.push(format!(
            r#"
[[stations]]
id = "{id}"
parent_id = "{parent_id}"
controlled_by = {controlled_by:?}
"#
        ));
        self
    }

    pub fn position(
        mut self,
        id: &str,
        prefixes: &[&str],
        frequency: &str,
        facility_type: &str,
    ) -> Self {
        self.positions.push(format!(
            r#"
[[positions]]
id = "{id}"
prefixes = {prefixes:?}
frequency = "{frequency}"
facility_type = "{facility_type}"
"#
        ));
        self
    }

    pub fn position_with_profile(
        mut self,
        id: &str,
        prefixes: &[&str],
        frequency: &str,
        facility_type: &str,
        profile_id: &str,
    ) -> Self {
        self.positions.push(format!(
            r#"
[[positions]]
id = "{id}"
prefixes = {prefixes:?}
frequency = "{frequency}"
facility_type = "{facility_type}"
profile_id = "{profile_id}"
"#
        ));
        self
    }

    /// Add a tabbed profile with the given station keys.
    ///
    /// Each entry in `station_keys` is a `(label, station_id)` pair.
    pub fn tabbed_profile(mut self, id: &str, station_keys: &[(&str, &str)]) -> Self {
        let keys: String = station_keys
            .iter()
            .map(|(label, station_id)| {
                format!(
                    r#"
[[tabs.page.keys]]
label = "{label}"
station_id = "{station_id}"
"#
                )
            })
            .collect::<Vec<_>>()
            .join("");

        let content = format!(
            r#"
id = "{id}"
type = "Tabbed"

[[tabs]]
label = "Main"

[tabs.page]
rows = 4
{keys}"#
        );

        self.profiles.push((id.to_string(), content));
        self
    }

    pub fn create(self, dir: &std::path::Path) {
        let fir_path = dir.join(&self.name);
        if !fir_path.exists() {
            std::fs::create_dir(&fir_path).unwrap();
        }

        if !self.stations.is_empty() {
            std::fs::write(fir_path.join("stations.toml"), self.stations.join("\n")).unwrap();
        }

        if !self.positions.is_empty() {
            std::fs::write(fir_path.join("positions.toml"), self.positions.join("\n")).unwrap();
        }

        if !self.profiles.is_empty() {
            let profiles_dir = fir_path.join("profiles");
            std::fs::create_dir_all(&profiles_dir).unwrap();
            for (id, content) in &self.profiles {
                std::fs::write(profiles_dir.join(format!("{id}.toml")), content).unwrap();
            }
        }
    }

    /// Create the FIR directory and load it as a [`Network`].
    pub fn build(self, dir: &std::path::Path) -> Network {
        self.create(dir);
        Network::load_from_dir(dir).unwrap()
    }
}
