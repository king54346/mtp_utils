use std::path::PathBuf;
use crate::copy_operate::device_folder_imp::DeviceFolder;
use crate::copy_operate::{do_copy, get_destination_path_info, has_wildcard, inspect_path};
use crate::copy_operate::local_folder_imp::LocalFolder;
use crate::find::find_file_or_folder;
use crate::path::{DeviceStoragePath, get_path_type, PathType};
use crate::Paths;
use crate::wpd::manager::Manager;



pub fn copy(
    paths: &Paths,
    recursive: bool,
    mirror: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    log::trace!("command_copy paths={:?}", paths);
    let manager = Manager::get_portable_device_manager()?;

    let src_path = paths.src.as_str();
    let dest_path = paths.dest.as_str();

    // 1. 获取源路径和目标路径类型
    let src_path_type = get_path_type(src_path);
    let dest_path_type = get_path_type(dest_path);

    // 2. 检查路径是否包含通配符，不支持通配符
    for (path, path_type) in [(src_path, src_path_type), (dest_path, dest_path_type)] {
        if has_wildcard(path, path_type)? {
            return Err(format!("Wildcard characters in the {} path are not allowed.",
                               if path == src_path { "source" } else { "destination" }).into());
        }
    }

    // 3. 检查目标路径状态
    let dest_inspection = inspect_path(&manager, dest_path, dest_path_type)?;
    log::trace!("dest_inspection = {:?}", &dest_inspection);

    // 判断目标路径是否是父文件夹
    let (dest_base_path, dest_name) = match get_destination_path_info(&dest_inspection, dest_path)? {
        Some(info) => info,
        None => return Err("cannot create the destination path.".into()),
    };

    // 处理不同路径类型的复制逻辑
    match dest_path_type {
        // 复制到设备存储
        PathType::DeviceStorage => {
            let storage_path = DeviceStoragePath::from(dest_base_path)?;
            if let Some((_, device, object_info)) = find_file_or_folder(&manager, &storage_path)? {
                let mut destination_folder = DeviceFolder::new(&device, object_info)?;
                do_copy(
                    &manager,
                    src_path,
                    src_path_type,
                    &mut destination_folder,
                    dest_name.is_none(),
                    dest_name,
                    recursive,
                    mirror,
                )
            }else {
                Err("failed to open source path.".into())
            }
        }
        // 复制到本地
        PathType::Local => {
            let mut destination_folder = LocalFolder::new(PathBuf::from(dest_base_path));
            do_copy(
                &manager,
                src_path,
                src_path_type,
                &mut destination_folder,
                !dest_name.is_none(),
                dest_name,
                recursive,
                mirror,
            )
        },
        PathType::Invalid => Err("invalid destination path.".into()),
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::PathType;
    use crate::Paths;
    use std::error::Error;
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

    #[test]
    fn command_copy_device_to_local() -> Result<(), Box<dyn Error>> {
        let paths = Paths {
            src: "device:/test_data/file.txt".to_string(),
            dest: "dest/test_data/file.txt".to_string(),
        };
        let result = copy(&paths, false, false);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn command_copy_local_to_device() -> Result<(), Box<dyn Error>> {
        let paths = Paths {
            src: "C:\\Users\\admin\\java_error_in_gateway64_20100.log".to_string(),
            dest: "Redmi K70:内部存储设备:/Pictures/file.txt".to_string(),
        };
        unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
        let result = copy(&paths, false, false);
        assert!(result.is_ok());
        Ok(())
    }
}