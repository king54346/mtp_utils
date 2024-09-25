use std::path::{Path, PathBuf};
use crate::copy::copy_processor::CopyProcessor;
use crate::copy::device_copy_processor::DeviceCopyProcessor;
use crate::copy::device_folder_imp::DeviceFolder;
use crate::copy::file_info::FileInfo;
use crate::copy::folder_operate::FolderOperate;
use crate::copy::local_copy_processor::LocalCopyProcessor;
use crate::copy::local_folder_imp::LocalFolder;
use crate::find::{find_file_or_folder};
use crate::path::{DeviceStoragePath, get_path_type, PathType, SEPARATORS, WILDCARD_CHARACTERS};
use crate::Paths;
use crate::wpd::manager::{Manager};

mod folder_operate;
mod local_file_reader;
mod file_info;
mod device_folder_imp;
mod local_folder_imp;
mod device_copy_processor;
mod local_copy_processor;
mod copy_processor;


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TargetStatus {
    NotExist,
    Hidden,
    File,
    Folder,
}

#[derive(Debug)]
struct TargetInspectionResult {
    // 目标路径名称
    target_name: Option<String>,
    // 目标路径状态
    target_status: TargetStatus,
    // 父路径状态
    parent_status: TargetStatus,
    // 父路径,用于判断是否可以创建目标路径
    parent_path: Option<String>,
}

pub fn command_copy(
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
// 获取目标路径信息
// 如果目标路径是隐藏文件或文件夹，返回错误
// 如果目标路径不存在或是文件，判断父路径是否是文件夹，返回父路径和目标路径名称,复制到父文件夹下
// 如果目标路径是文件夹，返回目标路径和空的目标路径名称,copy到目标文件夹下
// dest_is_parent_folder 用来决定目标路径是作为父文件夹还是具体的目标文件夹,true复制到父文件夹下，false复制到具体的目标文件夹下
fn get_destination_path_info<'a>(dest_inspection: &'a TargetInspectionResult, dest_path: &'a str) -> Result<Option<(&'a str, Option<&'a str>)>, Box<dyn std::error::Error>> {
    match dest_inspection.target_status {
        TargetStatus::Hidden => return Err("destination path is a hidden file or folder.".into()),
        TargetStatus::NotExist | TargetStatus::File => {
            match dest_inspection.parent_status {
                TargetStatus::Folder => Ok(Some((
                    dest_inspection.parent_path.as_ref().unwrap().as_str(),
                    dest_inspection.target_name.as_deref(),
                ))),
                _ => Ok(None),
            }
        }
        TargetStatus::Folder => Ok(Some((dest_path, None))),
    }
}


fn do_copy(
    manager: &Manager,
    src_path: &str,
    src_path_type: PathType,
    destination_folder: &mut impl FolderOperate,
    dest_is_parent_folder: bool,
    dest_name: Option<&str>,
    recursive: bool,
    mirror: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match src_path_type {
        PathType::DeviceStorage => {
            let storage_path = DeviceStoragePath::from(src_path)?;
            copy_to_device_storage(manager, src_path, destination_folder, dest_is_parent_folder, dest_name, recursive, mirror)
        }
        PathType::Local => {
            copy_to_local(src_path, destination_folder, dest_is_parent_folder, dest_name, recursive, mirror)
        }
        PathType::Invalid => {
            return Err("invalid source path.".into());
        }
    }
}




fn copy_to_device_storage(
    manager: &Manager,
    src_path: &str,
    destination_folder: &mut impl FolderOperate,
    dest_is_parent_folder: bool,
    dest_name: Option<&str>,
    recursive: bool,
    mirror: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let storage_path = DeviceStoragePath::from(src_path)?;
    if let Some((_device_info, device, content_object)) = find_file_or_folder(manager, &storage_path)? {
        let processor = DeviceCopyProcessor::new(&device, content_object.clone());
        let real_dest_name = dest_name.unwrap_or(&content_object.name);
        processor.copy(
            real_dest_name,
            destination_folder,
            dest_is_parent_folder,
            recursive,
            mirror,
        )
    } else {
        Err("failed to open source path.".into())
    }
}

fn copy_to_local(
    src_path: &str,
    destination_folder: &mut impl FolderOperate,
    dest_is_parent_folder: bool,
    dest_name: Option<&str>,
    recursive: bool,
    mirror: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // 处理本地路径
    let src_path_buf;
    let real_dest_name;
    match dest_name {
        Some(name) => {
            real_dest_name = name;
        }
        None => {
            src_path_buf = PathBuf::from(src_path);
            match src_path_buf.file_name() {
                Some(p) => {
                    real_dest_name = p.to_str().unwrap();
                }
                None => {
                    return Err("cannot copy the root directory.".into());
                }
            }
        }
    }

    let processor = LocalCopyProcessor::new(src_path);
    processor.copy(
        real_dest_name,
        destination_folder,
        dest_is_parent_folder,
        recursive,
        mirror,
    )
}



// 判断是否包含通配符，含通配符不支持
fn has_wildcard(path: &str, path_type: PathType) -> Result<bool, Box<dyn std::error::Error>> {
    let path_to_check = match path_type {
        PathType::DeviceStorage => {
            // 解析 `DeviceStoragePath`，获取实际需要检查的路径
            let storage_path = DeviceStoragePath::from(path)?;
            storage_path.path
        }
        PathType::Local => path.to_string(),
        _ => return Ok(false), // 如果是其他类型，直接返回 false
    };

    // 检查路径中是否包含通配符 `*` 或 `?`
    Ok(path_to_check.split(SEPARATORS).any(|p| p.contains(WILDCARD_CHARACTERS)))
}

fn inspect_path(
    manager: &Manager,
    path: &str,
    path_type: PathType,
) -> Result<TargetInspectionResult, Box<dyn std::error::Error>> {
    match path_type {
        PathType::DeviceStorage => inspect_device_path(manager, path),
        PathType::Local => inspect_local_path(path),
        PathType::Invalid => Err(format!("invalid path: {}", path).into()),
    }
}

fn inspect_device_path(
    manager: &Manager,
    path: &str,
) -> Result<TargetInspectionResult, Box<dyn std::error::Error>> {
    let storage_path = DeviceStoragePath::from(path)?;
    let target_name: Option<String> = storage_path.file_name().and_then(|v| Some(String::from(v)));
    let target_status = inspect_device_path_status(manager, &storage_path)?;

    // 获取父路径状态和名称
    let parent_status: TargetStatus;
    let parent_path: Option<String>;
    match storage_path.parent() {
        Some(p) => {
            parent_status = inspect_device_path_status(manager, &p)?;
            parent_path = Some(p.full_path());
        }
        None => {
            parent_status = TargetStatus::NotExist;
            parent_path = None;
        }
    }

    Ok(TargetInspectionResult {
        target_name,
        target_status,
        parent_status,
        parent_path,
    })
}

// 检查本地路径
fn inspect_local_path(
    path: &str
) -> Result<TargetInspectionResult, Box<dyn std::error::Error>> {
    let path_obj = Path::new(path);

    // 获取目标路径状态和名称
    let target_status = inspect_local_path_status(path_obj)?;
    let target_name = path_obj
        .file_name()
        .and_then(|s| s.to_str().map(String::from));

    if target_status != TargetStatus::NotExist && target_name.is_none() {
        return Err("Failed to get the file name of the destination path.".into());
    }

    // 获取父路径状态和名称
    let (parent_status, parent_path) = match path_obj.parent() {
        Some(p) => {
            let parent_path = p.to_str().map(String::from);
            let parent_status = inspect_local_path_status(p)?;
            (parent_status, parent_path)
        }
        None => (TargetStatus::NotExist, None),
    };

    // 返回目标和父路径的检查结果
    Ok(TargetInspectionResult {
        target_name,
        target_status,
        parent_status,
        parent_path,
    })
}
// 检查本地路径状态，通过判断路径是否存在、是否是隐藏文件、系统文件、文件夹
fn inspect_local_path_status(path_obj: &Path) -> Result<TargetStatus, Box<dyn std::error::Error>> {
    if !path_obj.exists() {
        Ok(TargetStatus::NotExist)
    } else {
        let file_info = FileInfo::from_metadata(&path_obj.metadata()?, "")?;
        if file_info.is_hidden || file_info.is_system {
            Ok(TargetStatus::Hidden)
        } else if file_info.is_folder {
            Ok(TargetStatus::Folder)
        } else {
            Ok(TargetStatus::File)
        }
    }
}

//
fn inspect_device_path_status(
    manager: &Manager,
    storage_path: &DeviceStoragePath,
) -> Result<TargetStatus, Box<dyn std::error::Error>> {
    if let Some((_, _, content_object_info)) = find_file_or_folder(manager, storage_path)? {
        match (
            content_object_info.is_hidden || content_object_info.is_system,
            content_object_info.is_folder() || content_object_info.is_storage(),
            content_object_info.is_file(),
        ) {
            (true, _, _) => Ok(TargetStatus::Hidden),
            (_, true, _) => Ok(TargetStatus::Folder),
            (_, _, true) => Ok(TargetStatus::File),
            _ => Ok(TargetStatus::Hidden), // 处理未知情况
        }
    } else {
        Ok(TargetStatus::NotExist)
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
        let result = command_copy(&paths, false, false);
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
        let result = command_copy(&paths, false, false);
        assert!(result.is_ok());
        Ok(())
    }
}