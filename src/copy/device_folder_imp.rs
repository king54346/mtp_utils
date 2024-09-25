use std::collections::{HashMap, HashSet};
use windows::core::Error;
use crate::copy::folder_operate::FolderOperate;
use crate::wpd::device::{ContentObjectInfo, Device};
use super::file_info::FileInfo;
use crate::glob::file_reader::FileReader;

pub struct DeviceFolder<'d> {
    device: &'d Device,
    // 文件夹对象信息
    folder_object_info: ContentObjectInfo,
    // 文件夹下所有的文件 key: 文件名/文件夹名，value: 文件信息
    entry_map: HashMap<String, ContentObjectInfo>,
    // 保留的文件或文件夹
    retained: HashSet<String>,
}

impl<'d> DeviceFolder<'d> {
    // 给某个设备的某个文件夹创建一个新的DeviceFolder对象
    pub fn new(device: &'d Device, folder_object_info: ContentObjectInfo) -> Result<DeviceFolder<'d>, Box<dyn std::error::Error>> {
        let mut iter = device.get_object_iterator(&folder_object_info.content_object)?;
        let mut entry_map = HashMap::<String, ContentObjectInfo>::new();
        // 遍历文件夹中的对象
        while let Some(object) = iter.next()? {
            // 获取对象信息，存入entry_map
            let object_info = device.get_object_info(object)?;
            entry_map.insert(object_info.name.clone(), object_info);
        }
        let retained = HashSet::<String>::new();
        Ok(DeviceFolder::<'d> {
            device,
            folder_object_info,
            entry_map,
            retained,
        })
    }
}

impl<'d> FolderOperate for DeviceFolder<'d> {
    // 获取文件的信息
    fn get_file_info(&mut self, name: &str) -> Result<Option<FileInfo>, Box<dyn std::error::Error>> {
        match self.entry_map.get(name) {
            None => Ok(None),
            Some(object_info) => Ok(Some(FileInfo::from_content_object_info(object_info)?)),
        }
    }

    // 创建文件
    fn create_file(
        &mut self,
        name: &str,
        reader: &mut impl FileReader,
        size: u64,
        created: &Option<String>,
        modified: &Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 创建文件
        let mut resource_writer = self.device.create_file(
            &self.folder_object_info.content_object,
            name,
            size,
            created,
            modified,
        )?;

        // 循环读取并写入数据
        while let Some(bytes) = reader.seek(resource_writer.get_buffer_size())? {
            resource_writer.write(bytes)?;
        }

        // 提交资源并获取内容对象
        let content_object = resource_writer.commit()?;
        let object_info = self.device.get_object_info(content_object)?;

        // 更新条目映射
        self.entry_map.insert(object_info.name.clone(), object_info);

        Ok(())
    }

    fn open_or_create_folder<FBeforeOpen, FBeforeCreate>(
        &mut self,
        name: &str,
        before_open: FBeforeOpen,
        before_create: FBeforeCreate,
    ) -> Result<Box<Self>, Box<dyn std::error::Error>>
        where
            FBeforeOpen: FnOnce(&str),
            FBeforeCreate: FnOnce(&str),
    {
        if let Some(object_info_ref) = self.entry_map.get(name) {
            // 如果文件夹已存在，则打开它
            before_open(name);
            Ok(Box::new(DeviceFolder::new(self.device, object_info_ref.clone())?))
        } else {
            // 如果文件夹不存在，则创建它
            before_create(name);
            let content_object = self.device.create_folder(&self.folder_object_info.content_object, name)?;
            let object_info = self.device.get_object_info(content_object)?;
            self.entry_map.insert(object_info.name.clone(), object_info.clone());
            Ok(Box::new(DeviceFolder::new(self.device, object_info)?))
        }
    }

    fn delete_file_or_folder(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(object_info) = self.entry_map.get(name) {
            self.device.delete(&object_info.content_object)?;
            self.entry_map.remove(name);
        }
        Ok(())
    }

    fn retain(&mut self, name: &str) {
        self.retained.insert(String::from(name));
    }

    fn delete_unretained<FBeforeDeleteFile, FBeforeDeleteFolder>(
        &mut self,
        before_delete_file: FBeforeDeleteFile,
        before_delete_folder: FBeforeDeleteFolder,
    ) -> Result<(), Box<dyn std::error::Error>>
        where
            FBeforeDeleteFile: Fn(&str),
            FBeforeDeleteFolder: Fn(&str),
    {
        let mut delete_error: Option<Error> = None;
        let names_to_delete: Vec<String> = self.entry_map.iter()
            .filter_map(|(name, object_info)| {
                if (object_info.is_file() || object_info.is_folder()) && !self.retained.contains(name) {
                    if object_info.is_file() {
                        before_delete_file(name);
                    } else {
                        before_delete_folder(name);
                    }

                    // 尝试删除对象，捕获错误
                    if let Err(err) = self.device.delete(&object_info.content_object) {
                        delete_error = Some(err);
                        None // 返回 None 以停止迭代
                    } else {
                        Some(name.to_string()) // 返回要删除的名称
                    }
                } else {
                    None // 不处理的情况
                }
            })
            .collect();

        // 移除已删除的条目
        for name in names_to_delete {
            self.entry_map.remove(&name);
        }

        // 处理删除错误
        delete_error.map_or(Ok(()), |err| Err(err.into()))
    }
}
