use crate::common::file_reader::FileReader;
use super::file_info::FileInfo;


// 文件夹操作接口
pub trait FolderOperate {
    // 获取文件的信息
    fn get_file_info(&mut self, name: &str) -> Result<Option<FileInfo>, Box<dyn std::error::Error>>;
    // 创建文件
    fn create_file(
        &mut self,
        name: &str,
        reader: &mut impl FileReader,
        size: u64,
        created: &Option<String>,
        modified: &Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>>;
    // 打开或创建文件夹,before_open为打开文件夹前的回调函数,before_create为创建文件夹前的回调函数
    fn open_or_create_folder<FBeforeOpen, FBeforeCreate>(
        &mut self,
        name: &str,
        before_open: FBeforeOpen,
        before_create: FBeforeCreate,
    ) -> Result<Box<Self>, Box<dyn std::error::Error>>
    where
        FBeforeOpen: FnOnce(&str),
        FBeforeCreate: FnOnce(&str);
    // 删除文件或文件夹
    fn delete_file_or_folder(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>>;
    // 标记一个文件或文件夹为保留,delete_unretained配合
    fn retain(&mut self, name: &str);
    // 删除未保留的文件或文件夹,用于镜像文件模式,before_delete_file为删除文件前的回调函数,before_delete_folder为删除文件夹前的回调函数
    fn delete_unretained<FBeforeDeleteFile, FBeforeDeleteFolder>(
        &mut self,
        before_delete_file: FBeforeDeleteFile,
        before_delete_folder: FBeforeDeleteFolder,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        FBeforeDeleteFile: Fn(&str),
        FBeforeDeleteFolder: Fn(&str);
}
