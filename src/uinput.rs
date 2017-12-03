use std::{slice, mem};
use std::path::Path;
use std::ffi::CString;
use std::os::unix::io::*;
use std::os::unix::ffi::*;
use nix::{Error, unistd};
pub use nix::Errno;
use libc;

use data;
use raw::*;

#[macro_export]
macro_rules! uinput_ioctl {
    ($name:ident($($arg:expr),+)) => {{
        unsafe { ::raw::$name($($arg,)+) }.and_then(::uinput::Errno::result)
    }}
}

pub struct Device {
    fd: RawFd,
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            let _ = ui_dev_destroy(self.fd);
            // Linux close(2) can fail, but there is nothing to do if it does.
            libc::close(self.fd);
        }
    }
}

impl Device {
    pub fn fd(&self) -> RawFd {
        self.fd
    }

    pub fn write_raw(&mut self, event: input_event) -> Result<(), Error> {
        unistd::write(self.fd, unsafe { slice::from_raw_parts(&event as *const _ as *const u8, mem::size_of_val(&event)) })?;
        Ok(())
    }

    pub fn write(&mut self, _type: data::Types, code: u16, value: i32) -> Result<(), Error> {
        let mut event = input_event::default();
        event._type = _type.number();
        event.code = code;
        event.value = value;
        self.write_raw(event)
    }
}

pub struct Builder {
    fd: RawFd,
}

impl Builder {
    pub fn fd(&self) -> RawFd {
        self.fd
    }

    pub fn new(path: &AsRef<Path>) -> Result<Builder, Error> {
        let cstr = match CString::new(path.as_ref().as_os_str().as_bytes()) {
            Ok(s) => s,
            Err(_) => return Err(Error::InvalidPath),
        };

        Ok(Builder {
            fd: Errno::result(unsafe { libc::open(cstr.as_ptr(), libc::O_RDWR | libc::O_CLOEXEC, 0) })?,
        })
    }

    pub fn setup(self, setup: uinput_setup) -> Result<Device, Error> {
        uinput_ioctl!(ui_dev_setup(self.fd, &setup))?;
        uinput_ioctl!(ui_dev_create(self.fd))?;
        Ok(Device {
            fd: self.fd
        })
    }
}
