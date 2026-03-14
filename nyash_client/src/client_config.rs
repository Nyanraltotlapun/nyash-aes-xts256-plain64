use ocl::{Device, Platform, DeviceType, flags};
use serde::{Deserialize, Serialize};
use serde_json;
use std::error::Error;
use std::{io};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct DevConf {
    pub dev_name: String,
    pub platform_name: String,
    pub id: usize,
    pub work_size: usize,
    pub batch_size: u64,
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
    let file_path = std::path::Path::new(file_name);
    let file_data = std::fs::read_to_string(file_path)?;
    let app_conf: AppConfig = serde_json::from_str(file_data.as_str())?;
    return Ok(app_conf);
}

pub fn save_config(file_name: &str, app_conf: &AppConfig) -> Result<(), Box<dyn Error>> {
    let file_path = std::path::Path::new(file_name);
    let conf_str = serde_json::to_string_pretty(app_conf)?;
    std::fs::write(file_path, conf_str)?;
    return Ok(());
}

fn dev_type_from_str(s: &str) -> Result<flags::DeviceType, ()> {
    match s {
        "CPU" => Ok(flags::DeviceType::CPU),
        "GPU" => Ok(flags::DeviceType::GPU),
        "ALL" => Ok(flags::DeviceType::ALL),
        "CUSTOM" => Ok(flags::DeviceType::CUSTOM),
        "ACCELERATOR" => Ok(flags::DeviceType::ACCELERATOR),
        "DEFAULT" => Ok(flags::DeviceType::DEFAULT),
        _ => Err(()),
    }
}

fn str_or_empty(r: ocl::error::Result<String>) -> String {
    match r {
        Ok(s) => s,
        Err(_) => "".to_string(),
    }
}

fn print_devices(dev_list: &Vec<(Device, Platform)>) {
    let mut i = 0;
    for (dev, plt) in dev_list.iter() {
        let dev_name = str_or_empty(dev.name());
        let plt_name = str_or_empty(plt.name());
        println!("({i}) device: \"{dev_name}\" ----- platorm: \"{plt_name}\"");
        i += 1;
    }
}

fn choose_devices(devices_num: usize) -> Result<Vec<usize>, String> {
    println!("Please input desired device to use as a number and press Enter.");
    let mut result: Vec<usize> = Vec::new();

    let mut s_devs_nums = String::new();

    io::stdin()
        .read_line(&mut s_devs_nums)
        .expect("Failed to read line");

    for s_dev_num in s_devs_nums.split(' ') {
        let dev_num: usize = match s_dev_num.trim().parse() {
            Ok(num) => num,
            Err(_) => return Err("You must input a number from device list.".to_string()),
        };
        if dev_num >= devices_num {
            return Err("You must input a number from device list.".to_string());
        };
        result.push(dev_num);
    }
    return Ok(result);
}

fn list_devices(dev_type: DeviceType) -> Vec<(Device, Platform)> {
    let platforms = Platform::list();
    let mut devices: Vec<(Device, Platform)> = Vec::new();
    for plt in platforms.iter() {
        //let plat_name = str_or_empty(plt.name());
        let list_res = Device::list(plt, Some(dev_type));
        match list_res {
            Ok(dev_l) => devices.extend(dev_l.iter().map(|dev| (*dev, plt.clone()))),
            Err(_) => ()
        }
    }
    return devices;
}


fn dev_sel_dialog(all_devices: &Vec<(Device, Platform)>) -> Vec<usize> {
    let devs_nums = loop {
        print_devices(&all_devices);
        match choose_devices(all_devices.len()) {
            Ok(value) => break value,
            Err(exc) => {
                println!("Error! {exc}\n")
            }
        }
    };
    return devs_nums;
}

pub fn get_devices_conf(file_name: &str) -> Result<(Vec<(Device, Platform)>, AppConfig), String> {
    let dev_type = dev_type_from_str("GPU").expect("Unexpected device type!");

    // Get devices to be used for key search
    let mut all_devices: Vec<(Device, Platform)> = list_devices(dev_type);
    if all_devices.len() == 0 {
        println!("Cannot detect GPU devices. Will try to list all!");
        all_devices = list_devices(dev_type_from_str("ALL").expect("Unexpected device type!"));
    }
    
    if all_devices.len() == 0 {
        return Err("Cannot find any usable devices.".to_string());
    };

    let app_conf = match load_config(file_name) {
        Ok(readed_config) => {
            let dev_found = all_devices
                .iter()
                .filter(|dp| readed_config.device_exist(&dp.0))
                .count();
            if dev_found < readed_config.devices.len() {
                println!("Devices from config not found in the system!");
                let devs_nums = dev_sel_dialog(&all_devices);
                let res = AppConfig::from_dev_list(&all_devices, devs_nums);
                save_config(file_name, &res).expect("Error saving config!");
                res
            } else {
                readed_config
            }
        }
        Err(_) => {
            println!("Cannot find config file {}", file_name);
            let devs_nums = dev_sel_dialog(&all_devices);
            let res = AppConfig::from_dev_list(&all_devices, devs_nums);
            save_config(file_name, &res).expect("Error saving config!");
            res
        }
    };

    let selected_devs = all_devices
        .iter()
        .filter(|dp| app_conf.device_exist(&dp.0))
        .cloned()
        .collect();

    return Ok((selected_devs, app_conf));
}