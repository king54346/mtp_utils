use crate::copy::copy_processor::{can_skip_copying, CopyProcessor, report_copying_end, report_copying_start, report_creating_new_folder, report_delete_file, report_delete_folder};
use crate::copy::folder_operate::FolderOperate;
use crate::wpd::device::{ContentObjectInfo, Device};
use super::file_info::FileInfo;

pub struct DeviceCopyProcessor<'d> {
    device: &'d Device,
    source_root_object_info: ContentObjectInfo,
}

impl<'d> DeviceCopyProcessor<'d> {
    pub fn new(device: &'d Device, source_root_object_info: ContentObjectInfo) -> Self {
        Self {
            device,
            source_root_object_info,
        }
    }
}

impl<'d> CopyProcessor for DeviceCopyProcessor<'d> {
    fn copy(
        &self,
        name: &str,
        dest: &mut impl FolderOperate,
        dest_is_parent_folder: bool,
        recursive: bool,
        mirror: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        copy_iter(
            self.device,
            dest,
            dest_is_parent_folder,
            &self.source_root_object_info,
            name,
            recursive,
            mirror,
        )
    }
}

fn copy_iter(
    device: &Device,
    dest: &mut impl FolderOperate,
    dest_is_parent_folder: bool,
    target_object_info: &ContentObjectInfo,
    dest_name: &str,
    recursive: bool,
    mirror: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // 过滤系统文件和隐藏文件
    if target_object_info.is_system || target_object_info.is_hidden {
        return Ok(());
    }
    // 根据对象类型决定复制逻辑
    if target_object_info.is_file() {
        copy_file(device, dest, target_object_info, dest_name)?;
    } else if target_object_info.is_folder() {
        copy_folder(device, dest, dest_is_parent_folder, target_object_info, dest_name, recursive, mirror)?;
    }
    Ok(())
}

// 复制文件的逻辑
fn copy_file(
    device: &Device,
    dest: &mut impl FolderOperate,
    target_object_info: &ContentObjectInfo,
    dest_name: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let src_file_info = FileInfo::from_content_object_info(target_object_info)?;
    let dest_file_info = dest.get_file_info(dest_name)?;

    // 如果可以跳过复制，则直接返回
    if let Some(dest_file_info_ref) = dest_file_info.as_ref() {
        if can_skip_copying(&src_file_info, dest_file_info_ref) {
            dest.retain(dest_name);
            return Ok(());
        }
    }

    // 如果目标文件已经存在，先删除它
    if dest_file_info.is_some() {
        dest.delete_file_or_folder(dest_name)?;
    }

    let mut res_reader = device.get_resoure(&target_object_info.content_object)?;
    report_copying_start(&src_file_info);

    // 创建目标文件
    dest.create_file(
        dest_name,
        &mut res_reader,
        src_file_info.data_size,
        &target_object_info.time_created,
        &target_object_info.time_modified,
    )?;

    dest.retain(dest_name);
    report_copying_end();
    Ok(())
}

// 复制文件夹的逻辑
fn copy_folder(
    device: &Device,
    dest: &mut impl FolderOperate,
    dest_is_parent_folder: bool,
    target_object_info: &ContentObjectInfo,
    dest_name: &str,
    recursive: bool,
    mirror: bool
) -> Result<(), Box<dyn std::error::Error>> {
    let new_dest_ref: &mut impl FolderOperate;

    // 如果目标是父文件夹，则在目标中创建一个新文件夹
    if dest_is_parent_folder {
        let mut new_dest = dest.open_or_create_folder(dest_name, |_| {}, report_creating_new_folder)?;
        dest.retain(dest_name);
        new_dest_ref = new_dest.as_mut();
    } else {
        // 目标路径是现有文件夹，直接使用目标文件夹
        new_dest_ref = dest;
    }

    // 如果是递归复制
    if recursive {
        let mut iter = device.get_object_iterator(&target_object_info.content_object)?;
        while let Some(content_object) = iter.next()? {
            let content_object_info = device.get_object_info(content_object)?;
            copy_iter(
                device,
                new_dest_ref,
                true, // dest_is_parent_folder
                &content_object_info,
                &content_object_info.name,
                recursive,
                mirror,
            )?;
        }

        // 如果启用了镜像模式，多余的文件和文件夹将被删除
        if mirror {
            new_dest_ref.delete_unretained(report_delete_file, report_delete_folder)?;
        }
    }

    Ok(())
}
