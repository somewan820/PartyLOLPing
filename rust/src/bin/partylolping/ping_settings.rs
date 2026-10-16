use std::ffi::CStr;

use cocoa::base::{id, nil};
use cocoa::foundation::{NSString, NSUserDefaults};
use objc::{class, msg_send, runtime::BOOL, sel, sel_impl};
use party_engine::TriggerChord;

#[derive(Clone, Debug)]
pub struct PingSettings {
    pub enabled: bool,
    pub chord: TriggerChord,
    pub volume: f64,
    pub scale: f64,
}

impl Default for PingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            chord: TriggerChord::ControlOptionCommand,
            volume: 0.55,
            scale: 1.0,
        }
    }
}

impl PingSettings {
    pub fn load() -> Self {
        unsafe {
            let defaults: id = NSUserDefaults::standardUserDefaults();
            let mut settings = Self {
                enabled: defaults.bool_forKey_(ns_string("enabled")),
                ..Self::default()
            };
            let chord = object_for_key(defaults, "triggerChord");
            if !chord.is_null() {
                if let Some(value) = rust_string(chord) {
                    if let Some(chord) = TriggerChord::from_raw_value(&value) {
                        settings.chord = chord;
                    }
                }
            }
            if !object_for_key(defaults, "volume").is_null() {
                let value: f64 = msg_send![defaults, doubleForKey: ns_string("volume")];
                settings.volume = value.clamp(0.0, 1.0);
            }
            if !object_for_key(defaults, "effectScale").is_null() {
                let value: f64 = msg_send![defaults, doubleForKey: ns_string("effectScale")];
                settings.scale = value.clamp(0.75, 1.5);
            }
            settings
        }
    }

    pub fn save(&self) {
        unsafe {
            let defaults: id = NSUserDefaults::standardUserDefaults();
            defaults.setBool_forKey_(self.enabled as BOOL, ns_string("enabled"));
            let _: () = msg_send![
                defaults,
                setObject: ns_string(self.chord.raw_value())
                forKey: ns_string("triggerChord")
            ];
            let _: () = msg_send![defaults, setDouble: self.volume forKey: ns_string("volume")];
            let _: () = msg_send![defaults, setDouble: self.scale forKey: ns_string("effectScale")];
            let _: () = msg_send![defaults, synchronize];
        }
    }
}

pub fn open_accessibility_settings() {
    open_url("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility");
}

pub fn open_input_monitoring_settings() {
    open_url("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent");
}

fn open_url(value: &str) {
    unsafe {
        let url: id = msg_send![class!(NSURL), URLWithString: ns_string(value)];
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let _: BOOL = msg_send![workspace, openURL: url];
    }
}

fn object_for_key(defaults: id, key: &str) -> id {
    unsafe { msg_send![defaults, objectForKey: ns_string(key)] }
}

fn rust_string(value: id) -> Option<String> {
    unsafe {
        let bytes: *const i8 = msg_send![value, UTF8String];
        if bytes.is_null() {
            None
        } else {
            Some(CStr::from_ptr(bytes).to_string_lossy().into_owned())
        }
    }
}

fn ns_string(value: &str) -> id {
    unsafe { NSString::alloc(nil).init_str(value) }
}
