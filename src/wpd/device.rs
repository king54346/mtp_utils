use std::fmt::Debug;
use windows::core::{Error, GUID, PWSTR, PROPVARIANT as propvar};
use windows::core::imp::{PROPVARIANT};
use windows::Win32::Devices::PortableDevices::{IEnumPortableDeviceObjectIDs, IPortableDevice, IPortableDeviceContent, IPortableDeviceKeyCollection, IPortableDeviceProperties, IPortableDevicePropVariantCollection, IPortableDeviceResources, IPortableDeviceValues, PORTABLE_DEVICE_DELETE_WITH_RECURSION, PortableDevice, PortableDeviceKeyCollection, PortableDeviceManager, PortableDevicePropVariantCollection, PortableDeviceValues, WPD_CONTENT_TYPE_FOLDER, WPD_CONTENT_TYPE_FUNCTIONAL_OBJECT, WPD_CONTENT_TYPE_GENERIC_FILE, WPD_FUNCTIONAL_CATEGORY_DEVICE, WPD_FUNCTIONAL_CATEGORY_STORAGE, WPD_FUNCTIONAL_OBJECT_CATEGORY, WPD_OBJECT_CAN_DELETE, WPD_OBJECT_CONTENT_TYPE, WPD_OBJECT_DATE_CREATED, WPD_OBJECT_DATE_MODIFIED, WPD_OBJECT_FORMAT, WPD_OBJECT_FORMAT_ALL, WPD_OBJECT_ISHIDDEN, WPD_OBJECT_ISSYSTEM, WPD_OBJECT_NAME, WPD_OBJECT_ORIGINAL_FILE_NAME, WPD_OBJECT_PARENT_ID, WPD_OBJECT_SIZE, WPD_RESOURCE_DEFAULT};
use windows::Win32::Foundation::S_OK;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, IStream};
use crate::wpd::manager::DeviceInfo;
use crate::wpd::resource_stream::{ResourceReader, ResourceWriter};
use crate::wpd::utils::{IDStr, WStrBuf, WStrPtr};

pub struct ContentObject {
    pub id: IDStr,
}

impl ContentObject {
    pub fn new(id: IDStr) -> ContentObject {
        ContentObject { id }
    }
}

impl Clone for ContentObject {
    fn clone(&self) -> Self {
        ContentObject {
            id: self.id.clone(),
        }
    }
}

impl Debug for ContentObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContentObject")
            .field("id", &self.id)
            .finish()
    }
}

// 对象详情信息
#[derive(Debug)]
pub struct ContentObjectInfo {
    pub content_object: ContentObject,
    /// Name to display
    pub name: String,
    /// Content type GUID
    content_type: GUID,
    /// 如果device获取storage，则为零。
    functional_object_category: GUID,
    /// Size of the resource data
    pub data_size: u64,
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

impl Clone for ContentObjectInfo {
    fn clone(&self) -> Self {
        ContentObjectInfo {
            content_object: self.content_object.clone(),
            name: self.name.clone(),
            content_type: self.content_type.clone(),
            functional_object_category: self.functional_object_category.clone(),
            data_size: self.data_size,
            is_hidden: self.is_hidden,
            is_system: self.is_system,
            can_delete: self.can_delete,
            time_created: self.time_created.clone(),
            time_modified: self.time_modified.clone(),
        }
    }
}

impl ContentObjectInfo {
    pub fn is_functional_object(&self) -> bool {
        self.content_type == WPD_CONTENT_TYPE_FUNCTIONAL_OBJECT
    }

    pub fn is_device(&self) -> bool {
        self.functional_object_category == WPD_FUNCTIONAL_CATEGORY_DEVICE
    }

    pub fn is_storage(&self) -> bool {
        self.functional_object_category == WPD_FUNCTIONAL_CATEGORY_STORAGE
    }

    pub fn is_folder(&self) -> bool {
        self.content_type == WPD_CONTENT_TYPE_FOLDER
    }

    pub fn is_file(&self) -> bool {
        !self.is_functional_object() && !self.is_folder()
    }
}

pub struct Device {
    device: IPortableDevice,
    content: IPortableDeviceContent,
    properties: IPortableDeviceProperties,
    resources: IPortableDeviceResources,
    pub name: String,
}

impl Device {

    pub fn open(info: &DeviceInfo) -> Result<Device, Error> {
        log::trace!("open Device ({})", &info.name);
        //  创建 PortableDevice 实例
        let device: IPortableDevice = unsafe {
            CoCreateInstance(&PortableDevice, None, CLSCTX_ALL)?
        };
        // 创建 PortableDeviceValues 实例
        let values: IPortableDeviceValues = unsafe {
            CoCreateInstance(&PortableDeviceValues, None, CLSCTX_ALL)?
        };

        unsafe {
            device.Open(info.id.clone().as_pwstr(), &values)?;
        }
        // 获取device的内容、属性和资源
        let content = unsafe { device.Content()? };
        let properties = unsafe { content.Properties()? };
        let resources = unsafe { content.Transfer()? };

        Ok(Device {
            device,
            content,
            properties,
            resources,
            name: info.name.clone(),
        })
    }

    pub fn get_root_object(&self) -> ContentObject {
        ContentObject::new(IDStr::create_empty())
    }

    // 获取parent对象下的所有对象的迭代器
    pub fn get_object_iterator(&self, parent: &ContentObject) -> Result<ContentObjectIterator, Error> {
        let enum_object_ids = unsafe {
            self.content.EnumObjects(
                    0,
                    parent.id.clone().as_pwstr(),
                    None,
                )?
        };

        Ok(ContentObjectIterator::new(enum_object_ids))
    }
    // 获取对象信息，对象包括是device、storages、文件夹、文件。
    pub fn get_object_info(&self, object: ContentObject) -> Result<ContentObjectInfo, Error> {
        let key_collection: IPortableDeviceKeyCollection = unsafe { CoCreateInstance(&PortableDeviceKeyCollection, None, CLSCTX_ALL)? };

        unsafe {
            for key in [
                &WPD_OBJECT_NAME,
                &WPD_OBJECT_ORIGINAL_FILE_NAME,
                &WPD_OBJECT_SIZE,
                &WPD_OBJECT_CONTENT_TYPE,
                &WPD_FUNCTIONAL_OBJECT_CATEGORY,
                &WPD_OBJECT_ISHIDDEN,
                &WPD_OBJECT_ISSYSTEM,
                &WPD_OBJECT_CAN_DELETE,
                &WPD_OBJECT_DATE_CREATED,
                &WPD_OBJECT_DATE_MODIFIED,
            ] {
                key_collection.Add(key)?;
            }
        }
        // 获取对象的属性值，上述key_collection中的属性值
        let values = unsafe { self.properties.GetValues(object.id.clone().as_pwstr(), &key_collection)? };
        // 从属性值中提取对象名称、对象类型、对象大小、是否隐藏、是否系统、是否可删除、创建时间、修改时间
        let name = unsafe { values.GetStringValue(&WPD_OBJECT_NAME)?.to_string()? };
        let content_type = unsafe { values.GetGuidValue(&WPD_OBJECT_CONTENT_TYPE)? };

        let (mut object_orig_name, mut data_size, mut is_hidden, mut is_system, mut can_delete) = (None, 0, false, false, true);
        let mut functional_object_category = GUID::zeroed();
        let (mut time_created, mut time_modified) = (None, None);
        // 根据内容类型处理属性值
        // 如果是device、storages 可以获取FUNCTIONAL_OBJECT GUID
        // 如果是文件夹、文件获取文件名称、文件大小、是否隐藏、是否系统、是否可删除、创建时间、修改时间
        unsafe {
            if content_type == WPD_CONTENT_TYPE_FUNCTIONAL_OBJECT {
                functional_object_category = values.GetGuidValue(&WPD_FUNCTIONAL_OBJECT_CATEGORY)?;
            } else {
                object_orig_name = values.GetStringValue(&WPD_OBJECT_ORIGINAL_FILE_NAME)?.to_string().ok();
                is_hidden = values.GetBoolValue(&WPD_OBJECT_ISHIDDEN).is_ok_and(|x| x.as_bool());
                is_system = values.GetBoolValue(&WPD_OBJECT_ISSYSTEM).is_ok_and(|x| x.as_bool());
                can_delete = values.GetBoolValue(&WPD_OBJECT_CAN_DELETE).is_ok_and(|x| x.as_bool());
                time_created = values.GetStringValue(&WPD_OBJECT_DATE_CREATED).iter().find_map(|x| x.to_string().ok());
                time_modified = values.GetStringValue(&WPD_OBJECT_DATE_MODIFIED).iter().find_map(|x| x.to_string().ok());

                if content_type != WPD_CONTENT_TYPE_FOLDER {
                    data_size = values.GetUnsignedLargeIntegerValue(&WPD_OBJECT_SIZE)?;
                }
            }
        }

        Ok(ContentObjectInfo {
            content_object: object,
            name,
            content_type,
            functional_object_category,
            data_size,
            is_hidden,
            is_system,
            can_delete,
            time_created,
            time_modified,
        })
    }


    pub fn get_resoure(&self, object: &ContentObject) -> Result<ResourceReader, Error> {
        const STGM_READ: u32 = 0;
        let mut buff_size: u32 = 0;
        let mut stream_receptor: Option<IStream> = None;
        unsafe {
            self.resources
                .GetStream(
                    object.id.clone().as_pwstr(),
                    &WPD_RESOURCE_DEFAULT,
                    STGM_READ,
                    &mut buff_size,
                    &mut stream_receptor,
                )?;
        }
        let stream = stream_receptor.unwrap();
        Ok(ResourceReader::new(stream, buff_size))
    }
    // 创建文件,parent为父文件夹对象，name为文件名称，size为文件大小，created为创建时间，modified为修改时间
    // todo 创建时间和修改时间暂时不支持
    pub fn create_file(
        &self,
        parent: &ContentObject,
        name: &str,
        size: u64,
        created: &Option<String>,
        modified: &Option<String>,
    ) -> Result<ResourceWriter, Error> {
        let values: IPortableDeviceValues = unsafe { CoCreateInstance(&PortableDeviceValues, None, CLSCTX_ALL)? };
        let mut name_buf = WStrBuf::from(name, true);
        unsafe {
            values
                .SetStringValue(&WPD_OBJECT_PARENT_ID, parent.id.clone().as_pwstr())?;
            values
                .SetStringValue(&WPD_OBJECT_NAME, name_buf.as_pwstr())?;
            values
                .SetStringValue(&WPD_OBJECT_ORIGINAL_FILE_NAME, name_buf.as_pwstr())?;
            values
                .SetGuidValue(&WPD_OBJECT_FORMAT, &WPD_OBJECT_FORMAT_ALL)?;
            values
                .SetGuidValue(&WPD_OBJECT_CONTENT_TYPE, &WPD_CONTENT_TYPE_GENERIC_FILE)?;
            values
                .SetUnsignedLargeIntegerValue(&WPD_OBJECT_SIZE, size)?;
        }
        // if let Some(&created_dt) = created.as_ref() {
        //     let dt = format_datetime(&created_dt);
        //     let mut dt_buf = WStrBuf::from(&dt, true);
        //     unsafe {
        //         values
        //             .SetStringValue(&WPD_OBJECT_DATE_CREATED, dt_buf.as_pwstr())
        //             .ok()?;
        //     }
        // }
        // if let Some(&modified_dt) = modified.as_ref() {
        //     let dt = format_datetime(&modified_dt);
        //     let mut dt_buf = WStrBuf::from(&dt, true);
        //     unsafe {
        //         values
        //             .SetStringValue(&WPD_OBJECT_DATE_MODIFIED, dt_buf.as_pwstr())
        //             .ok()?;
        //     }
        // }

        let mut stream_receptor: Option<IStream> = None;
        let mut buffer_size: u32 = 0;

        unsafe {
            self.content
                .CreateObjectWithPropertiesAndData(
                    &values,
                    &mut stream_receptor,
                    &mut buffer_size,
                    std::ptr::null_mut(),
                )?;
        }

        let stream = stream_receptor.unwrap();

        Ok(ResourceWriter::new(stream, buffer_size))
    }
    // 创建文件夹,parent为父文件夹对象，name为文件夹名称
    pub fn create_folder(&self, parent: &ContentObject, name: &str) -> Result<ContentObject, Error> {
        let values: IPortableDeviceValues = unsafe { CoCreateInstance(&PortableDeviceValues, None, CLSCTX_ALL)? };
        // name 转成 pwstr
        let mut name_buf = WStrBuf::from(name, true);
        unsafe {
            values
                .SetStringValue(&WPD_OBJECT_PARENT_ID, parent.id.clone().as_pwstr())?;
            values
                .SetStringValue(&WPD_OBJECT_NAME, name_buf.as_pwstr())?;
            values
                .SetGuidValue(&WPD_OBJECT_FORMAT, &WPD_OBJECT_FORMAT_ALL)?;
            values
                .SetGuidValue(&WPD_OBJECT_CONTENT_TYPE, &WPD_CONTENT_TYPE_FOLDER)?;
        }

        let mut object_id = WStrPtr::create();
        unsafe {
            self.content
                .CreateObjectWithPropertiesOnly(&values, object_id.as_pwstr_mut_ptr())?;
        }
        let content_object = ContentObject::new(object_id.to_idstr());

        Ok(content_object)
    }

    // 删除对象
    pub fn delete(&self, object: &ContentObject) -> Result<(), Error> {
        let mut str = object.id.clone();
        unsafe {
            let collection: IPortableDevicePropVariantCollection = CoCreateInstance(&PortableDevicePropVariantCollection, None, CLSCTX_ALL)?;
            let propvariant = propvar::default();
            let mut var: PROPVARIANT = core::mem::zeroed();
            var.Anonymous.Anonymous.vt = 31; // VT_LPWSTR
            var.Anonymous.Anonymous.Anonymous.pwszVal = str.as_pwstr().as_ptr();
            let propvar = propvar::from_raw(var);
            collection.Add(&propvar)?;
            self.content.Delete(
                PORTABLE_DEVICE_DELETE_WITH_RECURSION.0 as u32,
                &collection,
                std::ptr::null_mut(),
            )?;
            // 延长str的生命周期
            std::mem::forget(str);
        }
        println!("delete object: {:?}", object.id);
        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        // log::trace!("drop Device ({})", &self.name);
        unsafe {
            let _ = self.device.Close();
        }
    }
}


pub struct ContentObjectIterator {
    enum_object_ids: IEnumPortableDeviceObjectIDs,
    object_ids: Option<Vec<IDStr>>,
    completed: bool,
}

impl ContentObjectIterator {
    fn new(enum_object_ids: IEnumPortableDeviceObjectIDs) -> ContentObjectIterator {
        ContentObjectIterator {
            enum_object_ids,
            object_ids: None,
            completed: false,
        }
    }

    pub fn next(&mut self) -> Result<Option<ContentObject>, Error> {
        if let Some(object_ids_ref) = self.object_ids.as_mut() {
            if let Some(id) = object_ids_ref.pop() {
                return Ok(Some(ContentObject::new(id)));
            }
        }
        //
        if self.completed {
            return Ok(None);
        }

        const ARRAY_SIZE: u32 = 32;
        let mut object_ids:Vec<PWSTR> = vec![PWSTR::null(); ARRAY_SIZE as usize];
        let mut read = 0u32;
        let err;
        unsafe {
            err = self
                .enum_object_ids
                .Next(object_ids.as_mut_slice(), &mut read);
        }
        err.ok()?;

        if read == 0 {
            self.object_ids = None;
            self.completed = true;
            return Ok(None);
        }

        let mut object_ids_vec = object_ids
            .iter()
            .take(read as usize)
            .map(|p| IDStr::from(*p))
            .collect::<Vec<IDStr>>();
        object_ids_vec.reverse(); // for moving item out by pop()
        self.object_ids = Some(object_ids_vec);

        if err != S_OK {
            self.completed = true;
        }

        self.next()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::GUID;
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
    use crate::find::find_file_or_folder;
    use crate::path;
    use crate::wpd::manager::Manager;
    use crate::wpd::utils::IDStr;

    #[test]
    fn create_folder_success() {
        unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
        let manager = Manager::get_portable_device_manager().unwrap();
        let storage_path = path::DeviceStoragePath::from("Redmi K70:内部存储设备:/Pictures").unwrap();
        let option = find_file_or_folder(&manager, &storage_path).unwrap();
        let (device_info, device, content_object_info) = option.unwrap();
        println!("device_info: {:?}", device_info);
        println!("storage_object: {:?}", content_object_info);

        let folder_name = "New Folder";
        let result = device.create_folder(&content_object_info.content_object, folder_name);
        assert!(result.is_ok());
        println!("create folder: {:?}", result.unwrap());
    }

    #[test]
    fn delete_folder_success() {
        unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
        let manager = Manager::get_portable_device_manager().unwrap();
        let folder_name = "New Folder";
        let path = format!("Redmi K70:内部存储设备:/Pictures/{}", folder_name);
        let storage_path = path::DeviceStoragePath::from(path.as_str()).unwrap();
        let option = find_file_or_folder(&manager, &storage_path).unwrap();
        let (device_info, device, content_object_info) = option.unwrap();
        println!("device_info: {:?}", device_info);
        println!("storage_object: {:?}", content_object_info);
        let result = device.delete(&content_object_info.content_object);
        assert!(result.is_ok());
    }

    #[test]
    fn create_file_success() {
        unsafe { CoInitializeEx(Some(std::ptr::null_mut()), COINIT_MULTITHREADED).ok().unwrap(); }
        let manager = Manager::get_portable_device_manager().unwrap();
        let storage_path = path::DeviceStoragePath::from("Redmi K70:内部存储设备:/Pictures/").unwrap();
        let option = find_file_or_folder(&manager, &storage_path).unwrap();
        let (device_info, device, content_object_info) = option.unwrap();
        println!("device_info: {:?}", device_info);
        println!("storage_object: {:?}", content_object_info);

        let file_name = "New File.txt";
        let file_size = 1024;
        let result = device.create_file(&content_object_info.content_object, file_name, file_size,&None, &None);
        let mut writer = result.unwrap();
        let mut buffer = vec![0u8; file_size as usize];
        for i in 0..file_size {
            buffer[i as usize] = (i % 256) as u8;
        }
        let write_result = writer.write(&buffer);
        assert!(write_result.is_ok());
        let commit_result = writer.commit();
        println!("create file: {:?}", commit_result);
        assert!(commit_result.is_ok());
        device.delete(&commit_result.unwrap()).unwrap();
    }

}