mod find;
mod list;
mod wpd;
pub mod path;
mod common;
pub mod copy_operate;
pub mod copy;

use std::error::Error;
use clap::{Parser, Subcommand};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use crate::list::{list_files, list_storages};

#[derive(Debug)]
pub struct Paths {
    src: String,
    dest: String,
}
#[derive(Subcommand)]
enum Commands {
    #[clap(about = "List all device's storages ")]
    ListStorages {
    },
    #[clap(about = "List files in a storage")]
    ListFiles {
        #[clap(value_parser, help ="The path to list files, e.g. \"<device>:<storage>:<path>\"")]
        path: String, //必填
        #[clap(short = 'r', long, help ="List files recursively")]
        recursive: bool,
        #[clap(short = 'd', long,help ="Show file details")]
        detail: bool,
    },
    #[clap(about = "Copy files from source to destination")]
    Copy {
        #[clap(value_parser,help ="The source path to copy from, e.g. \"<device>:<storage>:<path>\"")]
        src: String,
        #[clap(value_parser,help ="The destination path to copy to, e.g. \"<device>:<storage>:<path>\"")]
        dest: String,
        #[clap(short = 'r', long,help ="Copy files recursively")]
        recursive: bool,
        #[clap(short = 'm', long,help ="Mirror the source to the destination")]
        mirror: bool,
    },
}

#[derive(Parser)]
#[command(name = "mtp_util")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();
    match &cli.command {
        Commands::ListStorages { } => {
            unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
            match list_storages() {
                Ok(_) => {}
                Err(err) => {
                    println!("Error: {}", err);
                }
            }
        }
        Commands::ListFiles { path, recursive, detail } => {
            unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
            match list_files(path.clone(),*recursive,*detail) {
                Ok(_) => {}
                Err(err) => {
                    println!("Error: {}", err);
                }
            }
        }
        Commands::Copy { src, dest, recursive, mirror } => {
            unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
            let paths = Paths {
                src: src.clone(),
                dest: dest.clone(),
            };
            match copy::copy(&paths,  *recursive, *mirror) {
                Ok(_) => {
                    println!("Copy successfully.");
                }
                Err(err) => {
                    println!("Error: {}", err);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::{core::Result, Win32::System::Threading::*,Win32::Devices::PortableDevices::*,Win32::System::Com::*};
    use crate::find::{find_storage, find_file_or_folder};
    use crate::list::{list_devices, list_device_storages, list_storages, list_files};
    use crate::wpd::device::Device;
    use crate::wpd::manager::{DeviceInfo, Manager};

    static COUNTER: std::sync::RwLock<i32> = std::sync::RwLock::new(0);
    #[test]
    fn test_win_threading_pool() {
        unsafe {
            let work = CreateThreadpoolWork(Some(callback), None, None).unwrap();

            for _ in 0..10 {
                SubmitThreadpoolWork(work);
            }

            WaitForThreadpoolWorkCallbacks(work, false);
        }

        let counter = COUNTER.read().unwrap();
        println!("counter: {}", *counter);
    }

    extern "system" fn callback(_: PTP_CALLBACK_INSTANCE, _: *mut std::ffi::c_void, _: PTP_WORK) {
        let mut counter = COUNTER.write().unwrap();
        *counter += 1;
    }


    #[test]
    fn test_portable_devices() {
        unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
        let manager = Manager::get_portable_device_manager().unwrap();
        let vec = list_devices(&manager, None).unwrap();
        let mut count = 0;
        for device_info in vec {
            match Device::open(&device_info) {
                Err(err) => {
                    log::debug!("{}", err);
                    log::warn!("failed to open \"{}\" (skipped)", device_info.name);
                }
                Ok(device) => match list_device_storages(&device, None) {
                    Err(err) => {
                        log::debug!("{}", err);
                        log::warn!(
                        "failed to get storages from \"{}\" (skipped)",
                        device_info.name
                    );
                    }
                    Ok(storage_object_vec) => {
                        for storage_object_info in storage_object_vec {
                            count += 1;
                            println!("{}:{}:", &device_info.name, &storage_object_info.name);
                        }
                    }
                },
            }
        }
    }
    #[test]
    fn test_get_storage() {
        unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
        let manager = Manager::get_portable_device_manager().unwrap();
        let storage_path = path::DeviceStoragePath::from("Redmi K70:内部存储设备:\\").unwrap();
        let option = find_storage(&manager, &storage_path).unwrap();
        let (device_info, device, storage_object) = option.unwrap();
        println!("device_info: {:?}", device_info);
        println!("storage_object: {:?}", storage_object);
    }
    #[test]
    fn test_find_file_or_folder() {
        unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
        let manager = Manager::get_portable_device_manager().unwrap();
        let storage_path = path::DeviceStoragePath::from("Redmi K70:内部存储设备:/Pictures").unwrap();
        let option = find_file_or_folder(&manager, &storage_path).unwrap();
        let (device_info, device, content_object_info) = option.unwrap();
        println!("device_info: {:?}", device_info);
        println!("storage_object: {:?}", content_object_info);
    }
    #[test]
    fn test_list_storages() {
        unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
        list_storages();
    }

    #[test]
    fn test_list_files() {
        unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
        list_files("Redmi K70:内部存储设备:/Pictures".to_string(), true, true).unwrap();
    }
}