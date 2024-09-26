use std::fs::File;
use std::io::Read;
use crate::common::file_reader::FileReader;


pub struct LocalFileReader {
    file: File,
    buf: Vec<u8>,
}

impl LocalFileReader {
    pub fn new(file: File) -> LocalFileReader {
        let mut buf = Vec::<u8>::new();
        buf.resize(32768, 0);
        LocalFileReader { file, buf }
    }
}

impl FileReader for LocalFileReader {
    // 读取文件时的缓冲区大小
    fn buffer_size(&self) -> u32 {
        self.buf.len() as u32
    }
    // 读取文件内容 max_size 为读取的最大字节数
    fn seek(&mut self, max_size: u32) -> Result<Option<&[u8]>, Box<dyn std::error::Error>> {
        // 重新调整缓冲区大小
        self.buf.resize(max_size as usize, 0);
        let len = self.file.read(self.buf.as_mut_slice())?;
        if len > 0 {
            Ok(Some(&self.buf.as_slice()[..len]))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::io::Seek;
    use std::io::SeekFrom;
    use tempfile::tempfile;

    fn create_temp_file_with_content(content: &[u8]) -> File {
        let mut file = tempfile().unwrap();
        file.write_all(content).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        file
    }

    #[test]
    fn new_creates_local_file_reader_with_correct_buffer_size() {
        let file = create_temp_file_with_content(b"");
        let reader = LocalFileReader::new(file);
        assert_eq!(reader.buffer_size(), 32768);
    }

    #[test]
    fn next_reads_data_into_buffer() {
        let file = create_temp_file_with_content(b"Hello, world!");
        let mut reader = LocalFileReader::new(file);
        let result = reader.seek(5).unwrap();
        assert_eq!(result, Some(&b"Hello"[..]));
    }

    #[test]
    fn next_returns_none_when_no_more_data() {
        let file = create_temp_file_with_content(b"");
        let mut reader = LocalFileReader::new(file);
        let result = reader.seek(5).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn next_resizes_buffer_correctly() {
        let file = create_temp_file_with_content(b"Hello, world!");
        let mut reader = LocalFileReader::new(file);
        reader.seek(5).unwrap();
        assert_eq!(reader.buffer_size(), 5);
    }

    #[test]
    fn next_handles_partial_reads() {
        let file = create_temp_file_with_content(b"Hello");
        let mut reader = LocalFileReader::new(file);
        let result = reader.seek(10).unwrap();
        assert_eq!(result, Some(&b"Hello"[..]));
    }
}
