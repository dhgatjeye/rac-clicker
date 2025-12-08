use crate::config::{ServerRegistry, Settings};
use crate::core::RacResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigProfile {
    pub settings: Settings,
    pub server_registry: ServerRegistry,
}

impl ConfigProfile {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_target_process(&self) -> RacResult<String> {
        let server = self.server_registry.get_active()?;
        Ok(server.process_name.clone())
    }

    pub fn get_left_cps(&self) -> RacResult<u8> {
        if self.settings.left_cps_override > 0 {
            Ok(self.settings.left_cps_override)
        } else {
            let server = self.server_registry.get_active()?;
            Ok(server.left_click.max_cps)
        }
    }

    pub fn get_right_cps(&self) -> RacResult<u8> {
        if self.settings.right_cps_override > 0 {
            Ok(self.settings.right_cps_override)
        } else {
            let server = self.server_registry.get_active()?;
            Ok(server.right_click.max_cps)
        }
    }

    pub fn switch_server(&mut self, server_type: crate::core::ServerType) -> RacResult<()> {
        self.server_registry.set_active(server_type)?;
        self.settings.active_server = server_type;
        Ok(())
    }
}
