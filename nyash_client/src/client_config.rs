use ocl::{Device, Platform};
use serde::{Deserialize, Serialize};
use serde_json;
use std::error::Error;

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct DevConf {
    pub dev_name: String,
    pub platform_name: String,
    pub id: usize,
    pub work_size: usize,
    pub batch_size: u32,
}

impl DevConf {
    // Constructor with parameters
    pub fn from_cl_dev(dev_pl: (Device, Platform), id: usize) -> Self {
        Self {
            dev_name: dev_pl.0.name().unwrap_or("Noname".to_string()),
            platform_name: dev_pl.1.name().unwrap_or("Noname".to_string()),
            id: id,
            work_size: 0,
            batch_size: 0,
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct AppConfig {
    pub devices: Vec<DevConf>,
    pub dev_fill: u8,
}

impl AppConfig {
    // Constructor with parameters
    pub fn from_dev_list(all_devices: &Vec<(Device, Platform)>, devs_nums: Vec<usize>) -> Self {
        Self {
            devices: devs_nums
                .iter()
                .map(|id| DevConf::from_cl_dev(all_devices[*id], *id))
                .collect(),
            dev_fill: 100,
        }
    }

    pub fn device_name_exist(&self, dev_name: &str) -> bool {
        match self.devices.iter().find(|d| d.dev_name == dev_name) {
            Some(_) => true,
            None => false,
        }
    }

    pub fn device_exist(&self, dev: &Device) -> bool {
        match dev.name() {
            Ok(dev_name) => match self.devices.iter().find(|d| d.dev_name == dev_name) {
                Some(_) => true,
                None => false,
            },
            Err(_) => false,
        }
    }
}

pub fn load_config(file_name: &str) -> Result<AppConfig, Box<dyn Error>> {
    let file_data = std::fs::read_to_string(file_name)?;
    let app_conf: AppConfig = serde_json::from_str(file_data.as_str())?;
    return Ok(app_conf);
}

pub fn save_config(file_name: &str, app_conf: &AppConfig) -> Result<(), Box<dyn Error>> {
    let conf_str = serde_json::to_string(app_conf)?;
    std::fs::write(file_name, conf_str)?;
    return Ok(());
}
