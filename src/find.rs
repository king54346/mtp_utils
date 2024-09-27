use crate::common::path_matcher::{create_path_pattern_matcher, PathMatcher, PathMatchingState};
use crate::list::{list_devices, list_device_storages};
use crate::path::{DeviceStoragePath, SEPARATORS};
use crate::wpd::device::{ContentObject, ContentObjectInfo, ContentObjectIterator, Device};
use crate::wpd::manager::{DeviceInfo, Manager};

// 查找设备存储
// input: storage_path = "设备名:存储名"
// output: 设备信息、设备实例和存储信息
pub fn find_storage(manager: &Manager, storage_path: &DeviceStoragePath) -> Result<Option<(DeviceInfo, Device, ContentObjectInfo)>, Box<dyn std::error::Error>> {
    log::trace!("find_device_storage: storage_path = {:?}", storage_path);
    fn ensure_single_match<T>(
        vec: Vec<T>,
        entity_name: &str,
        search_key: &str,
    ) -> Result<T, Box<dyn std::error::Error>> {
        match vec.len() {
            0 => Err(format!("{} was not found: {}", entity_name, search_key).into()),
            1 => Ok(vec.into_iter().next().unwrap()),
            _ => Err(format!("multiple {} were matched: {}", entity_name, search_key).into()),
        }
    }
    // 1. 找到设备
    let device_info = ensure_single_match(
        list_devices(manager, Some(&storage_path.device_name))?,
        "device",
        &storage_path.device_name,
    )?;

    // 2. 打开设备
    let device = Device::open(&device_info)?;

    // 3. 找到存储
    let storage_object = ensure_single_match(
        list_device_storages(&device, Some(&storage_path.storage_name))?,
        "storage",
        &format!("{}:{}", &storage_path.device_name, &storage_path.storage_name),
    )?;

    log::trace!(
        "find_device_storage: found {:?} {:?}",
        &device_info,
        &storage_object
    );
    // 返回设备信息、设备实例和存储信息
    Ok(Some((device_info, device, storage_object)))
}

// 查找文件或文件夹，
// input: storage_path = "设备名:存储名:路径"
// output: 设备信息、设备实例和存储信息
pub fn find_file_or_folder(manager: &Manager, storage_path: &DeviceStoragePath) -> Result<Option<(DeviceInfo, Device, ContentObjectInfo)>, Box<dyn std::error::Error>> {
    log::trace!("find_device_file_or_folder");
    // 尝试查找设备存储
    if let Some((device_info, device, storage_object)) = find_storage(manager, storage_path)?{
        log::trace!("find_device_file_or_folder: storage found");
        // 尝试查找文件或文件夹
        match find_device_storage_file_or_folder(
            &device,
            &device_info,
            &storage_object,
            &storage_path.path,
        )? {
            Some((content_object_info, _)) => {
                log::trace!("find_device_file_or_folder: file/folder object found");
                Ok(Some((device_info, device, content_object_info)))
            }
            None => {
                log::trace!("find_device_file_or_folder: no object found");
                Ok(None)
            }
        }
    } else {
        log::trace!("find_device_file_or_folder: storage was not found");
        Ok(None)
    }
}

// 查询某个设备的某个storage的文件或文件夹，path：设备名:存储名:路径
fn find_device_storage_file_or_folder(
    device: &Device,
    device_info: &DeviceInfo,
    storage_object: &ContentObjectInfo,
    path: &str,
) -> Result<Option<(ContentObjectInfo, String)>, Box<dyn std::error::Error>> {
    let mut result: Option<(ContentObjectInfo, String)> = None;
    iterate_file_or_folder(
        device,
        device_info,
        storage_object,
        path,
        false,
        |content_object_info, path| {
            result = Some((content_object_info.clone(), String::from(path)));
        },
    )?;
    Ok(result)
}
// 获取storage的文件或文件夹
//  recursive: 是否递归, callback: 回调函数,
pub fn iterate_file_or_folder<F>(
    device: &Device,
    device_info: &DeviceInfo,
    storage_object: &ContentObjectInfo,
    path: &str,
    recursive: bool,
    mut callback: F,
) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(&ContentObjectInfo, &str),
{
    log::trace!("device_iterate path={}", path);

    let root_path_matcher = create_path_pattern_matcher(path)?;
    let storage_path = format!("{}:{}:", &device_info.name, &storage_object.name);
    let (state, next_matcher) = root_path_matcher.matches_root();
    log::trace!("  matches_root state {:?}", &state);

    match state {
        PathMatchingState::Rejected => Ok(()),
        PathMatchingState::Completed => {
            let path = join_path(&storage_path, "");
            log::trace!("  call callback path={:?}", &path);
            callback(storage_object, &path);

            if recursive {
                log::trace!("  go recursively");
                if let Some(iter) = get_object_iterator(device, &storage_object.content_object, &storage_path)? {
                    iterate_file_or_folder_recursive(device, iter, &PathMatcher::CompleteMatcher, storage_path, &mut callback, recursive)?;
                }
            }
            Ok(())
        }
        PathMatchingState::Accepted => {
            if let Some(iter) = get_object_iterator(device, &storage_object.content_object, &storage_path)? {
                iterate_file_or_folder_recursive(device, iter, next_matcher.unwrap(), storage_path, &mut callback, recursive)?;
            }
            Ok(())
        }
    }
}

fn get_object_iterator(
    device: &Device,
    content_object: &ContentObject,
    storage_path: &str,
) -> Result<Option<ContentObjectIterator>, Box<dyn std::error::Error>> {
    match device.get_object_iterator(content_object) {
        Err(err) => {
            log::debug!("{}", err);
            log::warn!("failed to open: {}", &storage_path);
            Ok(None)
        }
        Ok(iter) => Ok(Some(iter)),
    }
}

fn iterate_file_or_folder_recursive<F>(
    device: &Device,
    mut content_object_iterator: ContentObjectIterator,
    path_matcher: &PathMatcher,
    base_path: String,
    callback: &mut F,
    recursive: bool,
) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(&ContentObjectInfo, &str),
{
    log::trace!("device_iterate_recursive start base_path={}", &base_path);

    while let Some(content_object) = content_object_iterator.next()? {
        log::trace!("  detected {:?}", &content_object);
        let content_object_info = device.get_object_info(content_object)?;
        log::trace!("  details {:?}", &content_object_info);

        if !content_object_info.is_file() && !content_object_info.is_folder() {
            log::trace!("  --> skip");
            continue;
        }

        let (state, next_matcher) = path_matcher.matches(&content_object_info.name, content_object_info.is_folder());
        log::trace!("  matching state {:?}", &state);

        let next_base_path = join_path(&base_path, &content_object_info.name);
        match state {
            PathMatchingState::Rejected => (),
            PathMatchingState::Completed => {
                log::trace!("  call callback path={:?}", &next_base_path);
                callback(&content_object_info, &next_base_path);
                if recursive {
                    log::trace!("  go recursively");
                    if let Some(iter) = get_object_iterator(device, &content_object_info.content_object, &next_base_path)? {
                        iterate_file_or_folder_recursive(device, iter, &PathMatcher::CompleteMatcher, next_base_path, callback, recursive)?;
                    }
                }
            }
            PathMatchingState::Accepted => {
                if let Some(iter) = get_object_iterator(device, &content_object_info.content_object, &next_base_path)? {
                    iterate_file_or_folder_recursive(device, iter, next_matcher.unwrap(), next_base_path, callback, recursive)?;
                }
            }
        }
    }
    log::trace!("device_iterate_recursive end base_path={}", &base_path);
    Ok(())
}

fn join_path(base_path: &str, sub_path: &str) -> String {
    let mut s = String::from(base_path);
    if !s.ends_with(SEPARATORS) {
        s.push('\\');
    }
    s.push_str(sub_path);
    s
}





