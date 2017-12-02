extern crate libc;
#[macro_use]
extern crate evdev;

use evdev::{uinput, raw, data};

fn type_key(dev: &mut uinput::Device, k: data::Key) {
    dev.write(data::KEY, k as u16, 1).expect("write");
    dev.write(data::SYNCHRONIZATION, data::SYN_REPORT as u16, 0).expect("write");
    std::thread::sleep(std::time::Duration::from_millis(50));
    dev.write(data::KEY, k as u16, 0).expect("write");
    dev.write(data::SYNCHRONIZATION, data::SYN_REPORT as u16, 0).expect("write");
}

fn main() {
    let mut conf = raw::uinput_setup::default();
    conf.set_name("Devicey McDeviceFace").expect("set_name");
    conf.id.bustype = 0x16;
    conf.id.vendor = 69;
    conf.id.product = 69;
    let mut d = uinput::Device::open(&std::path::Path::new("/dev/uinput"), conf, |fd| {
        uinput_ioctl!(ui_set_evbit(fd, data::KEY.number()))?;
        uinput_ioctl!(ui_set_keybit(fd, data::KEY_1 as i32))?;
        uinput_ioctl!(ui_set_keybit(fd, data::KEY_2 as i32))?;
        uinput_ioctl!(ui_set_keybit(fd, data::KEY_3 as i32))?;
        uinput_ioctl!(ui_set_keybit(fd, data::KEY_4 as i32))?;
        uinput_ioctl!(ui_set_keybit(fd, data::KEY_5 as i32))?;
        Ok(())
    }).expect("open");
    std::thread::sleep(std::time::Duration::from_secs(1)); // let clients initialize
    type_key(&mut d, data::KEY_1);
    type_key(&mut d, data::KEY_2);
    type_key(&mut d, data::KEY_3);
    type_key(&mut d, data::KEY_4);
    type_key(&mut d, data::KEY_5);
    std::thread::sleep(std::time::Duration::from_secs(1)); // let clients read
}
