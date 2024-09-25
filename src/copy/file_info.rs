use crate::wpd::device::ContentObjectInfo;

use std::{
    fs::Metadata,
    os::windows::prelude::MetadataExt,
};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct FileInfo {
    /// Name to display
    pub name: String,
    /// Size of the resource data
    pub data_size: u64,
    /// Whether this entry is a folder
    pub is_folder: bool,
    /// Hidden flag
    pub is_hidden: bool,
    /// System flag
    pub is_system: bool,
    /// Whether the object can be deleted
    pub can_delete: bool,
    /// Time created (or None if not provided)
    pub time_created: Option<String>,
    /// Time modified (or None if not provided)
    pub time_modified: Option<String>,
}

impl FileInfo {
    pub fn from_content_object_info(info: &ContentObjectInfo) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(FileInfo {
            name: info.name.clone(),
            data_size: info.data_size,
            is_folder: info.is_folder(),
            is_hidden: info.is_hidden,
            is_system: info.is_system,
            can_delete: info.can_delete,
            time_created: info.time_created.clone(),
            time_modified: info.time_modified.clone(),
        })
    }

    pub fn from_metadata(
        metadata: &Metadata,
        name: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let created_date_time  = system_time_to_string(metadata.created()?);
        let modified_date_time =  system_time_to_string(metadata.modified()?);
        //  Windows 文件属性标志位 2 为隐藏，4 为系统 1 为只读 0x10 为目录。。。
        //  通过按位或操作，可以组合多个属性
        let file_attr = metadata.file_attributes();
        let data_size = if metadata.is_dir() {
            0
        } else {
            metadata.file_size()
        };
        Ok(FileInfo {
            name: name.to_string(),
            data_size,
            is_folder: metadata.is_dir(),
            is_hidden: (file_attr & 2) != 0,
            is_system: (file_attr & 4) != 0,
            can_delete: true,
            time_created: Some(created_date_time),
            time_modified: Some(modified_date_time),
        })
    }
}

fn system_time_to_string(time: SystemTime) -> String {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let timestamp = duration.as_secs();
            timestamp.to_string()
        }
        Err(e) => format!("Error: {:?}", e),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::fs::metadata;
    use std::time::{SystemTime, Duration};
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
    use crate::find::find_file_or_folder;
    use crate::path;
    use crate::wpd::manager::Manager;

    #[test]
    fn from_content_object_info_creates_correct_file_info() {
        unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
        let manager = Manager::get_portable_device_manager().unwrap();
        let storage_path = path::DeviceStoragePath::from("Redmi K70:内部存储设备:/Pictures").unwrap();
        let option = find_file_or_folder(&manager, &storage_path).unwrap();
        let (device_info, device, content_object_info) = option.unwrap();
        let file_info = FileInfo::from_content_object_info(&content_object_info).unwrap();
        println!("{:?}", file_info);
    }

    #[test]
    fn from_metadata_creates_correct_file_info() {
        let file_path = "test_file.txt";
        let mut file = File::create(file_path).unwrap();
        writeln!(file, "Hello, world!").unwrap();
        let metadata = metadata(file_path).unwrap();

        let file_info = FileInfo::from_metadata(&metadata, file_path).unwrap();

        assert_eq!(file_info.name, file_path.to_string());
        assert_eq!(file_info.data_size, metadata.len());
        assert!(!file_info.is_folder);
        assert!(!file_info.is_hidden);
        assert!(!file_info.is_system);
        assert!(file_info.can_delete);
        assert!(file_info.time_created.is_some());
        assert!(file_info.time_modified.is_some());

        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn from_metadata_handles_directory() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path();

        let metadata = metadata(dir_path).unwrap();

        let file_info = FileInfo::from_metadata(&metadata, dir_path.to_str().unwrap()).unwrap();
        println!("{:?}", file_info);
        assert_eq!(file_info.name, dir_path.to_str().unwrap().to_string());
        assert_eq!(file_info.data_size, 0);
        assert!(file_info.is_folder);
        assert!(!file_info.is_hidden);
        assert!(!file_info.is_system);
        assert!(file_info.can_delete);
        assert!(file_info.time_created.is_some());
        assert!(file_info.time_modified.is_some());
    }

    #[test]
    fn system_time_to_string_converts_correctly() {
        let system_time = SystemTime::UNIX_EPOCH + Duration::new(1627846261, 0);
        let time_string = system_time_to_string(system_time);
        assert_eq!(time_string, "1627846261");
    }

    #[test]
    fn system_time_to_string_handles_error() {
        let system_time = SystemTime::UNIX_EPOCH - Duration::new(1, 0);
        let time_string = system_time_to_string(system_time);
        assert!(time_string.starts_with("Error:"));
    }
}