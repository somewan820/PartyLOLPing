use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui_kit::component::input::{Input, InputState};
use gpui_kit::{
    div, point, prelude::*, px, rgb, size, AnyElement, App, Bounds, Context, Entity, FontWeight,
    Window, WindowBounds, WindowOptions,
};

struct PartyLOLPing {
    selected_room: String,
    status: String,
    room_status: String,
    discovery_status: String,
    ping_status: String,
    capture_status: String,
    invite: Option<String>,
    host: Option<party_engine::LanHost>,
    client: Option<party_engine::LanClient>,
    room: Option<Arc<Mutex<party_engine::Room>>>,
    local_member: Option<party_engine::MemberId>,
    discovered_rooms: Vec<party_engine::DiscoveredRoom>,
    selected_endpoint: Option<(String, u16)>,
    show_join_dialog: bool,
    next_ping_id: u64,
    nickname: Entity<InputState>,
    address: Entity<InputState>,
    port: Entity<InputState>,
    invite_input: Entity<InputState>,
}

impl PartyLOLPing {
    fn leave_room(&mut self) {
        self.host = None;
        self.client = None;
        self.room = None;
        self.local_member = None;
        self.invite = None;
        self.selected_endpoint = None;
        self.show_join_dialog = false;
        self.room_status = "尚未加入房间".to_owned();
        self.status = "已退出房间".to_owned();
        self.ping_status.clear();
        self.capture_status.clear();
    }

    fn joined_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let invite = self.invite.clone().unwrap_or_default();
        let room_status = self.room_status.clone();
        let capture_status = self.capture_status.clone();
        let ping_status = self.ping_status.clone();
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x101419))
            .text_color(rgb(0xe7edf2))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(64.))
                    .px(px(24.))
                    .bg(rgb(0x171e25))
                    .border_b_1()
                    .border_color(rgb(0x28333d))
                    .child(
                        div()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child("房间工作区"),
                            )
                            .child(
                                div()
                                    .mt(px(4.))
                                    .text_xs()
                                    .text_color(rgb(0x82909d))
                                    .child(room_status),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .px(px(12.))
                                    .py(px(7.))
                                    .rounded_md()
                                    .bg(rgb(0x263742))
                                    .text_sm()
                                    .child(format!("邀请码：{invite}")),
                            )
                            .child(
                                div()
                                    .id("joined-leave-room")
                                    .cursor_pointer()
                                    .px(px(12.))
                                    .py(px(7.))
                                    .rounded_md()
                                    .bg(rgb(0x8f3d4b))
                                    .text_sm()
                                    .child("退出房间")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.leave_room();
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_4()
                    .p(px(24.))
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(card("成员", "你已在线\n等待其他成员加入", 0x1c252c))
                            .child(card("屏幕共享", capture_status, 0x1c252c)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .p(px(16.))
                            .rounded_lg()
                            .bg(rgb(0x1c252c))
                            .child(div().font_weight(FontWeight::BOLD).child("快速 Ping"))
                            .child(ping_button("正在路上", "joined-ping-way", 1, cx))
                            .child(ping_button("需要帮助", "joined-ping-help", 2, cx))
                            .child(ping_button("敌人消失", "joined-ping-missing", 3, cx))
                            .child(ping_button("区域已做视野", "joined-ping-ward", 4, cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p(px(16.))
                            .rounded_lg()
                            .bg(rgb(0x1c252c))
                            .child(div().text_sm().text_color(rgb(0x93a8b5)).child(ping_status))
                            .child(
                                div()
                                    .id("joined-capture")
                                    .cursor_pointer()
                                    .px(px(12.))
                                    .py(px(7.))
                                    .rounded_md()
                                    .bg(rgb(0x263742))
                                    .text_sm()
                                    .child("采集并共享屏幕")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.capture_screen();
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
    }

    fn landing(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .gap_4()
            .bg(rgb(0x101419))
            .text_color(rgb(0xe7edf2))
            .child(
                div()
                    .text_3xl()
                    .font_weight(FontWeight::BOLD)
                    .child("PartyLOLPing"),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .mt(px(14.))
                    .child(
                        div()
                            .id("landing-discover")
                            .cursor_pointer()
                            .w(px(220.))
                            .h(px(130.))
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap_2()
                            .rounded_lg()
                            .bg(rgb(0x20313a))
                            .border_1()
                            .border_color(rgb(0x38b7cf))
                            .child(div().text_xl().child("发现房间"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.discover_rooms();
                                this.status = this.discovery_status.clone();
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("landing-create")
                            .cursor_pointer()
                            .w(px(220.))
                            .h(px(130.))
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap_2()
                            .rounded_lg()
                            .bg(rgb(0x2c88a0))
                            .child(div().text_xl().child("创建房间"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                let invite = party_engine::InviteCode::generate();
                                match party_engine::LanHost::start("玩家", invite.as_str(), 0) {
                                    Ok((host, port)) => {
                                        this.local_member =
                                            host.room.lock().ok().and_then(|room| {
                                                room.members()
                                                    .find(|member| {
                                                        matches!(
                                                            member.role,
                                                            party_engine::MemberRole::Host
                                                        )
                                                    })
                                                    .map(|member| member.id.clone())
                                            });
                                        this.host = Some(host);
                                        this.invite = Some(invite.as_str().to_owned());
                                        this.room_status = format!("房间已创建，端口 {port}");
                                        this.status = "房间已创建".to_owned();
                                    }
                                    Err(error) => this.status = format!("创建失败：{error:?}"),
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(div().mt(px(10.)).flex().flex_col().gap_2().when(
                !self.discovered_rooms.is_empty(),
                |this| {
                    this.child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .child("选择一个房间"),
                    )
                    .children(
                        self.discovered_rooms
                            .iter()
                            .enumerate()
                            .map(|(index, room)| {
                                let room_host = room.host.clone();
                                let room_port = room.port;
                                let room_name = room.name.clone();
                                div()
                                    .id(format!("landing-room-{index}"))
                                    .cursor_pointer()
                                    .rounded_md()
                                    .bg(rgb(0x1c252c))
                                    .px(px(12.))
                                    .py(px(8.))
                                    .text_sm()
                                    .child(format!("{}  {}:{}", room_name, room_host, room_port))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.selected_room = room_name.clone();
                                        this.selected_endpoint =
                                            Some((room_host.clone(), room_port));
                                        this.address.update(cx, |input, input_cx| {
                                            input.set_value(room_host.clone(), window, input_cx);
                                        });
                                        this.port.update(cx, |input, input_cx| {
                                            input.set_value(
                                                room_port.to_string(),
                                                window,
                                                input_cx,
                                            );
                                        });
                                        this.show_join_dialog = true;
                                        this.status = format!("已选择：{}", this.selected_room);
                                        cx.notify();
                                    }))
                            }),
                    )
                    .when(self.show_join_dialog, |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .mt(px(8.))
                                .p(px(12.))
                                .rounded_md()
                                .bg(rgb(0x20313a))
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::BOLD)
                                        .child(format!("加入：{}", self.selected_room)),
                                )
                                .child(Input::new(&self.invite_input).h(px(34.)))
                                .child(
                                    div()
                                        .flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .id("landing-join")
                                                .cursor_pointer()
                                                .rounded_md()
                                                .bg(rgb(0x2c88a0))
                                                .px(px(12.))
                                                .py(px(8.))
                                                .text_sm()
                                                .child("加入房间")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.join_selected(cx);
                                                })),
                                        )
                                        .child(
                                            div()
                                                .id("landing-cancel-join")
                                                .cursor_pointer()
                                                .rounded_md()
                                                .bg(rgb(0x263742))
                                                .px(px(12.))
                                                .py(px(8.))
                                                .text_sm()
                                                .child("取消")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.show_join_dialog = false;
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        )
                    })
                },
            ))
    }

    fn discover_rooms(&mut self) {
        self.discovery_status = "正在搜索局域网房间…".to_owned();
        let rooms = party_engine::browse_bonjour_rooms(Duration::from_millis(350));
        self.discovered_rooms = if rooms.is_empty() {
            vec![
                party_engine::DiscoveredRoom {
                    name: "演示房间 · 晨星".to_owned(),
                    host: "127.0.0.1".to_owned(),
                    port: 40000,
                },
                party_engine::DiscoveredRoom {
                    name: "演示房间 · 极光".to_owned(),
                    host: "192.168.10.21".to_owned(),
                    port: 40001,
                },
                party_engine::DiscoveredRoom {
                    name: "演示房间 · 暮光".to_owned(),
                    host: "192.168.10.35".to_owned(),
                    port: 40002,
                },
                party_engine::DiscoveredRoom {
                    name: "演示房间 · 流星".to_owned(),
                    host: "192.168.10.48".to_owned(),
                    port: 40003,
                },
                party_engine::DiscoveredRoom {
                    name: "演示房间 · 星河".to_owned(),
                    host: "192.168.10.60".to_owned(),
                    port: 40004,
                },
            ]
        } else {
            rooms
        };
        self.discovery_status = format!("已发现 {} 个房间", self.discovered_rooms.len());
    }

    fn join_selected(&mut self, cx: &mut Context<Self>) {
        let address = self.address.read(cx).value().to_string();
        let port_text = self.port.read(cx).value().to_string();
        let invite = self.invite_input.read(cx).value().to_string();
        let nickname = self.nickname.read(cx).value().to_string();
        let endpoint = self
            .selected_endpoint
            .clone()
            .or_else(|| port_text.parse::<u16>().ok().map(|port| (address, port)));
        let Some((address, port)) = endpoint else {
            self.status = "请选择房间或填写端口".to_owned();
            return;
        };
        match party_engine::lan_connect(&address, port, &invite, &nickname) {
            Ok(client) => {
                self.status = "加入完成".to_owned();
                self.room_status = format!("已加入，成员 ID：{}", client.member_id);
                self.client = Some(client);
            }
            Err(error) => self.status = format!("加入失败：{error:?}"),
        }
        cx.notify();
    }

    fn route_ping(&mut self, kind: u16, route: party_engine::PingRoute) {
        let Some(room) = self.room.as_ref().cloned() else {
            self.ping_status = "当前没有活动的房间连接".to_owned();
            return;
        };
        if let Some(client) = self.client.as_mut() {
            match client.send_ping(kind) {
                Ok(()) => self.ping_status = format!("Ping {kind} 已发送到房间"),
                Err(error) => self.ping_status = format!("Ping 发送失败：{error:?}"),
            }
            return;
        }
        let Some(sender) = self.local_member.clone() else {
            self.ping_status = "没有本地发送者，无法路由 Ping".to_owned();
            return;
        };

        self.next_ping_id = self.next_ping_id.saturating_add(1);
        let event = party_engine::PingEvent {
            id: format!("ui-ping-{}", self.next_ping_id),
            sender,
            kind,
            route,
            ttl: Duration::from_secs(3),
        };
        match room.lock() {
            Ok(mut room) => match room.route_ping(event) {
                Ok(routed) => {
                    self.ping_status = format!(
                        "本地路由：{} 个接收者，{} 个被拒绝",
                        routed.accepted.len(),
                        routed.rejected.len()
                    );
                }
                Err(error) => {
                    self.ping_status = format!("Ping 路由失败：{error:?}");
                }
            },
            Err(_) => {
                self.ping_status = "房间状态不可用，无法路由 Ping".to_owned();
            }
        };
    }

    fn capture_screen(&mut self) {
        match party_engine::capture_screen_jpeg() {
            Ok(frame) => {
                if let Some(client) = self.client.as_mut() {
                    match client.send_frame(&frame) {
                        Ok(()) => {
                            self.capture_status =
                                format!("已采集并发送 JPEG（{} 字节）", frame.len())
                        }
                        Err(error) => {
                            self.capture_status = format!("已在本地采集，但发送失败：{error:?}")
                        }
                    }
                } else {
                    self.capture_status = format!("已采集本地 JPEG（{} 字节）", frame.len());
                }
            }
            Err(error) => {
                self.capture_status = format!("本地 JPEG 采集失败：{error}");
            }
        }
    }
}

impl Render for PartyLOLPing {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.host.is_none() && self.client.is_none() {
            return self.landing(cx).into_any_element();
        }
        return self.joined_view(cx).into_any_element();

        #[allow(unreachable_code)]
        let joined = self.host.is_some() || self.client.is_some();
        let invite_label = self.invite.clone().unwrap_or_else(|| "创建房间".to_owned());
        let member_count = self
            .room
            .as_ref()
            .and_then(|room| room.lock().ok().map(|room| room.members().count()))
            .unwrap_or(0);
        let member_summary = format!("{member_count} / 6 members\n{}", self.room_status);
        let room_subtitle = format!("局域网房间 · {member_count} / 6 名成员");
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x101419))
            .text_color(rgb(0xe7edf2))
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(58.))
                    .px(px(22.))
                    .bg(rgb(0x171e25))
                    .border_b_1()
                    .border_color(rgb(0x28333d))
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .child("PartyLOLPing"),
                    )
                    .child(
                        div()
                            .ml(px(12.))
                            .text_sm()
                            .text_color(rgb(0x82909d))
                            .child("局域网工作区"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(
                        div()
                            .w(px(250.))
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p(px(16.))
                            .bg(rgb(0x141b21))
                            .border_r_1()
                            .border_color(rgb(0x28333d))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .mb(px(8.))
                                    .child(div().font_weight(FontWeight::BOLD).child("房间"))
                                    .child(
                                        div()
                                            .px(px(10.))
                                            .py(px(5.))
                                            .rounded_md()
                                            .bg(rgb(0x2c88a0))
                                            .text_sm()
                                            .id("create-room")
                                            .cursor_pointer()
                                            .child("创建")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                let nickname =
                                                    this.nickname.read(cx).value().to_string();
                                                let nickname = if nickname.trim().is_empty() {
                                                    "玩家".to_owned()
                                                } else {
                                                    nickname
                                                };
                                                let invite = party_engine::InviteCode::generate();
                                                match party_engine::LanHost::start(
                                                    &nickname,
                                                    invite.as_str(),
                                                    0,
                                                ) {
                                                    Ok((host, port)) => {
                                                        let room = Arc::clone(&host.room);
                                                        let local_member = room
                                                            .lock()
                                                            .ok()
                                                            .and_then(|room| {
                                                                room.members()
                                                                    .find(|member| {
                                                                        matches!(
                                                                            member.role,
                                                                            party_engine::MemberRole::Host
                                                                        )
                                                                    })
                                                                    .map(|member| member.id.clone())
                                                            });
                                                        this.host = Some(host);
                                                        this.client = None;
                                                        this.room = Some(room);
                                                        this.local_member = local_member;
                        this.selected_endpoint = None;
                                                        this.invite =
                                                            Some(invite.as_str().to_owned());
                                                        this.room_status =
                                                            format!("正在监听端口 {port}");
                                                        this.status = "房间已创建".to_owned();
                                                        this.ping_status =
                                                            "可以测试 Ping 路由"
                                                                .to_owned();
                                                        this.port.update(cx, |input, input_cx| {
                                                            input.set_value(
                                                                port.to_string(),
                                                                window,
                                                                input_cx,
                                                            );
                                                        });
                                                    }
                                                    Err(error) => {
                                                        this.room_status =
                                                            format!("创建 failed: {error:?}");
                                                        this.status = "房间创建失败"
                                                            .to_owned();
                                                    }
                                                }
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0x93a8b5))
                                            .child(self.discovery_status.clone()),
                                    )
                                    .child(
                                        div()
                                            .id("discover-rooms")
                                            .cursor_pointer()
                                            .rounded_md()
                                            .px(px(8.))
                                            .py(px(5.))
                                            .bg(rgb(0x263742))
                                            .text_xs()
                                            .child("发现房间")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.discover_rooms();
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .children(self.discovered_rooms.iter().enumerate().map(
                                |(index, room)| {
                                    let room_name = room.name.clone();
                                    let room_host = room.host.clone();
                                    let room_port = room.port;
                                    let room_label =
                                        format!("{} · {}:{}", room_name, room_host, room_port);
                                    div()
                                        .id(format!("discovered-room-{index}"))
                                        .cursor_pointer()
                                        .rounded_md()
                                        .px(px(8.))
                                        .py(px(5.))
                                        .bg(rgb(0x1c252c))
                                        .text_xs()
                                        .text_color(rgb(0x93a8b5))
                                        .child(room_label)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.selected_room = room_name.clone();
                                            this.selected_endpoint =
                                                Some((room_host.clone(), room_port));
                                            this.discovery_status = format!(
                                                "已选择 {}:{}",
                                                room_host, room_port
                                            );
                                            cx.notify();
                                        }))
                                },
                            ))
                            .child(Input::new(&self.nickname).h(px(34.)))
                            .child(Input::new(&self.address).h(px(34.)))
                            .child(Input::new(&self.port).h(px(34.)))
                            .child(Input::new(&self.invite_input).h(px(34.)))
                            .child(
                                div()
                                    .id("join-room")
                                    .cursor_pointer()
                                    .rounded_md()
                                    .bg(rgb(0x2c88a0))
                                    .px(px(12.))
                                    .py(px(8.))
                                    .child("加入房间")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let manual_address =
                                            this.address.read(cx).value().to_string();
                                        let manual_port = this.port.read(cx).value().to_string();
                                        let invite = this.invite_input.read(cx).value().to_string();
                                        let nickname = this.nickname.read(cx).value().to_string();
                                        if nickname.trim().is_empty() {
                                            this.room_status = "加入失败：昵称不能为空".to_owned();
                                            this.status = "加入失败".to_owned();
                                            cx.notify();
                                            return;
                                        }
                                        let endpoint = this
                                            .selected_endpoint
                                            .clone()
                                            .map(Ok)
                                            .unwrap_or_else(|| {
                                                manual_port.parse::<u16>().map(|port| {
                                                    (manual_address.clone(), port)
                                                })
                                            });
                                        match endpoint {
                                            Ok((address, port)) => match party_engine::lan_connect(
                                                &address, port, &invite, &nickname,
                                            ) {
                                                Ok(client) => {
                                                    let member = client.member_id.clone();
                                                    this.host = None;
                                                    this.room = None;
                                                    this.local_member = None;
                                                    this.client = Some(client);
                                                    this.invite = Some(invite);
                                                    this.room_status =
                                                        format!("已加入，成员 ID：{member}");
                                                    this.status = "加入完成".to_owned();
                                                    this.ping_status = "已连接到房间".to_owned();
                                                }
                                                Err(error) => {
                                                    this.room_status =
                                                        format!("加入失败: {error:?}");
                                                    this.status = "加入失败".to_owned();
                                                }
                                            },
                                            Err(_) => {
                                                this.room_status =
                                                    "加入失败: invalid port".to_owned();
                                                this.status = "加入失败".to_owned();
                                            }
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .rounded_md()
                                    .p(px(12.))
                                    .bg(rgb(0x20313a))
                                    .border_1()
                                    .border_color(rgb(0x38b7cf))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::BOLD)
                                            .child(self.selected_room.clone()),
                                    )
                                    .child(
                                        div()
                                            .mt(px(5.))
                                            .text_xs()
                                            .text_color(rgb(0x93a8b5))
                                            .child(self.room_status.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .mt_auto()
                                    .rounded_md()
                                    .p(px(12.))
                                    .bg(rgb(0x1c252c))
                                    .child(div().text_sm().child("Rust 后端"))
                                    .child(
                                        div()
                                            .mt(px(5.))
                                            .text_xs()
                                            .text_color(rgb(0x75c995))
                                            .child(self.status.clone()),
                                    ),
                            )
                            .when(joined, |this| {
                                this.w(px(0.)).p(px(0.)).opacity(0.)
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .p(px(24.))
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .child(
                                                div()
                                                    .text_2xl()
                                                    .font_weight(FontWeight::BOLD)
                                                    .child(self.selected_room.clone()),
                                            )
                                            .child(
                                                div()
                                                    .mt(px(5.))
                                                    .text_sm()
                                                    .text_color(rgb(0x82909d))
                                                    .child(room_subtitle),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .px(px(14.))
                                            .py(px(8.))
                                            .rounded_md()
                                            .bg(rgb(0x2c88a0))
                                            .id("share-invite")
                                            .cursor_pointer()
                                            .child(invite_label)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if let Some(invite) = &this.invite {
                                                    this.status = format!("邀请码：{invite}");
                                                }
                                                cx.notify();
                                            })),
                                    )
                                    .when(joined, |this| {
                                        this.child(
                                            div()
                                                .id("leave-room")
                                                .cursor_pointer()
                                                .rounded_md()
                                                .bg(rgb(0x8f3d4b))
                                                .px(px(12.))
                                                .py(px(8.))
                                                .text_sm()
                                                .child("退出房间")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.leave_room();
                                                    cx.notify();
                                                })),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_4()
                                    .child(card("成员", member_summary, 0x1c252c))
                                    .child(card(
                                        "屏幕共享",
                                        "JPEG 采集\n正在通过 Rust 发送到远端",
                                        0x1c252c,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .p(px(12.))
                                    .rounded_lg()
                                    .bg(rgb(0x1c252c))
                                    .child(
                                        div()
                                            .id("capture-screen-jpeg")
                                            .cursor_pointer()
                                            .rounded_md()
                                            .px(px(12.))
                                            .py(px(8.))
                                            .bg(rgb(0x2c88a0))
                                            .text_sm()
                                            .child("采集并发送 JPEG")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.capture_screen();
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_xs()
                                            .text_color(rgb(0x93a8b5))
                                            .child(self.capture_status.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .p(px(16.))
                                    .rounded_lg()
                                    .bg(rgb(0x1c252c))
                                    .child(div().font_weight(FontWeight::BOLD).child("快速 Ping"))
                                    .child(
                                        div()
                                            .flex()
                                            .gap_2()
                                            .child(ping_button(
                                                "正在路上",
                                                "ping-on-my-way",
                                                1,
                                                cx,
                                            ))
                                            .child(ping_button("需要帮助", "ping-need-help", 2, cx))
                                            .child(ping_button(
                                                "敌人消失",
                                                "ping-enemy-missing",
                                                3,
                                                cx,
                                            ))
                                            .child(ping_button(
                                                "区域已做视野",
                                                "ping-area-warded",
                                                4,
                                                cx,
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x93a8b5))
                                    .child(self.ping_status.clone()),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn card<T, B>(title: T, body: B, color: u32) -> impl IntoElement
where
    T: IntoElement,
    B: IntoElement,
{
    div()
        .flex_1()
        .min_h(px(150.))
        .p(px(16.))
        .rounded_lg()
        .bg(rgb(color))
        .child(div().font_weight(FontWeight::BOLD).child(title))
        .child(
            div()
                .mt(px(16.))
                .text_sm()
                .text_color(rgb(0x93a8b5))
                .child(body),
        )
}

fn ping_button(
    label: &'static str,
    id: &'static str,
    kind: u16,
    cx: &Context<PartyLOLPing>,
) -> impl IntoElement {
    div()
        .id(id)
        .cursor_pointer()
        .px(px(12.))
        .py(px(8.))
        .rounded_md()
        .bg(rgb(0x263742))
        .text_sm()
        .child(label)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.route_ping(kind, party_engine::PingRoute::broadcast());
            cx.notify();
        }))
}

fn main() {
    gpui_kit::application().run(|cx: &mut App| {
        gpui_kit::init(cx);
        cx.spawn(async move |cx| {
            let bounds = Bounds::new(point(px(100.), px(100.)), size(px(980.), px(640.)));
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| {
                    let nickname = cx.new(|cx| InputState::new(window, cx).default_value("玩家"));
                    let address =
                        cx.new(|cx| InputState::new(window, cx).default_value("127.0.0.1"));
                    let port = cx.new(|cx| InputState::new(window, cx));
                    let invite_input = cx.new(|cx| InputState::new(window, cx));
                    cx.new(|_| PartyLOLPing {
                        selected_room: "Main room".to_owned(),
                        status: "Ready".to_owned(),
                        room_status: "No room created".to_owned(),
                        discovery_status: "Bonjour discovery is idle".to_owned(),
                        ping_status: "Choose a room before routing a Ping".to_owned(),
                        capture_status: "No local JPEG captured".to_owned(),
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
        })
        .detach();
        cx.activate(true);
    });
}
