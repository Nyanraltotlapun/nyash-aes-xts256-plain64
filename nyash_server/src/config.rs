

use std::{ops::Not, path::Path, u64, u128};

use serde::Deserialize;

const DB_DIR: &str = "/var/lib/nyash-aes-xts256-plain64/";

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub listen_port: u16,
    pub db_dir: Option<String>,
    //keys: Keys,
}

// #[derive(Deserialize, Debug)]
// struct Keys {
//     github: String,
//     travis: Option<String>,
// }


pub fn read_config(file_path: &Path) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let config_str = std::fs::read_to_string(file_path)?;
    let mut serer_config: ServerConfig = toml::from_str(&config_str)?;
    serer_config.db_dir = serer_config.db_dir.or_else(|| Some(DB_DIR.to_string()));
    println!("IP: {}", serer_config.bind_addr);
    println!("Listen port: {:?}", serer_config.listen_port);
    Ok(serer_config)
}