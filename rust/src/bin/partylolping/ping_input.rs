use std::collections::{HashSet, VecDeque};
use std::ffi::c_void;
use std::fmt;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::date::CFDate;
use core_foundation::dictionary::CFDictionary;
use core_foundation::mach_port::{CFMachPort, CFMachPortInvalidate, CFMachPortRef};
use core_foundation::runloop::{
    kCFRunLoopCommonModes, CFRunLoop, CFRunLoopSource, CFRunLoopTimer, CFRunLoopTimerContext,
    CFRunLoopTimerInvalidate, CFRunLoopTimerRef,
};
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventMask, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventTapProxy, CGEventType, EventField,
};
use core_graphics::event_source::CGEventSourceStateID;
use core_graphics::sys::CGEventRef;
use foreign_types::ForeignType;
use objc::{class, msg_send, sel, sel_impl};
use party_engine::{
    GestureAction, GestureMachine, GesturePhase, Modifiers, PingPoint, TriggerChord, WheelGeometry,
};

pub const TRIGGER_DELAY_SECONDS: f64 = 0.18;

#[derive(Clone, Debug)]
pub enum PingInputEvent {
    Action(GestureAction),
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PingInputStartError {
    Permission,
    EventTap,
}

impl fmt::Display for PingInputStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Permission => formatter.write_str("请先在系统设置中允许 PartyLOLPing 使用辅助功能。"),
            Self::EventTap => formatter.write_str(
                "无法接收全局按键。请检查辅助功能权限；若系统要求输入监控权限，也请为 PartyLOLPing 开启，然后重试。",
            ),
        }
    }
}

struct Shared {
    machine: GestureMachine,
    events: VecDeque<PingInputEvent>,
    keys_down: HashSet<i64>,
    swallowed_keys: HashSet<i64>,
    swallowed_right: bool,
    timer: Option<CFRunLoopTimer>,
    timer_context: Option<*mut c_void>,
}

impl Shared {
    fn new(chord: TriggerChord, scale: f64) -> Self {
        let mut machine = GestureMachine::default();
        machine.chord = chord;
        machine.dead_zone = WheelGeometry::CENTER_RADIUS * scale;
        machine.reset(false);
        Self {
            machine,
            events: VecDeque::new(),
            keys_down: HashSet::new(),
            swallowed_keys: HashSet::new(),
            swallowed_right: false,
            timer: None,
            timer_context: None,
        }
    }
}

struct CallbackContext {
    shared: Arc<Mutex<Shared>>,
    tap: CFMachPortRef,
    recovery_attempts: u32,
}

struct EventTap {
    port: CFMachPort,
    source: CFRunLoopSource,
    context: *mut CallbackContext,
}

pub struct PingInput {
    shared: Arc<Mutex<Shared>>,
    tap: Option<EventTap>,
}

impl PingInput {
    pub fn new(chord: TriggerChord, scale: f64) -> Self {
        Self {
            shared: Arc::new(Mutex::new(Shared::new(chord, scale))),
            tap: None,
        }
    }

    pub fn is_trusted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    pub fn request_permission() {
        unsafe {
            let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
            let options = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
            let _ = AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef().cast());
        }
    }

    pub fn start(&mut self, chord: TriggerChord, scale: f64) -> Result<(), PingInputStartError> {
        self.stop();
        if !Self::is_trusted() {
            return Err(PingInputStartError::Permission);
        }

        let shared = Arc::new(Mutex::new(Shared::new(chord, scale)));
        {
            let mut shared_guard = shared.lock().unwrap_or_else(|error| error.into_inner());
            let flags = current_modifiers();
            shared_guard
                .machine
                .reset(!flags.intersection(chord.modifiers()).is_empty());
        }

        let context = Box::new(CallbackContext {
            shared: Arc::clone(&shared),
            tap: std::ptr::null_mut(),
            recovery_attempts: 0,
        });
        let context = Box::into_raw(context);
        let mask = event_mask();
        let port_ref = unsafe {
            CGEventTapCreate(
                CGEventTapLocation::Session,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::Default,
                mask,
                event_tap_callback,
                context.cast(),
            )
        };
        if port_ref.is_null() {
            unsafe { drop(Box::from_raw(context)) };
            return Err(PingInputStartError::EventTap);
        }

        unsafe { (*context).tap = port_ref };
        let port = unsafe { CFMachPort::wrap_under_create_rule(port_ref) };
        let source = match port.create_runloop_source(0) {
            Ok(source) => source,
            Err(()) => {
                unsafe {
                    CGEventTapEnable(port_ref, false);
                    CFMachPortInvalidate(port_ref);
                    drop(Box::from_raw(context));
                }
                return Err(PingInputStartError::EventTap);
            }
        };
        CFRunLoop::get_main().add_source(&source, unsafe { kCFRunLoopCommonModes });
        unsafe { CGEventTapEnable(port_ref, true) };

        self.shared = shared;
        self.tap = Some(EventTap {
            port,
            source,
            context,
        });
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(shared) = Arc::get_mut(&mut self.shared) {
            let shared = shared.get_mut().unwrap_or_else(|error| error.into_inner());
            clear_timer(shared);
            shared.keys_down.clear();
            shared.swallowed_keys.clear();
            shared.swallowed_right = false;
            shared.machine.reset(false);
            shared.events.clear();
        } else {
            let mut shared = self
                .shared
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            clear_timer(&mut shared);
            shared.keys_down.clear();
            shared.swallowed_keys.clear();
            shared.swallowed_right = false;
            shared.machine.reset(false);
            shared.events.clear();
        }

        if let Some(tap) = self.tap.take() {
            unsafe { CGEventTapEnable(tap.port.as_concrete_TypeRef(), false) };
            CFRunLoop::get_main().remove_source(&tap.source, unsafe { kCFRunLoopCommonModes });
            unsafe { CFMachPortInvalidate(tap.port.as_concrete_TypeRef()) };
            unsafe { drop(Box::from_raw(tap.context)) };
        }
    }

    pub fn cancel_current(&mut self) {
        let actions = {
            let mut shared = self
                .shared
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            shared.machine.cancel()
        };
        dispatch_actions(&self.shared, actions);
    }

    pub fn set_wheel_center(&mut self, center: PingPoint) {
        let actions = {
            let mut shared = self
                .shared
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            shared.machine.set_wheel_center(center)
        };
        dispatch_actions(&self.shared, actions);
    }

    pub fn drain_events(&mut self) -> Vec<PingInputEvent> {
        let mut shared = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        shared.events.drain(..).collect()
    }
}

impl Drop for PingInput {
    fn drop(&mut self) {
        self.stop();
    }
}

fn event_mask() -> CGEventMask {
    const TYPES: &[CGEventType] = &[
        CGEventType::FlagsChanged,
        CGEventType::KeyDown,
        CGEventType::KeyUp,
        CGEventType::MouseMoved,
        CGEventType::LeftMouseDown,
        CGEventType::LeftMouseUp,
        CGEventType::LeftMouseDragged,
        CGEventType::RightMouseDown,
        CGEventType::RightMouseUp,
        CGEventType::RightMouseDragged,
        CGEventType::OtherMouseDown,
        CGEventType::OtherMouseUp,
        CGEventType::OtherMouseDragged,
        CGEventType::ScrollWheel,
    ];
    TYPES
        .iter()
        .fold(0, |mask, event_type| mask | (1_u64 << *event_type as u32))
}

fn modifiers(flags: CGEventFlags) -> Modifiers {
    let mut result = Modifiers::EMPTY;
    if flags.contains(CGEventFlags::CGEventFlagControl) {
        result |= Modifiers::CONTROL;
    }
    if flags.contains(CGEventFlags::CGEventFlagAlternate) {
        result |= Modifiers::OPTION;
    }
    if flags.contains(CGEventFlags::CGEventFlagCommand) {
        result |= Modifiers::COMMAND;
    }
    if flags.contains(CGEventFlags::CGEventFlagShift) {
        result |= Modifiers::SHIFT;
    }
    result
}

fn current_modifiers() -> Modifiers {
    unsafe {
        modifiers(CGEventSourceFlagsState(
            CGEventSourceStateID::CombinedSessionState,
        ))
    }
}

fn point(event: &CGEvent) -> PingPoint {
    let location = event.location();
    PingPoint::new(location.x, location.y)
}

fn mouse_buttons_pressed() -> bool {
    let buttons: usize = unsafe { msg_send![class!(NSEvent), pressedMouseButtons] };
    buttons != 0
}

unsafe extern "C" fn event_tap_callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: CGEventRef,
    user_info: *const c_void,
) -> CGEventRef {
    let context = user_info as *mut CallbackContext;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if matches!(
            event_type,
            CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
        ) {
            recover_tap(context);
            return false;
        }
        let event = ManuallyDrop::new(CGEvent::from_ptr(event));
        receive(context, event_type, &event)
    }));
    match result {
        Ok(true) => std::ptr::null_mut(),
        _ => event,
    }
}

unsafe fn recover_tap(context: *mut CallbackContext) {
    let context = &mut *context;
    let actions = {
        let mut shared = context
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        shared.machine.cancel()
    };
    dispatch_actions(&context.shared, actions);
    context.recovery_attempts += 1;
    if context.recovery_attempts <= 2 && PingInput::is_trusted() && !context.tap.is_null() {
        CGEventTapEnable(context.tap, true);
    } else {
        context
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .events
            .push_back(PingInputEvent::Unavailable);
    }
}

unsafe fn receive(context: *mut CallbackContext, event_type: CGEventType, event: &CGEvent) -> bool {
    let context = &mut *context;
    context.recovery_attempts = 0;
    let mut shared = context
        .shared
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut actions = Vec::new();
    let mut swallow = false;

    match event_type {
        CGEventType::FlagsChanged => {
            let other_input_held = mouse_buttons_pressed() || !shared.keys_down.is_empty();
            actions = shared.machine.flags_changed(
                modifiers(event.get_flags()),
                point(event),
                other_input_held,
            );
        }
        CGEventType::KeyDown => {
            let key = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
            shared.keys_down.insert(key);
            let active = matches!(
                shared.machine.phase,
                GesturePhase::Arming | GesturePhase::Open
            );
            if shared.swallowed_keys.contains(&key) {
                return true;
            }
            actions = shared.machine.cancel();
            if active && key == 53 {
                shared.swallowed_keys.insert(key);
                swallow = true;
            }
        }
        CGEventType::KeyUp => {
            let key = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
            shared.keys_down.remove(&key);
            if shared.swallowed_keys.remove(&key) {
                swallow = true;
            }
        }
        CGEventType::MouseMoved => {
            actions = shared.machine.moved_to(point(event));
        }
        CGEventType::RightMouseDown => {
            let active = matches!(
                shared.machine.phase,
                GesturePhase::Arming | GesturePhase::Open
            );
            actions = shared.machine.cancel();
            if active {
                shared.swallowed_right = true;
                swallow = true;
            }
        }
        CGEventType::RightMouseUp => {
            if shared.swallowed_right {
                shared.swallowed_right = false;
                swallow = true;
            }
        }
        CGEventType::RightMouseDragged => {
            if shared.swallowed_right {
                swallow = true;
            }
        }
        CGEventType::LeftMouseDown
        | CGEventType::OtherMouseDown
        | CGEventType::LeftMouseDragged
        | CGEventType::OtherMouseDragged
        | CGEventType::ScrollWheel => {
            actions = shared.machine.cancel();
        }
        _ => {}
    }

    drop(shared);
    dispatch_actions(&context.shared, actions);
    swallow
}

fn dispatch_actions(shared: &Arc<Mutex<Shared>>, actions: Vec<GestureAction>) {
    for action in actions {
        match action {
            GestureAction::Arm(_) => schedule_arm_timer(shared),
            GestureAction::Hide | GestureAction::Dismiss => {
                let mut guard = shared.lock().unwrap_or_else(|error| error.into_inner());
                clear_timer(&mut guard);
                guard.events.push_back(PingInputEvent::Action(action));
            }
            _ => {
                shared
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .events
                    .push_back(PingInputEvent::Action(action));
            }
        }
    }
}

fn schedule_arm_timer(shared: &Arc<Mutex<Shared>>) {
    {
        let mut guard = shared.lock().unwrap_or_else(|error| error.into_inner());
        clear_timer(&mut guard);
    }

    let timer_context = Arc::into_raw(Arc::clone(shared)) as *mut c_void;
    let mut context = CFRunLoopTimerContext {
        version: 0,
        info: timer_context,
        retain: None,
        release: None,
        copyDescription: None,
    };
    let timer = CFRunLoopTimer::new(
        CFDate::now().abs_time() + TRIGGER_DELAY_SECONDS,
        0.0,
        0,
        0,
        arm_timer_fired,
        &mut context,
    );
    CFRunLoop::get_main().add_timer(&timer, unsafe { kCFRunLoopCommonModes });
    let mut guard = shared.lock().unwrap_or_else(|error| error.into_inner());
    guard.timer = Some(timer);
    guard.timer_context = Some(timer_context);
}

extern "C" fn arm_timer_fired(_timer: CFRunLoopTimerRef, timer_context: *mut c_void) {
    let shared = unsafe {
        let shared = Arc::from_raw(timer_context as *const Mutex<Shared>);
        let clone = Arc::clone(&shared);
        std::mem::forget(shared);
        clone
    };
    let actions = {
        let mut guard = shared.lock().unwrap_or_else(|error| error.into_inner());
        guard.machine.delay_elapsed()
    };
    dispatch_actions(&shared, actions);
    let mut guard = shared.lock().unwrap_or_else(|error| error.into_inner());
    clear_timer(&mut guard);
}

fn clear_timer(shared: &mut Shared) {
    if let Some(timer) = shared.timer.take() {
        unsafe { CFRunLoopTimerInvalidate(timer.as_concrete_TypeRef()) };
    }
    if let Some(context) = shared.timer_context.take() {
        unsafe { drop(Arc::from_raw(context as *const Mutex<Shared>)) };
    }
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    static kAXTrustedCheckOptionPrompt: CFStringRef;
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: CGEventTapLocation,
        place: CGEventTapPlacement,
        options: CGEventTapOptions,
        events_of_interest: CGEventMask,
        callback: unsafe extern "C" fn(
            CGEventTapProxy,
            CGEventType,
            CGEventRef,
            *const c_void,
        ) -> CGEventRef,
        user_info: *const c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventSourceFlagsState(state_id: CGEventSourceStateID) -> CGEventFlags;
}
