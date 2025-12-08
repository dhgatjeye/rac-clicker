use crate::core::{ClickPattern, RacResult, ServerType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server_type: ServerType,
    pub process_name: String,
    pub left_click: ClickPattern,
    pub right_click: ClickPattern,
    pub description: String,
}

impl ServerConfig {
    pub fn craftrise() -> Self {
        Self {
            server_type: ServerType::Craftrise,
            process_name: "craftrise-x64.exe".to_string(),
            left_click: ClickPattern::from_cps(15),
            right_click: ClickPattern::from_cps(18),
            description: "Craftrise server optimized".to_string(),
        }
    }

    pub fn sonoyuncu() -> Self {
        Self {
            server_type: ServerType::Sonoyuncu,
            process_name: "javaw.exe".to_string(),
            left_click: ClickPattern::from_cps(15),
            right_click: ClickPattern::from_cps(18),
            description: "Sonoyuncu server optimized settings".to_string(),
        }
    }

    pub fn custom(process_name: String) -> Self {
        Self {
            server_type: ServerType::Custom,
            process_name,
            left_click: ClickPattern::default(),
            right_click: ClickPattern::from_cps(18),
            description: "Custom server configuration".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerRegistry {
    configs: HashMap<ServerType, ServerConfig>,
    active_server: ServerType,
}

impl Default for ServerRegistry {
    fn default() -> Self {
        let mut configs = HashMap::new();
        configs.insert(ServerType::Craftrise, ServerConfig::craftrise());
        configs.insert(ServerType::Sonoyuncu, ServerConfig::sonoyuncu());

        Self {
            configs,
            active_server: ServerType::Craftrise,
        }
    }
}

impl ServerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_active(&self) -> RacResult<&ServerConfig> {
        self.configs.get(&self.active_server).ok_or_else(|| {
            crate::core::RacError::ConfigError(format!(
                "Active server {:?} not found in registry",
                self.active_server
            ))
        })
    }

    pub fn get_active_mut(&mut self) -> RacResult<&mut ServerConfig> {
        self.configs.get_mut(&self.active_server).ok_or_else(|| {
            crate::core::RacError::ConfigError(format!(
                "Active server {:?} not found in registry",
                self.active_server
            ))
        })
    }

    pub fn get_server(&self, server_type: ServerType) -> Option<&ServerConfig> {
        self.configs.get(&server_type)
    }

    pub fn set_active(&mut self, server_type: ServerType) -> RacResult<()> {
        if !self.configs.contains_key(&server_type) {
            return Err(crate::core::RacError::ConfigError(format!(
                "Server {:?} not found in registry",
                server_type
            )));
        }
        self.active_server = server_type;
        Ok(())
    }

    pub fn register_server(&mut self, config: ServerConfig) {
        self.configs.insert(config.server_type, config);
    }

    pub fn active_server_type(&self) -> ServerType {
        self.active_server
    }

    pub fn list_servers(&self) -> Vec<ServerType> {
        self.configs.keys().copied().collect()
    }
}
