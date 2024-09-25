mod find;
mod list;
mod wpd;

pub mod path;
mod common;
mod copy;
mod command;

#[derive(Debug)]
pub struct Paths {
    src: String,
    dest: String,
}


fn main() {
    env_logger::init();

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