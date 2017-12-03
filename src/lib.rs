//! Linux event device handling.
//!
//! The Linux kernel's "evdev" subsystem exposes input devices to userspace in a generic,
//! consistent way. I'll try to explain the device model as completely as possible. The upstream
//! kernel documentation is split across two files:
//!
//! - https://www.kernel.org/doc/Documentation/input/event-codes.txt
//! - https://www.kernel.org/doc/Documentation/input/multi-touch-protocol.txt
//!
//! Devices can expose a few different kinds of events, specified by the `Types` bitflag. Each
//! event type (except for RELATIVE and SYNCHRONIZATION) also has some associated state. See the documentation for
//! `Types` on what each type corresponds to.
//!
//! This state can be queried. For example, the `DeviceState::led_vals` field will tell you which
//! LEDs are currently lit on the device. This state is not automatically synchronized with the
//! kernel. However, as the application reads events, this state will be updated if the event is
//! newer than the state timestamp (maintained internally).  Additionally, you can call
//! `Device::sync_state` to explicitly synchronize with the kernel state.
//!
//! As the state changes, the kernel will write events into a ring buffer. The application can read
//! from this ring buffer, thus retrieving events. However, if the ring buffer becomes full, the
//! kernel will *drop* every event in the ring buffer and leave an event telling userspace that it
//! did so. At this point, if the application were using the events it received to update its
//! internal idea of what state the hardware device is in, it will be wrong: it is missing some
//! events. This library tries to ease that pain, but it is best-effort. Events can never be
//! recovered once lost. For example, if a switch is toggled twice, there will be two switch events
//! in the buffer. However if the kernel needs to drop events, when the device goes to synchronize
//! state with the kernel, only one (or zero, if the switch is in the same state as it was before
//! the sync) switch events will be emulated.
//!
//! It is recommended that you dedicate a thread to processing input events, or use epoll with the
//! fd returned by `Device::fd` to process events when they are ready.

#![cfg(any(unix, target_os = "android"))]
#![allow(non_camel_case_types)]

#[macro_use]
extern crate bitflags;
extern crate strum;
#[macro_use]
extern crate strum_macros;
#[macro_use]
extern crate nix;
extern crate libc;
extern crate fixedbitset;
extern crate num;

#[macro_use]
pub mod raw;
pub mod data;
pub mod uinput;

use std::os::unix::io::*;
use std::os::unix::ffi::*;
use std::path::Path;
use std::ffi::CString;
use std::mem::{size_of, transmute};

use nix::Error;

use raw::*;
use data::*;

#[derive(Clone)]
pub struct DeviceState {
    /// The state corresponds to kernel state at this timestamp.
    pub timestamp: libc::timeval,
    /// Set = key pressed
    pub key_vals: FixedBitSet,
    pub abs_vals: Vec<input_absinfo>,
    /// Set = switch enabled (closed)
    pub switch_vals: FixedBitSet,
    /// Set = LED lit
    pub led_vals: FixedBitSet,
}

pub struct Device {
    fd: RawFd,
    ty: Types,
    name: CString,
    phys: Option<CString>,
    uniq: Option<CString>,
    id: input_id,
    props: Props,
    driver_version: (u8, u8, u8),
    key_bits: FixedBitSet,
    rel: RelativeAxis,
    abs: AbsoluteAxis,
    switch: Switch,
    led: Led,
    misc: Misc,
    ff: FixedBitSet,
    ff_stat: FFStatus,
    rep: Repeat,
    snd: Sound,
    pending_events: Vec<input_event>,
    clock: libc::c_int,
    // pending_events[last_seen..] is the events that have occurred since the last sync.
    last_seen: usize,
    state: DeviceState,
}

impl std::fmt::Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut ds = f.debug_struct("Device");
        ds.field("name", &self.name).field("fd", &self.fd).field("ty", &self.ty);
        if let Some(ref phys) = self.phys {
            ds.field("phys", phys);
        }
        if let Some(ref uniq) = self.uniq {
            ds.field("uniq", uniq);
        }
        ds.field("id", &self.id)
          .field("id", &self.id)
          .field("props", &self.props)
          .field("driver_version", &self.driver_version);
        if self.ty.contains(SYNCHRONIZATION) {

        }
        if self.ty.contains(KEY) {
            ds.field("key_bits", &self.key_bits)
              .field("key_vals", &self.state.key_vals);
        }
        if self.ty.contains(RELATIVE) {
            ds.field("rel", &self.rel);
        }
        if self.ty.contains(ABSOLUTE) {
            ds.field("abs", &self.abs);
            for idx in 0..0x3f {
                let abs = 1 << idx;
                // ignore multitouch, we'll handle that later.
                if (self.abs.bits() & abs) == 1 {
                    // eugh.
                    ds.field(&format!("abs_{:x}", idx), &self.state.abs_vals[idx as usize]);
                }
            }
        }
        if self.ty.contains(MISC) {

        }
        if self.ty.contains(SWITCH) {
            ds.field("switch", &self.switch)
              .field("switch_vals", &self.state.switch_vals);
        }
        if self.ty.contains(LED) {
            ds.field("led", &self.led)
              .field("led_vals", &self.state.led_vals);
        }
        if self.ty.contains(SOUND) {
            ds.field("snd", &self.snd);
        }
        if self.ty.contains(REPEAT) {
            ds.field("rep", &self.rep);
        }
        if self.ty.contains(FORCEFEEDBACK) {
            ds.field("ff", &self.ff);
        }
        if self.ty.contains(POWER) {
        }
        if self.ty.contains(FORCEFEEDBACKSTATUS) {
            ds.field("ff_stat", &self.ff_stat);
        }
        ds.finish()
    }
}

fn bus_name(x: u16) -> &'static str {
    match x {
        0x1 => "PCI",
        0x2 => "ISA Plug 'n Play",
        0x3 => "USB",
        0x4 => "HIL",
        0x5 => "Bluetooth",
        0x6 => "Virtual",
        0x10 => "ISA",
        0x11 => "i8042",
        0x12 => "XTKBD",
        0x13 => "RS232",
        0x14 => "Gameport",
        0x15 => "Parallel Port",
        0x16 => "Amiga",
        0x17 => "ADB",
        0x18 => "I2C",
        0x19 => "Host",
        0x1A => "GSC",
        0x1B => "Atari",
        0x1C => "SPI",
        _ => "Unknown",
    }
}

impl std::fmt::Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        try!(writeln!(f, "{:?}", self.name));
        try!(writeln!(f, "  Driver version: {}.{}.{}", self.driver_version.0, self.driver_version.1, self.driver_version.2));
        if let Some(ref phys) = self.phys {
            try!(writeln!(f, "  Physical address: {:?}", phys));
        }
        if let Some(ref uniq) = self.uniq {
            try!(writeln!(f, "  Unique name: {:?}", uniq));
        }

        try!(writeln!(f, "  Bus: {}", bus_name(self.id.bustype)));
        try!(writeln!(f, "  Vendor: 0x{:x}", self.id.vendor));
        try!(writeln!(f, "  Product: 0x{:x}", self.id.product));
        try!(writeln!(f, "  Version: 0x{:x}", self.id.version));
        try!(writeln!(f, "  Properties: {:?}", self.props));

        if self.ty.contains(SYNCHRONIZATION) {

        }

        if self.ty.contains(KEY) {
            try!(writeln!(f, "  Keys supported:"));
            for key_idx in 0..self.key_bits.len() {
                if self.key_bits.contains(key_idx) {
                    // Cross our fingers... (what did this mean?)
                    try!(writeln!(f, "    {:?} ({}index {})",
                                 unsafe { std::mem::transmute::<_, Key>(key_idx as libc::c_int) },
                                 if self.state.key_vals.contains(key_idx) { "pressed, " } else { "" },
                                 key_idx));
                }
            }
        }
        if self.ty.contains(RELATIVE) {
            try!(writeln!(f, "  Relative Axes: {:?}", self.rel));
        }
        if self.ty.contains(ABSOLUTE) {
            try!(writeln!(f, "  Absolute Axes:"));
            for idx in 0..0x3f {
                let abs = 1<< idx;
                if self.abs.bits() & abs != 0 {
                    // FIXME: abs val Debug is gross
                    try!(writeln!(f, "    {:?} ({:?}, index {})",
                         AbsoluteAxis::from_bits(abs).unwrap(),
                         self.state.abs_vals[idx as usize],
                         idx));
                }
            }
        }
        if self.ty.contains(MISC) {
            try!(writeln!(f, "  Miscellaneous capabilities: {:?}", self.misc));
        }
        if self.ty.contains(SWITCH) {
            try!(writeln!(f, "  Switches:"));
            for idx in 0..0xf {
                let sw = 1 << idx;
                if sw < SW_MAX.bits() && self.switch.bits() & sw == 1 {
                    try!(writeln!(f, "    {:?} ({:?}, index {})",
                         Switch::from_bits(sw).unwrap(),
                         self.state.switch_vals[idx as usize],
                         idx));
                }
            }
        }
        if self.ty.contains(LED) {
            try!(writeln!(f, "  LEDs:"));
            for idx in 0..0xf {
                let led = 1 << idx;
                if led < LED_MAX.bits() && self.led.bits() & led == 1 {
                    try!(writeln!(f, "    {:?} ({:?}, index {})",
                         Led::from_bits(led).unwrap(),
                         self.state.led_vals[idx as usize],
                         idx));
                }
            }
        }
        if self.ty.contains(SOUND) {
            try!(writeln!(f, "  Sound: {:?}", self.snd));
        }
        if self.ty.contains(REPEAT) {
            try!(writeln!(f, "  Repeats: {:?}", self.rep));
        }
        if self.ty.contains(FORCEFEEDBACK) {
            try!(writeln!(f, "  Force Feedback supported"));
        }
        if self.ty.contains(POWER) {
            try!(writeln!(f, "  Power supported"));
        }
        if self.ty.contains(FORCEFEEDBACKSTATUS) {
            try!(writeln!(f, "  Force Feedback status supported"));
        }
        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        // Linux close(2) can fail, but there is nothing to do if it does.
        unsafe { libc::close(self.fd); }
    }
}

impl Device {
    pub fn fd(&self) -> RawFd {
        self.fd
    }

    pub fn events_supported(&self) -> Types {
        self.ty
    }

    pub fn name(&self) -> &CString {
        &self.name
    }

    pub fn physical_path(&self) -> &Option<CString> {
        &self.phys
    }

    pub fn unique_name(&self) -> &Option<CString> {
        &self.uniq
    }

    pub fn input_id(&self) -> input_id {
        self.id
    }

    pub fn properties(&self) -> Props {
        self.props
    }

    pub fn driver_version(&self) -> (u8, u8, u8) {
        self.driver_version
    }

    pub fn keys_supported(&self) -> &FixedBitSet {
        &self.key_bits
    }

    pub fn relative_axes_supported(&self) -> RelativeAxis {
        self.rel
    }

    pub fn absolute_axes_supported(&self) -> AbsoluteAxis {
        self.abs
    }

    pub fn switches_supported(&self) -> Switch {
        self.switch
    }

    pub fn leds_supported(&self) -> Led {
        self.led
    }

    pub fn misc_properties(&self) -> Misc {
        self.misc
    }

    pub fn repeats_supported(&self) -> Repeat {
        self.rep
    }

    pub fn sounds_supported(&self) -> Sound {
        self.snd
    }

    pub fn state(&self) -> &DeviceState {
        &self.state
    }

    pub fn open(path: &AsRef<Path>) -> Result<Device, Error> {
        let cstr = match CString::new(path.as_ref().as_os_str().as_bytes()) {
            Ok(s) => s,
            Err(_) => return Err(Error::InvalidPath),
        };
        // FIXME: only need for writing is for setting LED values. re-evaluate always using RDWR
        // later.
        let fd = unsafe { libc::open(cstr.as_ptr(), libc::O_NONBLOCK | libc::O_RDWR | libc::O_CLOEXEC, 0) };
        if fd == -1 {
            return Err(Error::from_errno(::nix::Errno::last()));
        }

        let mut dev = Device {
            fd: fd,
            ty: Types::empty(),
            name: unsafe { CString::from_vec_unchecked(Vec::new()) },
            phys: None,
            uniq: None,
            id: unsafe { std::mem::zeroed() },
            props: Props::empty(),
            driver_version: (0, 0, 0),
            key_bits: FixedBitSet::with_capacity(KEY_MAX as usize),
            rel: RelativeAxis::empty(),
            abs: AbsoluteAxis::empty(),
            switch: Switch::empty(),
            led: Led::empty(),
            misc: Misc::empty(),
            ff: FixedBitSet::with_capacity(FF_MAX as usize + 1),
            ff_stat: FFStatus::empty(),
            rep: Repeat::empty(),
            snd: Sound::empty(),
            pending_events: Vec::with_capacity(64),
            last_seen: 0,
            state: DeviceState {
                timestamp: libc::timeval { tv_sec: 0, tv_usec: 0 },
                key_vals: FixedBitSet::with_capacity(KEY_MAX as usize),
                abs_vals: vec![],
                switch_vals: FixedBitSet::with_capacity(0x10),
                led_vals: FixedBitSet::with_capacity(0x10),
            },
            clock: libc::CLOCK_REALTIME
        };

        let mut bits: u32 = 0;
        let mut bits64: u64 = 0;
        let mut buf = [0u8; 256];

        do_ioctl!(eviocgbit(fd, 0, 4, &mut bits as *mut u32 as *mut u8));
        dev.ty = Types::from_bits(bits).expect("evdev: unexpected type bits! report a bug");

        dev.name = do_ioctl_buf!(buf, eviocgname, fd).unwrap_or(CString::default());
        dev.phys = do_ioctl_buf!(buf, eviocgphys, fd);
        dev.uniq = do_ioctl_buf!(buf, eviocguniq, fd);

        do_ioctl!(eviocgid(fd, &mut dev.id));
        let mut driver_version: i32 = 0;
        do_ioctl!(eviocgversion(fd, &mut driver_version));
        dev.driver_version =
            (((driver_version >> 16) & 0xff) as u8,
             ((driver_version >> 8) & 0xff) as u8,
              (driver_version & 0xff) as u8);

        do_ioctl!(eviocgprop(fd, std::slice::from_raw_parts_mut(&mut bits as *mut u32 as *mut u8, 0x1f))); // FIXME: handle old kernel
        dev.props = Props::from_bits(bits).expect("evdev: unexpected prop bits! report a bug");

        if dev.ty.contains(KEY) {
            do_ioctl!(eviocgbit(fd, KEY.number(), dev.key_bits.len() as libc::c_int, dev.key_bits.as_mut_slice().as_mut_ptr() as *mut u8));
        }

        if dev.ty.contains(RELATIVE) {
            do_ioctl!(eviocgbit(fd, RELATIVE.number(), 0xf, &mut bits as *mut u32 as *mut u8));
            dev.rel = RelativeAxis::from_bits(bits).expect("evdev: unexpected rel bits! report a bug");
        }

        if dev.ty.contains(ABSOLUTE) {
            do_ioctl!(eviocgbit(fd, ABSOLUTE.number(), 0x3f, &mut bits64 as *mut u64 as *mut u8));
            dev.abs = AbsoluteAxis::from_bits(bits64).expect("evdev: unexpected abs bits! report a bug");
            dev.state.abs_vals = vec![input_absinfo::default(); 0x3f];
        }

        if dev.ty.contains(SWITCH) {
            do_ioctl!(eviocgbit(fd, SWITCH.number(), 0xf, &mut bits as *mut u32 as *mut u8));
            dev.switch = Switch::from_bits(bits).expect("evdev: unexpected switch bits! report a bug");
        }

        if dev.ty.contains(LED) {
            do_ioctl!(eviocgbit(fd, LED.number(), 0xf, &mut bits as *mut u32 as *mut u8));
            dev.led = Led::from_bits(bits).expect("evdev: unexpected led bits! report a bug");
        }

        if dev.ty.contains(MISC) {
            do_ioctl!(eviocgbit(fd, MISC.number(), 0x7, &mut bits as *mut u32 as *mut u8));
            dev.misc = Misc::from_bits(bits).expect("evdev: unexpected misc bits! report a bug");
        }

        //do_ioctl!(eviocgbit(fd, ffs(FORCEFEEDBACK.bits()), 0x7f, &mut bits as *mut u32 as *mut u8));

        if dev.ty.contains(SOUND) {
            do_ioctl!(eviocgbit(fd, SOUND.number(), 0x7, &mut bits as *mut u32 as *mut u8));
            dev.snd = Sound::from_bits(bits).expect("evdev: unexpected sound bits! report a bug");
        }

        try!(dev.sync_state());

        Ok(dev)
    }

    /// Synchronize the `Device` state with the kernel device state.
    ///
    /// If there is an error at any point, the state will not be synchronized completely.
    pub fn sync_state(&mut self) -> Result<(), Error> {
        if self.ty.contains(KEY) {
            do_ioctl!(eviocgkey(self.fd, transmute::<&mut [u32], &mut [u8]>(self.state.key_vals.as_mut_slice())));
        }
        if self.ty.contains(ABSOLUTE) {
            for idx in 0..0x28 {
                let abs = 1 << idx;
                // ignore multitouch, we'll handle that later.
                if abs < ABS_MT_SLOT.bits() && self.abs.bits() & abs != 0 {
                    do_ioctl!(eviocgabs(self.fd, idx as u32, &mut self.state.abs_vals[idx as usize]));
                }
            }
        }
        if self.ty.contains(SWITCH) {
            do_ioctl!(eviocgsw(self.fd, transmute::<&mut [u32], &mut [u8]>(self.state.switch_vals.as_mut_slice())));
        }
        if self.ty.contains(LED) {
            do_ioctl!(eviocgled(self.fd, transmute::<&mut [u32], &mut [u8]>(self.state.led_vals.as_mut_slice())));
        }

        Ok(())
    }

    /// Do SYN_DROPPED synchronization, and compensate for missing events by inserting events into
    /// the stream which, when applied to any state being kept outside of this `Device`, will
    /// synchronize it with the kernel state.
    fn compensate_dropped(&mut self) -> Result<(), Error> {
        let mut drop_from = None;
        for (idx, event) in self.pending_events[self.last_seen..].iter().enumerate() {
            if event._type == SYN_DROPPED as u16 {
                drop_from = Some(idx);
                break
            }
        }
        // FIXME: see if we can *not* drop EV_REL events. EV_REL doesn't have any state, so
        // dropping its events isn't really helping much.
        if let Some(idx) = drop_from {
            // look for the nearest SYN_REPORT before the SYN_DROPPED, remove everything after it.
            let mut prev_report = 0; // (if there's no previous SYN_REPORT, then the entire vector is bogus)
            for (idx, event) in self.pending_events[..idx].iter().enumerate().rev() {
                if event._type == SYN_REPORT as u16 {
                    prev_report = idx;
                    break;
                }
            }
            self.pending_events.truncate(prev_report);
        } else {
            return Ok(())
        }

        // Alright, pending_events is in a sane state. Now, let's sync the local state. We will
        // create a phony packet that contains deltas from the previous device state to the current
        // device state.
        let old_state = self.state.clone();
        try!(self.sync_state());
        let time = gettime(self.clock);

        if self.ty.contains(KEY) {
            for key_idx in 0..self.key_bits.len() {
                if self.key_bits.contains(key_idx) {
                    if old_state.key_vals[key_idx] != self.state.key_vals[key_idx] {
                        self.pending_events.push(raw::input_event {
                            time: time,
                            _type: KEY.number(),
                            code: key_idx as u16,
                            value: if self.state.key_vals[key_idx] { 1 } else { 0 },
                        });
                    }
                }
            }
        }
        if self.ty.contains(ABSOLUTE) {
            for idx in 0..0x3f {
                let abs = 1 << idx;
                if self.abs.bits() & abs != 0 {
                    if old_state.abs_vals[idx as usize] != self.state.abs_vals[idx as usize] {
                        self.pending_events.push(raw::input_event {
                            time: time,
                            _type: ABSOLUTE.number(),
                            code: idx as u16,
                            value: self.state.abs_vals[idx as usize].value,
                        });
                    }
                }
            }
        }
        if self.ty.contains(SWITCH) {
            for idx in 0..0xf {
                let sw = 1 << idx;
                if sw < SW_MAX.bits() && self.switch.bits() & sw == 1 {
                    if old_state.switch_vals[idx as usize] != self.state.switch_vals[idx as usize] {
                        self.pending_events.push(raw::input_event {
                            time: time,
                            _type: SWITCH.number(),
                            code: idx as u16,
                            value: if self.state.switch_vals[idx as usize] { 1 } else { 0 },
                        });
                    }
                }
            }
        }
        if self.ty.contains(LED) {
            for idx in 0..0xf {
                let led = 1 << idx;
                if led < LED_MAX.bits() && self.led.bits() & led == 1 {
                    if old_state.led_vals[idx as usize] != self.state.led_vals[idx as usize] {
                        self.pending_events.push(raw::input_event {
                            time: time,
                            _type: LED.number(),
                            code: idx as u16,
                            value: if self.state.led_vals[idx as usize] { 1 } else { 0 },
                        });
                    }
                }
            }
        }

        self.pending_events.push(raw::input_event {
            time: time,
            _type: SYNCHRONIZATION.number(),
            code: SYN_REPORT as u16,
            value: 0,
        });
        Ok(())
    }

    fn fill_events(&mut self) -> Result<(), Error> {
        let buf = &mut self.pending_events;
        loop {
            buf.reserve(20);
            let pre_len = buf.len();
            let sz = unsafe {
                libc::read(self.fd,
                           buf.as_mut_ptr()
                              .offset(pre_len as isize) as *mut libc::c_void,
                           (size_of::<raw::input_event>() * (buf.capacity() - pre_len)) as libc::size_t)
            };
            if sz == -1 {
                let errno = ::nix::Errno::last();
                if errno != ::nix::Errno::EAGAIN {
                    return Err(Error::from_errno(errno));
                } else {
                    break;
                }
            } else {
                unsafe {
                    buf.set_len(pre_len + (sz as usize / size_of::<raw::input_event>()));
                }
            }
        }
        Ok(())
    }

    /// Exposes the raw evdev events without doing synchronization on SYN_DROPPED.
    pub fn events_no_sync(&mut self) -> Result<RawEvents, Error> {
        try!(self.fill_events());
        Ok(RawEvents::new(self))
    }

    /// Exposes the raw evdev events, doing synchronization on SYN_DROPPED.
    ///
    /// Will insert "fake" events
    pub fn events(&mut self) -> Result<RawEvents, Error> {
        try!(self.fill_events());
        try!(self.compensate_dropped());

        Ok(RawEvents(self))
    }
}

pub struct Events<'a>(&'a mut Device);

pub struct RawEvents<'a>(&'a mut Device);

impl<'a> RawEvents<'a> {
    fn new(dev: &'a mut Device) -> RawEvents<'a> {
        dev.pending_events.reverse();
        RawEvents(dev)
    }
}

impl<'a> Drop for RawEvents<'a> {
    fn drop(&mut self) {
        self.0.pending_events.reverse();
        self.0.last_seen = self.0.pending_events.len();
    }
}

impl<'a> Iterator for RawEvents<'a> {
    type Item = raw::input_event;

    #[inline(always)]
    fn next(&mut self) -> Option<raw::input_event> {
        self.0.pending_events.pop()
    }
}

/// Crawls `/dev/input` for evdev devices.
///
/// Will not bubble up any errors in opening devices or traversing the directory. Instead returns
/// an empty vector or omits the devices that could not be opened.
pub fn enumerate() -> Vec<Device> {
    let mut res = Vec::new();
    if let Ok(dir) = std::fs::read_dir("/dev/input") {
        for entry in dir {
            if let Ok(entry) = entry {
                if let Ok(dev) = Device::open(&entry.path()) {
                    res.push(dev)
                }
            }
        }
    }
    res
}

#[cfg(test)] mod test { include!("tests.rs"); }
