use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::{Arc, Mutex, Once};

use cocoa::appkit::{NSMenu, NSMenuItem, NSStatusBar};
use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel, NO};
use objc::{class, msg_send, sel, sel_impl};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusBarAction {
    Toggle,
    Open,
    Quit,
}

struct StatusBarContext {
    actions: Arc<Mutex<VecDeque<StatusBarAction>>>,
}

pub struct PingStatusBar {
    item: id,
    state_item: id,
    enable_item: id,
    delegate: id,
    context: *mut StatusBarContext,
    actions: Arc<Mutex<VecDeque<StatusBarAction>>>,
}

impl PingStatusBar {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let context = Box::new(StatusBarContext {
                actions: Arc::new(Mutex::new(VecDeque::new())),
            });
            let actions = Arc::clone(&context.actions);
            let context = Box::into_raw(context);
            let delegate_class = delegate_class();
            let delegate: id = msg_send![delegate_class, alloc];
            let delegate: id = msg_send![delegate, init];
            if delegate.is_null() {
                drop(Box::from_raw(context));
                return Err("无法创建菜单栏代理".to_owned());
            }
            (*delegate).set_ivar("context", context as *mut c_void);

            let status_bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
            let item = status_bar.statusItemWithLength_(-1.0);
            if item.is_null() {
                drop(Box::from_raw(context));
                let _: () = msg_send![delegate, release];
                return Err("无法创建菜单栏状态项".to_owned());
            }
            let item: id = msg_send![item, retain];
            let menu = NSMenu::alloc(nil).initWithTitle_(ns_string("PartyLOLPing"));

            let state_item = NSMenuItem::new(nil);
            let _: () = msg_send![state_item, setTitle: ns_string("PartyLOLPing · 已关闭")];
            let _: () = msg_send![state_item, setEnabled: NO];
            menu.addItem_(state_item);

            let enable_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                ns_string("启用 Ping"),
                sel!(togglePing:),
                ns_string(""),
            );
            enable_item.setTarget_(delegate);
            menu.addItem_(enable_item);

            let open_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                ns_string("打开控制窗口…"),
                sel!(openWindow:),
                ns_string(""),
            );
            open_item.setTarget_(delegate);
            menu.addItem_(open_item);
            menu.addItem_(NSMenuItem::separatorItem(nil));

            let quit_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                ns_string("退出 PartyLOLPing"),
                sel!(quitApplication:),
                ns_string("q"),
            );
            quit_item.setTarget_(delegate);
            menu.addItem_(quit_item);

            let _: () = msg_send![item, setMenu: menu];
            let _: () = msg_send![menu, release];
            let _: () = msg_send![open_item, release];
            let _: () = msg_send![quit_item, release];

            let mut result = Self {
                item,
                state_item,
                enable_item,
                delegate,
                context,
                actions,
            };
            result.update(false, false);
            Ok(result)
        }
    }

    pub fn update(&mut self, enabled: bool, awaiting_permission: bool) {
        unsafe {
            let state = if enabled {
                "已开启"
            } else if awaiting_permission {
                "等待授权"
            } else {
                "已关闭"
            };
            let _: () = msg_send![
                self.state_item,
                setTitle: ns_string(&format!("PartyLOLPing · {state}"))
            ];
            let _: () = msg_send![
                self.enable_item,
                setTitle: ns_string(if awaiting_permission { "取消启用" } else { "启用 Ping" })
            ];
            let state_value = if enabled {
                1_isize
            } else if awaiting_permission {
                -1_isize
            } else {
                0
            };
            let _: () = msg_send![self.enable_item, setState: state_value];
            let button: id = msg_send![self.item, button];
            let _: () = msg_send![
                button,
                setToolTip: ns_string(&format!("PartyLOLPing · {state}"))
            ];
            let symbol = if enabled {
                "location.circle.fill"
            } else {
                "location.circle"
            };
            let image: id = msg_send![
                class!(NSImage),
                imageWithSystemSymbolName: ns_string(symbol)
                accessibilityDescription: ns_string("PartyLOLPing")
            ];
            if !image.is_null() {
                let _: () = msg_send![button, setImage: image];
            } else {
                let _: () = msg_send![button, setTitle: ns_string("P")];
            }
        }
    }

    pub fn drain_actions(&mut self) -> Vec<StatusBarAction> {
        self.actions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .drain(..)
            .collect()
    }
}

impl Drop for PingStatusBar {
    fn drop(&mut self) {
        unsafe {
            let status_bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
            status_bar.removeStatusItem_(self.item);
            let _: () = msg_send![self.item, release];
            let _: () = msg_send![self.enable_item, release];
            let _: () = msg_send![self.state_item, release];
            let _: () = msg_send![self.delegate, release];
            drop(Box::from_raw(self.context));
        }
    }
}

fn delegate_class() -> *const Class {
    static REGISTER: Once = Once::new();
    static mut CLASS: *const Class = std::ptr::null();
    REGISTER.call_once(|| unsafe {
        let mut declaration = ClassDecl::new("PartyGPUIStatusDelegate", class!(NSObject))
            .expect("failed to register status delegate");
        declaration.add_ivar::<*mut c_void>("context");
        declaration.add_method(
            sel!(togglePing:),
            toggle_ping as extern "C" fn(&Object, Sel, id),
        );
        declaration.add_method(
            sel!(openWindow:),
            open_window as extern "C" fn(&Object, Sel, id),
        );
        declaration.add_method(
            sel!(quitApplication:),
            quit_application as extern "C" fn(&Object, Sel, id),
        );
        CLASS = declaration.register();
    });
    unsafe { CLASS }
}

extern "C" fn toggle_ping(this: &Object, _: Sel, _: id) {
    push_action(this, StatusBarAction::Toggle);
}

extern "C" fn open_window(this: &Object, _: Sel, _: id) {
    push_action(this, StatusBarAction::Open);
}

extern "C" fn quit_application(this: &Object, _: Sel, _: id) {
    push_action(this, StatusBarAction::Quit);
}

fn push_action(this: &Object, action: StatusBarAction) {
    unsafe {
        let context = *this.get_ivar::<*mut c_void>("context") as *mut StatusBarContext;
        if let Some(context) = context.as_ref() {
            context
                .actions
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push_back(action);
        }
    }
}

fn ns_string(value: &str) -> id {
    unsafe { NSString::alloc(nil).init_str(value) }
}
