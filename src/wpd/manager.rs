use std::fmt::Debug;
use windows::core::{Error, PWSTR};
use windows::Win32::Devices::PortableDevices::{IPortableDeviceManager, PortableDeviceManager};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

pub struct Manager {
    manager: IPortableDeviceManager,
}

#[derive(Debug)]
pub struct DeviceInfo {
    pub id: PWSTR,
    pub name: String,
}

impl Manager {
    pub fn get_portable_device_manager() -> Result<Manager, Error> {
        let manager: IPortableDeviceManager = unsafe { CoCreateInstance(
            &PortableDeviceManager,
            None,
            CLSCTX_ALL,
        )?};
        Ok(Manager { manager })
    }

    pub fn get_device_iterator<'a>(&'a self) -> Result<DeviceInfoIterator<'a>, Error> {
        // get number of devices
        let mut device_id_count = 0u32;
        unsafe {
            self.manager
                .GetDevices(std::ptr::null_mut(), &mut device_id_count)
                .ok();
        }

        // get device ids
        let mut device_ids = Vec::<PWSTR>::new();
        device_ids.resize(device_id_count as usize, PWSTR::null());
        unsafe {
            self.manager
                .GetDevices(device_ids.as_mut_ptr(), &mut device_id_count)
                .ok();
        }

        Ok(DeviceInfoIterator::new(
            &self.manager,
            device_ids,
        ))
    }
}

pub struct DeviceInfoIterator<'a> {
    manager: &'a IPortableDeviceManager,
    device_ids: Vec<PWSTR>,
}

impl<'a> DeviceInfoIterator<'a> {
    fn new(
        manager: &'a IPortableDeviceManager,
        mut device_ids: Vec<PWSTR>,
    ) -> DeviceInfoIterator<'a> {
        device_ids.reverse(); // for moving item out by pop()
        DeviceInfoIterator::<'a> {
            manager,
            device_ids,
        }
    }

    pub fn next(&mut self) -> Result<Option<DeviceInfo>, Error> {
        let device_id = match self.device_ids.pop() {
            Some(id) => id,
            None => return Ok(None),
        };

        // get name length
        let mut name_buf_len = 0u32;
        unsafe {
            self.manager
                .GetDeviceFriendlyName(
                    device_id,
                    PWSTR::null(),
                    &mut name_buf_len as *mut u32,
                )
                .ok();
        }

        // get name
        let mut name_buf: Vec<u16> = Vec::with_capacity(name_buf_len as usize);
        let name:String;
        unsafe {
            self.manager
                .GetDeviceFriendlyName(
                    device_id,
                    PWSTR(name_buf.as_mut_ptr()),
                    &mut name_buf_len as *mut u32,
                )
                .ok();
            name_buf.set_len(name_buf_len as usize);
            name = String::from_utf16_lossy(&name_buf);
        }



        Ok(Some(DeviceInfo {
            id: device_id,
            name,
        }))
    }
}

