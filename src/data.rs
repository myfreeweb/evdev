use num;
pub use fixedbitset::FixedBitSet;

pub use Key::*;
pub use FFEffect::*;
pub use Synchronization::*;

bitflags! {
    /// Event types supported by the device.
    pub flags Types: u32 {
        /// A bookkeeping event. Usually not important to applications.
        const SYNCHRONIZATION = 1 << 0x00,
        /// A key changed state. A key, or button, is usually a momentary switch (in the circuit sense). It has two
        /// states: down, or up. There are events for when keys are pressed (become down) and
        /// released (become up). There are also "key repeats", where multiple events are sent
        /// while a key is down.
        const KEY = 1 << 0x01,
        /// Movement on a relative axis. There is no absolute coordinate frame, just the fact that
        /// there was a change of a certain amount of units. Used for things like mouse movement or
        /// scroll wheels.
        const RELATIVE = 1 << 0x02,
        /// Movement on an absolute axis. Used for things such as touch events and joysticks.
        const ABSOLUTE = 1 << 0x03,
        /// Miscellaneous events that don't fall into other categories. I'm not quite sure when
        /// these happen or what they correspond to.
        const MISC = 1 << 0x04,
        /// Change in a switch value. Switches are boolean conditions and usually correspond to a
        /// toggle switch of some kind in hardware.
        const SWITCH = 1 << 0x05,
        /// An LED was toggled.
        const LED = 1 << 0x11,
        /// A sound was made.
        const SOUND = 1 << 0x12,
        /// There are no events of this type, to my knowledge, but represents metadata about key
        /// repeat configuration.
        const REPEAT = 1 << 0x14,
        /// I believe there are no events of this type, but rather this is used to represent that
        /// the device can create haptic effects.
        const FORCEFEEDBACK = 1 << 0x15,
        /// I think this is unused?
        const POWER = 1 << 0x16,
        /// A force feedback effect's state changed.
        const FORCEFEEDBACKSTATUS = 1 << 0x17,
    }
}

bitflags! {
    /// Device properties.
    pub flags Props: u32 {
        /// This input device needs a pointer ("cursor") for the user to know its state.
        const POINTER = 1 << 0x00,
        /// "direct input devices", according to the header.
        const DIRECT = 1 << 0x01,
        /// "has button(s) under pad", according to the header.
        const BUTTONPAD = 1 << 0x02,
        /// Touch rectangle only (I think this means that if there are multiple touches, then the
        /// bounding rectangle of all the touches is returned, not each touch).
        const SEMI_MT = 1 << 0x03,
        /// "softbuttons at top of pad", according to the header.
        const TOPBUTTONPAD = 1 << 0x04,
        /// Is a pointing stick ("clit mouse" etc, https://xkcd.com/243/)
        const POINTING_STICK = 1 << 0x05,
        /// Has an accelerometer. Probably reports relative events in that case?
        const ACCELEROMETER = 1 << 0x06
    }
}

include!("scancodes.rs"); // it's a huge glob of text that I'm tired of skipping over.

bitflags! {
    pub flags RelativeAxis: u32 {
        const REL_X = 1 << 0x00,
        const REL_Y = 1 << 0x01,
        const REL_Z = 1 << 0x02,
        const REL_RX = 1 << 0x03,
        const REL_RY = 1 << 0x04,
        const REL_RZ = 1 << 0x05,
        const REL_HWHEEL = 1 << 0x06,
        const REL_DIAL = 1 << 0x07,
        const REL_WHEEL = 1 << 0x08,
        const REL_MISC = 1 << 0x09,
    }
}

bitflags! {
    pub flags AbsoluteAxis: u64 {
        const ABS_X = 1 << 0x00,
        const ABS_Y = 1 << 0x01,
        const ABS_Z = 1 << 0x02,
        const ABS_RX = 1 << 0x03,
        const ABS_RY = 1 << 0x04,
        const ABS_RZ = 1 << 0x05,
        const ABS_THROTTLE = 1 << 0x06,
        const ABS_RUDDER = 1 << 0x07,
        const ABS_WHEEL = 1 << 0x08,
        const ABS_GAS = 1 << 0x09,
        const ABS_BRAKE = 1 << 0x0a,
        const ABS_HAT0X = 1 << 0x10,
        const ABS_HAT0Y = 1 << 0x11,
        const ABS_HAT1X = 1 << 0x12,
        const ABS_HAT1Y = 1 << 0x13,
        const ABS_HAT2X = 1 << 0x14,
        const ABS_HAT2Y = 1 << 0x15,
        const ABS_HAT3X = 1 << 0x16,
        const ABS_HAT3Y = 1 << 0x17,
        const ABS_PRESSURE = 1 << 0x18,
        const ABS_DISTANCE = 1 << 0x19,
        const ABS_TILT_X = 1 << 0x1a,
        const ABS_TILT_Y = 1 << 0x1b,
        const ABS_TOOL_WIDTH = 1 << 0x1c,
        const ABS_VOLUME = 1 << 0x20,
        const ABS_MISC = 1 << 0x28,
        /// "MT slot being modified"
        const ABS_MT_SLOT = 1 << 0x2f,
        /// "Major axis of touching ellipse"
        const ABS_MT_TOUCH_MAJOR = 1 << 0x30,
        /// "Minor axis (omit if circular)"
        const ABS_MT_TOUCH_MINOR = 1 << 0x31,
        /// "Major axis of approaching ellipse"
        const ABS_MT_WIDTH_MAJOR = 1 << 0x32,
        /// "Minor axis (omit if circular)"
        const ABS_MT_WIDTH_MINOR = 1 << 0x33,
        /// "Ellipse orientation"
        const ABS_MT_ORIENTATION = 1 << 0x34,
        /// "Center X touch position"
        const ABS_MT_POSITION_X = 1 << 0x35,
        /// "Center Y touch position"
        const ABS_MT_POSITION_Y = 1 << 0x36,
        /// "Type of touching device"
        const ABS_MT_TOOL_TYPE = 1 << 0x37,
        /// "Group a set of packets as a blob"
        const ABS_MT_BLOB_ID = 1 << 0x38,
        /// "Unique ID of the initiated contact"
        const ABS_MT_TRACKING_ID = 1 << 0x39,
        /// "Pressure on contact area"
        const ABS_MT_PRESSURE = 1 << 0x3a,
        /// "Contact over distance"
        const ABS_MT_DISTANCE = 1 << 0x3b,
        /// "Center X tool position"
        const ABS_MT_TOOL_X = 1 << 0x3c,
        /// "Center Y tool position"
        const ABS_MT_TOOL_Y = 1 << 0x3d,
    }
}

bitflags! {
    pub flags Switch: u32 {
        /// "set = lid shut"
        const SW_LID = 1 << 0x00,
        /// "set = tablet mode"
        const SW_TABLET_MODE = 1 << 0x01,
        /// "set = inserted"
        const SW_HEADPHONE_INSERT = 1 << 0x02,
        /// "rfkill master switch, type 'any'"
        const SW_RFKILL_ALL = 1 << 0x03,
        /// "set = inserted"
        const SW_MICROPHONE_INSERT = 1 << 0x04,
        /// "set = plugged into doc"
        const SW_DOCK = 1 << 0x05,
        /// "set = inserted"
        const SW_LINEOUT_INSERT = 1 << 0x06,
        /// "set = mechanical switch set"
        const SW_JACK_PHYSICAL_INSERT = 1 << 0x07,
        /// "set  = inserted"
        const SW_VIDEOOUT_INSERT = 1 << 0x08,
        /// "set = lens covered"
        const SW_CAMERA_LENS_COVER = 1 << 0x09,
        /// "set = keypad slide out"
        const SW_KEYPAD_SLIDE = 1 << 0x0a,
        /// "set = front proximity sensor active"
        const SW_FRONT_PROXIMITY = 1 << 0x0b,
        /// "set = rotate locked/disabled"
        const SW_ROTATE_LOCK = 1 << 0x0c,
        /// "set = inserted"
        const SW_LINEIN_INSERT = 1 << 0x0d,
        /// "set = device disabled"
        const SW_MUTE_DEVICE = 1 << 0x0e,
        /// "set = pen inserted"
        const SW_PEN_INSERTED = 1 << 0x0f,
        const SW_MAX = 0xf,
    }
}

bitflags! {
    /// LEDs specified by USB HID.
    pub flags Led: u32 {
        const LED_NUML = 1 << 0x00,
        const LED_CAPSL = 1 << 0x01,
        const LED_SCROLLL = 1 << 0x02,
        const LED_COMPOSE = 1 << 0x03,
        const LED_KANA = 1 << 0x04,
        /// "Stand-by"
        const LED_SLEEP = 1 << 0x05,
        const LED_SUSPEND = 1 << 0x06,
        const LED_MUTE = 1 << 0x07,
        /// "Generic indicator"
        const LED_MISC = 1 << 0x08,
        /// "Message waiting"
        const LED_MAIL = 1 << 0x09,
        /// "External power connected"
        const LED_CHARGING = 1 << 0x0a,
        const LED_MAX = 1 << 0x0f,
    }
}

bitflags! {
    /// Various miscellaneous event types. Current as of kernel 4.1.
    pub flags Misc: u32 {
        /// Serial number, only exported for tablets ("Transducer Serial Number")
        const MSC_SERIAL = 1 << 0x00,
        /// Only used by the PowerMate driver, right now.
        const MSC_PULSELED = 1 << 0x01,
        /// Completely unused.
        const MSC_GESTURE = 1 << 0x02,
        /// "Raw" event, rarely used.
        const MSC_RAW = 1 << 0x03,
        /// Key scancode
        const MSC_SCAN = 1 << 0x04,
        /// Completely unused.
        const MSC_TIMESTAMP = 1 << 0x05,
        const MSC_MAX = 1 << 0x07,
    }
}

bitflags! {
    pub flags FFStatus: u32 {
        const FF_STATUS_STOPPED	= 1 << 0x00,
        const FF_STATUS_PLAYING	= 1 << 0x01,
    }
}

#[repr(C)]
#[derive(Copy, Clone, EnumString)]
pub enum FFEffect {
    FF_RUMBLE = 0x50,
    FF_PERIODIC = 0x51,
    FF_CONSTANT = 0x52,
    FF_SPRING = 0x53,
    FF_FRICTION = 0x54,
    FF_DAMPER = 0x55,
    FF_INERTIA = 0x56,
    FF_RAMP = 0x57,
    FF_SQUARE = 0x58,
    FF_TRIANGLE = 0x59,
    FF_SINE = 0x5a,
    FF_SAW_UP = 0x5b,
    FF_SAW_DOWN = 0x5c,
    FF_CUSTOM = 0x5d,
    FF_GAIN = 0x60,
    FF_AUTOCENTER = 0x61,
    FF_MAX = 0x7f,
}

bitflags! {
    pub flags Repeat: u32 {
        const REP_DELAY = 1 << 0x00,
        const REP_PERIOD = 1 << 0x01,
    }
}

bitflags! {
    pub flags Sound: u32 {
        const SND_CLICK = 1 << 0x00,
        const SND_BELL = 1 << 0x01,
        const SND_TONE = 1 << 0x02,
    }
}

macro_rules! impl_number {
    ($($t:ident),*) => {
        $(impl $t {
            /// Given a bitflag with only a single flag set, returns the event code corresponding to that
            /// event. If multiple flags are set, the one with the most significant bit wins. In debug
            /// mode,
            #[inline(always)]
            pub fn number<T: num::FromPrimitive>(&self) -> T {
                let val = self.bits().trailing_zeros();
                debug_assert!(self.bits() == 1 << val, "{:?} ought to have only one flag set to be used with .number()", self);
                T::from_u32(val).unwrap()
            }
        })*
    }
}

impl_number!(Types, Props, RelativeAxis, AbsoluteAxis, Switch, Led, Misc, FFStatus, Repeat, Sound);

#[repr(C)]
#[derive(Copy, Clone, Debug, EnumString)]
pub enum Synchronization {
    /// Terminates a packet of events from the device.
    SYN_REPORT = 0,
    /// Appears to be unused.
    SYN_CONFIG = 1,
    /// "Used to synchronize and separate touch events"
    SYN_MT_REPORT = 2,
    /// Ring buffer filled, events were dropped.
    SYN_DROPPED = 3,
}
