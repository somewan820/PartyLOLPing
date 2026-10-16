#![allow(unexpected_cfgs)]

#[path = "partylolping/app_state.rs"]
mod app_state;
#[path = "partylolping/ping_page.rs"]
mod ping_page;
#[path = "partylolping/room_page.rs"]
mod room_page;
#[path = "partylolping/widgets.rs"]
mod widgets;

#[path = "partylolping/ping_assets.rs"]
mod ping_assets;
#[path = "partylolping/ping_controller.rs"]
mod ping_controller;
#[path = "partylolping/ping_input.rs"]
mod ping_input;
#[path = "partylolping/ping_overlay.rs"]
mod ping_overlay;
#[path = "partylolping/ping_settings.rs"]
mod ping_settings;
#[path = "partylolping/ping_sound.rs"]
mod ping_sound;
#[path = "partylolping/ping_status_bar.rs"]
mod ping_status_bar;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use app_state::*;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::{
    canvas, div, point, prelude::*, px, rgb, size, AnyWindowHandle, App, Bounds, Context, Div,
    Entity, FontWeight, Global, Stateful, UpdateGlobal, Window, WindowBounds, WindowOptions,
};
use party_engine::{PingKind, TriggerChord};
use ping_assets::PingAssets;
use ping_controller::{PingCommand, PingController};

impl Render for PartyLOLPing {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.page == MainPage::Ping {
            return self.ping_page(cx).into_any_element();
        }
        if self.host.is_none() && self.client.is_none() {
            self.landing(cx).into_any_element()
        } else {
            self.joined_view(cx).into_any_element()
        }
    }
}

fn main() {
    if check_assets() {
        return;
    }
    gpui_kit::application().run(|cx: &mut App| {
        gpui_kit::init(cx);
        let assets = match PingAssets::load() {
            Ok(assets) => Arc::new(assets),
            Err(error) => {
                eprintln!("{error}");
                return;
            }
        };
        cx.set_global(PingController::new(assets));
        cx.set_global(MainAppState { window: None });
        PingController::update_global(cx, |ping, cx| ping.restore_state(cx));
        open_main_window(cx);
        cx.spawn(async move |cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            let commands =
                cx.update(|cx| PingController::update_global(cx, |ping, cx| ping.poll(cx)));
            for command in commands {
                match command {
                    PingCommand::OpenWindow => {
                        let _ = cx.update(|cx| open_main_window(cx));
                    }
                    PingCommand::Quit => {
                        let _ = cx.update(|cx| {
                            PingController::update_global(cx, |ping, cx| ping.shutdown(cx));
                            cx.quit();
                        });
                    }
                    PingCommand::Commit(kind) => {
                        let _ = cx.update(|cx| route_global_commit(cx, kind));
                    }
                }
            }
        })
        .detach();
        cx.activate(true);
    });
}

fn local_lan_address() -> Option<String> {
    // Connecting a UDP socket does not send a packet; it asks the kernel which
    // local interface would be used for a private-network destination. This
    // gives the host a copyable fallback address when Bonjour is unavailable.
    let socket = std::net::UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    socket.connect(("192.0.2.1", 9)).ok()?;
    let address = socket.local_addr().ok()?.ip();
    if address.is_loopback() {
        None
    } else {
        Some(address.to_string())
    }
}

fn check_assets() -> bool {
    if !std::env::args().any(|argument| argument == "--check-assets") {
        return false;
    }
    match PingAssets::load() {
        Ok(assets) => {
            let missing = assets.missing_files();
            if missing.is_empty() {
                println!("PASS 九种 Ping 矢量图标与音效完整");
            } else {
                eprintln!("素材缺失：{}", missing.join("、"));
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
    true
}

fn open_main_window(cx: &mut App) {
    let existing = cx.global::<MainAppState>().window;
    if let Some(window) = existing {
        if window
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            return;
        }
    }

    let bounds = Bounds::new(point(px(80.), px(70.)), size(px(1080.), px(700.)));
    let window = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let nickname = cx.new(|cx| {
                    InputState::new(window, cx)
                        .default_value("玩家")
                        .placeholder("你的昵称")
                });
                let address = cx.new(|cx| {
                    InputState::new(window, cx)
                        .default_value("")
                        .placeholder("主机地址")
                });
                let port = cx.new(|cx| InputState::new(window, cx).placeholder("端口"));
                let invite_input =
                    cx.new(|cx| InputState::new(window, cx).placeholder("输入 6 位数字邀请码"));
                cx.new(|_| PartyLOLPing {
                    page: MainPage::Room,
                    selected_room: "未选择房间".to_owned(),
                    status: "准备就绪，发现或创建一个房间".to_owned(),
                    room_status: "尚未加入房间".to_owned(),
                    discovery_status: "尚未搜索局域网房间".to_owned(),
                    ping_status: "选择信号后，将广播给房间成员".to_owned(),
                    capture_status: "尚未采集画面".to_owned(),
                    invite: None,
                    host: None,
                    client: None,
                    room: None,
                    local_member: None,
                    discovered_rooms: Vec::new(),
                    selected_endpoint: None,
                    show_join_dialog: false,
                    next_ping_id: 0,
                    nickname,
                    address,
                    port,
                    invite_input,
                })
            },
        )
        .expect("failed to open PartyLOLPing window");
    cx.global_mut::<MainAppState>().window = Some(window.into());
}

fn route_global_commit(cx: &mut App, kind: PingKind) {
    let Some(window) = cx.global::<MainAppState>().window else {
        return;
    };
    let _ = window.update(cx, |root, _, cx| {
        if let Ok(root) = root.downcast::<PartyLOLPing>() {
            root.update(cx, |this, cx| {
                if this.host.is_some() || this.client.is_some() {
                    this.route_ping(kind.code(), party_engine::PingRoute::broadcast());
                    cx.notify();
                }
            });
        }
    });
}
