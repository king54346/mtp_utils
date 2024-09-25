use windows::core::{Error, Interface};
use windows::Win32::Devices::PortableDevices::IPortableDeviceDataStream;
use windows::Win32::Foundation::STG_E_REVERTED;
use windows::Win32::System::Com::{IStream, STGC_DEFAULT};
use crate::glob::file_reader::FileReader;
use crate::wpd::utils::IDStr;
use super::{device::ContentObject, utils::WStrPtr};

// wpd 文件数据流读取器

pub struct ResourceReader {
    stream: IStream,
    buffer: Vec<u8>,
}

impl FileReader for ResourceReader {
    fn buffer_size(&self) -> u32 {
        self.buffer.len() as u32
    }

    fn seek(&mut self, max_size: u32) -> Result<Option<&[u8]>, Box<dyn std::error::Error>> {
        Ok(self.next(max_size)?)
    }
}

impl ResourceReader {
    pub fn new(stream: IStream, buff_size: u32) -> ResourceReader {
        let mut buffer = Vec::<u8>::with_capacity(buff_size as usize);
        buffer.resize(buff_size as usize, 0);
        ResourceReader { stream, buffer }
    }

    pub fn next(&mut self, max_size: u32) -> Result<Option<&[u8]>, Error> {
        let available_buffer_size = std::cmp::min(self.buffer.len() as u32, max_size);
        let read: Option<*mut u32> = None;
        unsafe {
            self.stream.Read(
                self.buffer.as_mut_ptr().cast(),
                available_buffer_size,
                read,
            )
                .ok()?;
        }
        match read {
            None => {
                return Ok(None);
            }
            Some(v) => unsafe {
                let read = *v;
                if read == 0 {
                    Ok(None)
                } else {
                    Ok(Some(&self.buffer[..read as usize]))
                }
            }
        }
    }

    pub fn get_optimized_buffer_size(&self) -> u32 {
        self.buffer.len() as u32
    }
}


// wpd 文件数据流写入器

pub struct ResourceWriter {
    buff_size: u32,
    stream: IStream,
    committed: bool,
}

impl ResourceWriter {
    pub fn new(stream: IStream, buff_size: u32) -> ResourceWriter {
        ResourceWriter {
            buff_size,
            stream,
            committed: false,
        }
    }

    pub fn get_buffer_size(&self) -> u32 {
        self.buff_size
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        let data_len = data.len() as u32;
        let mut data_offset: u32 = 0;
        while data_offset < data_len {
            let write_len = std::cmp::min(data_len - data_offset, self.buff_size as u32);
            let mut bytes_written: u32 = 0;
            let pcbwritten = &mut bytes_written as *mut u32;
            unsafe {
                self.stream
                    .Write(
                        data.as_ptr().offset(data_offset as isize) as *const std::ffi::c_void,
                        write_len,
                        Some(pcbwritten),
                    )
                    .ok()?;
            }
            ;
            data_offset += bytes_written;
        }
        Ok(())
    }

    pub fn commit(&mut self) -> Result<ContentObject, Error> {
        self.committed = true;
        unsafe {
            self.stream.Commit(STGC_DEFAULT)?;
        }

        let data_stream: IPortableDeviceDataStream = self.stream.cast()?;

        let object_id = unsafe{data_stream.GetObjectID()?};

        Ok(ContentObject::new(IDStr::from(object_id)))
    }
}
