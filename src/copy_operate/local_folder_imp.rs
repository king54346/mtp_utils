use std::collections::HashSet;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::common::file_reader::FileReader;
use crate::common::time_transfer::string_to_system_time;
use crate::copy_operate::folder_operate::FolderOperate;

use super::file_info::FileInfo;



pub struct LocalFolder {
    folder_path: PathBuf,
    retained: HashSet<String>,
}

impl LocalFolder {
    pub fn new(folder_path: PathBuf) -> LocalFolder {
        let retained = HashSet::<String>::new();
        LocalFolder {
            folder_path,
            retained,
        }
    }
}

impl FolderOperate for LocalFolder {
    fn get_file_info(
        &mut self,
        name: &str,
    ) -> Result<Option<FileInfo>, Box<dyn std::error::Error>> {
        let path_buf = Path::new(&self.folder_path).join(name);
        if let Ok(metadata) = path_buf.metadata() {
            Ok(Some(FileInfo::from_metadata(&metadata, name)?))
        } else {
            Ok(None)
        }
    }

    fn create_file(
        &mut self,
        name: &str,
        reader: &mut impl FileReader,
        #[allow(unused_variables)] size: u64,
        created: &Option<String>,
        modified: &Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path_buf = Path::new(&self.folder_path).join(name);

        let copy_result;
        {
            // a scope in which a File object lives
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path_buf)?;

            copy_result = copy_to_file(reader, &mut file);
        }

        if let Err(err) = copy_result {
            let _ = std::fs::remove_file(&path_buf);
            return Err(err);
        }
        // created 如果不为空，将创建时间 转换成 SystemTime
        let created = if let Some(created) = created {
                Some(string_to_system_time(created)?)
            } else {
                None
            };
        // modified 如果不为空，将修改时间设置为 modified
        let modified = if let Some(modified) = modified {
                Some(string_to_system_time(modified)?)
            } else {
                None
            };

        set_file_times(&path_buf, &created, &modified)?;

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
        let path_buf = Path::new(&self.folder_path).join(name);

        if path_buf.exists() {
            if !path_buf.is_dir() {
                return Err(format!("cannot open a folder: {}", path_buf.to_str().unwrap()).into());
            }
            before_open(name);
        } else {
            before_create(name);
            std::fs::create_dir_all(&path_buf)?;
        }
        Ok(Box::new(LocalFolder::new(path_buf)))
    }

    fn delete_file_or_folder(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path_buf = Path::new(&self.folder_path).join(name);

        if path_buf.is_file() {
            std::fs::remove_file(path_buf)?;
        } else if path_buf.is_dir() {
            std::fs::remove_dir_all(path_buf)?;
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
        // 遍历文件夹中的所有文件和子文件夹
        for entry_result in self.folder_path.read_dir()? {
            let entry = entry_result?;
            if let Some(name) = entry.file_name().to_str() {
                let metadata = entry.metadata()?;
                let file_info = FileInfo::from_metadata(&metadata, name)?;
                // 跳过隐藏文件和系统文件
                if !file_info.is_hidden && !file_info.is_system {
                    if !self.retained.contains(name) {
                        if file_info.is_folder {
                            before_delete_folder(name);
                        } else {
                            before_delete_file(name);
                        }

                        self.delete_file_or_folder(name)?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn copy_to_file(
    reader: &mut impl FileReader,
    file: &mut File,
) -> Result<(), Box<dyn std::error::Error>> {
    while let Some(bytes) = reader.seek(reader.buffer_size())? {
        file.write_all(bytes)?;
    }
    Ok(())
}

fn set_file_times(path: &Path, created: &Option<SystemTime>, modified: &Option<SystemTime>) -> std::io::Result<()> {
    // 打开文件以确保它存在
    let _file = File::open(path)?;

    // 获取当前的文件元数据
    let mut metadata = fs::metadata(path)?;

    // 设置创建时间
    if let Some(created_time) = created {
        // Rust 标准库不直接支持设置创建时间，通常需要使用外部库或系统调用
        // 例如，在 Windows 上，可以使用 WinAPI
    }

    // 设置修改时间
    if let Some(modified_time) = modified {
        // Rust 标准库不直接支持设置修改时间，通常需要使用外部库或系统调用
        // 例如，在 Windows 上，可以使用 WinAPI
    }

    Ok(())
}

// fn naive_date_time_to_file_time(
//     dt_opt: &Option<String>,
// ) -> Result<Option<FILETIME>, Box<dyn std::error::Error>> {
//     if dt_opt.is_none() {
//         return Ok(None);
//     }
//
//     let dt = dt_opt.unwrap();
//     let dt_local = Local.from_local_datetime(&dt).latest();
//     if dt_local.is_none() {
//         return Err(format!("Cannot convert to a local time. : {}", dt.to_string()).into());
//     }
//     let dt_utc = dt_local.unwrap().with_timezone(&Utc);
//
//     let st = SYSTEMTIME {
//         wYear: dt_utc.year() as u16,
//         wMonth: dt_utc.month() as u16,
//         wDayOfWeek: dt_utc.weekday().num_days_from_sunday() as u16,
//         wDay: dt_utc.day() as u16,
//         wHour: dt_utc.hour() as u16,
//         wMinute: dt_utc.minute() as u16,
//         wSecond: dt_utc.second() as u16,
//         wMilliseconds: dt_utc.timestamp_subsec_millis() as u16,
//     };
//
//     let mut ft = FILETIME {
//         dwHighDateTime: 0,
//         dwLowDateTime: 0,
//     };
//
//     let r = unsafe { SystemTimeToFileTime(&st, &mut ft) };
//     if r.as_bool() {
//         Ok(Some(ft))
//     } else {
//         Err("SystemTimeToFileTime failed.".into())
//     }
// }

#[cfg(test)]
mod local_destination_folder_tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn test_get_file_info_folder() -> Result<(), Box<dyn std::error::Error>> {
        let tempdir = tempfile::tempdir()?;
        let path = tempdir.path().join("foo bar");
        std::fs::create_dir(path)?;

        let mut ldf = LocalFolder::new(PathBuf::from(tempdir.path()));
        let file_info_opt = ldf.get_file_info(&"foo bar".to_string())?;

        assert!(file_info_opt.is_some());
        let file_info = file_info_opt.unwrap();
        assert_eq!(file_info.name, "foo bar");
        assert_eq!(file_info.data_size, 0u64);
        assert_eq!(file_info.is_folder, true);
        assert_eq!(file_info.is_hidden, false);
        assert_eq!(file_info.is_system, false);
        assert_eq!(file_info.can_delete, true);
        assert!(file_info.time_created.is_some());
        assert!(file_info.time_modified.is_some());
        // let now = Local::now().naive_local();
        // let created_duration_ms = now
        //     .signed_duration_since(file_info.time_created.unwrap())
        //     .num_milliseconds();
        // let modified_duration_ms = now
        //     .signed_duration_since(file_info.time_modified.unwrap())
        //     .num_milliseconds();
        // assert!(0 <= created_duration_ms);
        // assert!(created_duration_ms < 500);
        // assert!(0 <= modified_duration_ms);
        // assert!(modified_duration_ms < 500);
        //
        Ok(())
    }

    #[test]
    fn test_get_file_info_file() -> Result<(), Box<dyn std::error::Error>> {
        let tempdir = tempfile::tempdir()?;
        let path = tempdir.path().join("foo bar");
        std::fs::write(&path, "abc")?;

        let mut ldf = LocalFolder::new(PathBuf::from(tempdir.path()));
        let file_info_opt = ldf.get_file_info(&"foo bar".to_string())?;

        assert!(file_info_opt.is_some());
        let file_info = file_info_opt.unwrap();
        assert_eq!(file_info.name, "foo bar");
        assert_eq!(file_info.data_size, 3u64);
        assert_eq!(file_info.is_folder, false);
        assert_eq!(file_info.is_hidden, false);
        assert_eq!(file_info.is_system, false);
        assert_eq!(file_info.can_delete, true);
        assert!(file_info.time_created.is_some());
        assert!(file_info.time_modified.is_some());
        // let now = Local::now().naive_local();
        // let created_duration_ms = now
        //     .signed_duration_since(file_info.time_created.unwrap())
        //     .num_milliseconds();
        // let modified_duration_ms = now
        //     .signed_duration_since(file_info.time_modified.unwrap())
        //     .num_milliseconds();
        // assert!(0 <= created_duration_ms);
        // assert!(created_duration_ms < 500);
        // assert!(0 <= modified_duration_ms);
        // assert!(modified_duration_ms < 500);

        Ok(())
    }

    struct TestingFileReader {
        n: u8,
        buf: [u8; 10],
        count: u32,
    }

    impl TestingFileReader {
        fn new() -> TestingFileReader {
            TestingFileReader {
                n: 0,
                buf: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                count: 0,
            }
        }
    }

    impl FileReader for TestingFileReader {
        fn buffer_size(&self) -> u32 {
            self.buf.len() as u32
        }

        fn seek(&mut self, _max_size: u32) -> Result<Option<&[u8]>, Box<dyn std::error::Error>> {
            if self.count >= 3 {
                Ok(None)
            } else {
                for i in 0..self.buf.len() {
                    self.n = self.n.wrapping_add(1);
                    self.buf[i] = self.n;
                }
                self.count += 1;
                Ok(Some(&self.buf))
            }
        }
    }

    #[test_case(false; "new file")]
    #[test_case(true; "overwrite existing file")]
    fn test_create_file(overwrite: bool) -> Result<(), Box<dyn std::error::Error>> {
        let tempdir = tempfile::tempdir()?;
        let path = tempdir.path().join("foo bar");

        if overwrite {
            std::fs::write(&path, "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")?;
        }

        // let created = Some(NaiveDateTime::new(
        //     NaiveDate::from_ymd(2001, 2, 3),
        //     NaiveTime::from_hms_milli(4, 5, 6, 789),
        // ));
        // let modified = Some(NaiveDateTime::new(
        //     NaiveDate::from_ymd(2002, 3, 4),
        //     NaiveTime::from_hms_milli(5, 6, 7, 890),
        // ));

        let file_size = path.metadata()?.len();
        let mut reader = TestingFileReader::new();
        let mut ldf = LocalFolder::new(PathBuf::from(tempdir.path()));
        ldf.create_file(
            &"foo bar".to_string(),
            &mut reader,
            file_size,
            &None,
            &None,
        )?;

        let metadata = path.metadata()?;
        assert!(metadata.is_file());
        // let file_created_dt = DateTime::<Local>::from(metadata.created()?).naive_local();
        // let file_modified_dt = DateTime::<Local>::from(metadata.modified()?).naive_local();
        // assert_eq!(file_created_dt, created.unwrap());
        // assert_eq!(file_modified_dt, modified.unwrap());

        let actual_content = std::fs::read(&path)?;
        let expected_content_array: [u8; 30] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, //
            11, 12, 13, 14, 15, 16, 17, 18, 19, 20, //
            21, 22, 23, 24, 25, 26, 27, 28, 29, 30, //
        ];
        let expected_content: Vec<u8> = expected_content_array.into();
        assert_eq!(actual_content, expected_content);

        Ok(())
    }

    #[test_case(false; "create new folder")]
    #[test_case(true; "open existing folder")]
    fn test_open_or_create_folder(open_existing: bool) -> Result<(), Box<dyn std::error::Error>> {
        let tempdir = tempfile::tempdir()?;
        let path = tempdir.path().join("foo bar");

        if open_existing {
            std::fs::create_dir(&path)?;
        }

        let mut ldf = LocalFolder::new(PathBuf::from(tempdir.path()));
        let mut before_open_called = false;
        let mut before_create_called = false;
        let ldf2 = ldf.open_or_create_folder(
            &"foo bar".to_string(),
            |_name| before_open_called = true,
            |_name| before_create_called = true,
        )?;
        assert_eq!(&ldf2.folder_path, &path);
        assert_eq!(before_open_called, open_existing);
        assert_eq!(before_create_called, !open_existing);

        Ok(())
    }
}
