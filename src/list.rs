use windows::core::Error;
use crate::find::device_iterate_file_or_folder;
use crate::path::DeviceStoragePath;
use crate::wpd::device::{ContentObjectInfo, Device};
use crate::wpd::manager::{DeviceInfo, Manager};


// 列出设备
pub fn list_devices(manager: &Manager, pattern: Option<&str>) -> Result<Vec<DeviceInfo>, Error> {

    let mut devices = Vec::<DeviceInfo>::new();

    let mut iter = manager.get_device_iterator()?;
    while let Some(device_info) = iter.next()? {
        devices.push(device_info);
    }
    Ok(devices)
}

// 获取设备对象
fn get_device_object(device: &Device) -> Result<Option<ContentObjectInfo>, Box<dyn std::error::Error>> {
    let root = device.get_root_object();
    match device.get_object_iterator(&root) {
        Err(err) => {
            log::debug!("{}", err);
            log::warn!("failed to get the device object: {}", &device.name);
        }
        Ok(mut iter) => {
            while let Some(obj) = iter.next()? {
                log::trace!("  detected device root entry {:?}", &obj);
                let info = device.get_object_info(obj)?;
                if info.is_device() {
                    log::trace!("   --> device object found");
                    return Ok(Some(info));
                }
            }
        }
    }
    Ok(None)
}

// 列出某个设备的存储对象
pub fn list_device_storages(device: &Device, pattern: Option<&str>) -> Result<Vec<ContentObjectInfo>, Box<dyn std::error::Error>> {
    log::trace!("device_find_storage_objects pattern={:?}", &pattern);

    let mut objects = Vec::<ContentObjectInfo>::new();

    let device_obj_info = match get_device_object(device)? {
        Some(info) => info,
        None => return Ok(objects),
    };

    match device.get_object_iterator(&device_obj_info.content_object) {
        Err(err) => {
            log::debug!("{}", err);
            log::warn!("failed to open device: {}", &device_obj_info.name);
        }
        Ok(mut iter) => {
            while let Some(obj) = iter.next()? {
                log::trace!("  detected device object entry {:?}", &obj);
                let info = device.get_object_info(obj)?;
                log::trace!("   details {:?}", &info);
                if info.is_storage(){
                    log::trace!("   --> storage object found");
                    objects.push(info);
                }
            }
        }
    }
    Ok(objects)
}

// 列出所有设备的storages
pub fn list_storages() -> Result<(), Box<dyn std::error::Error>> {
    log::trace!("COMMAND list-storages");

    let manager = Manager::get_portable_device_manager().unwrap();
    let device_info_vec = list_devices(&manager, None)?;

    let mut count = 0;
    for device_info in device_info_vec {
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
    if count == 0 {
        println!("no storages were found.")
    }
    Ok(())
}

// 列出文件 path: Redmi K70:内部存储设备:/Pictures,recurse是否递归, detail是否显示详细信息
pub fn list_files(path: String, recursive: bool, detail: bool) -> Result<(), Box<dyn std::error::Error>> {

    let storage_path = DeviceStoragePath::from(&path)?;

    let manager = Manager::get_portable_device_manager()?;
    let device_info_vec = list_devices(&manager, Some(&storage_path.device_name))?;

    if device_info_vec.len() == 0 {
        return Err("No device matched.".into());
    }

    for device_info in device_info_vec {
        let device = Device::open(&device_info)?;
        let storage_object_vec = list_device_storages(&device, Some(&storage_path.storage_name))?;

        let callback = if detail{
            show_file_or_folder_with_details
        } else {
            show_file_or_folder_path_only
        };

        for storage_object_info in storage_object_vec {
            device_iterate_file_or_folder(
                &device,
                &device_info,
                &storage_object_info,
                &storage_path.path,
                recursive,
                callback,
            )?;
        }
    }
    Ok(())
}


fn show_file_or_folder_with_details(info: &ContentObjectInfo, path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "[{:<4}] {:<19} {:<19} {}",
        if info.is_file() {
            "FILE"
        } else if info.is_folder() {
            "DIR"
        } else {
            ""
        },
        if info.is_system { "S" } else { "-" },
        if info.is_hidden { "H" } else { "-" },
        path
    );
    Ok(true)
}

fn show_file_or_folder_path_only(_info: &ContentObjectInfo, path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    println!("{}", path);
    Ok(true)
}