use crate::data_feed::DataFeed;
use crate::{ControllerInfo, FacilityType};
use async_trait::async_trait;

#[derive(Debug)]
pub struct MockDataFeed {
    should_error: bool,
    controllers: Vec<ControllerInfo>,
}

impl MockDataFeed {
    pub fn new(controllers: Vec<ControllerInfo>) -> Self {
        Self {
            should_error: false,
            controllers,
        }
    }

    pub fn add(&mut self, controller: ControllerInfo) {
        self.controllers.push(controller);
    }

    pub fn remove(&mut self, cid: &str) {
        self.controllers.retain(|c| c.cid != cid);
    }

    pub fn clear(&mut self) {
        self.controllers.clear();
    }

    pub fn set_error(&mut self, should_error: bool) {
        self.should_error = should_error;
    }
}

impl Default for MockDataFeed {
    fn default() -> Self {
        Self::new(vec![ControllerInfo {
            cid: "client1".to_string(),
            callsign: "client1".to_string(),
            frequency: "100.000".to_string(),
            facility_type: FacilityType::Enroute,
        }])
    }
}

#[async_trait]
impl DataFeed for MockDataFeed {
    async fn fetch_controller_info(&self) -> anyhow::Result<Vec<ControllerInfo>> {
        if self.should_error {
            return Err(anyhow::anyhow!("Mock error"));
        }
        Ok(self.controllers.clone())
    }
}
