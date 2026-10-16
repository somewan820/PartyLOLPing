use super::widgets::*;
use super::*;

impl PartyLOLPing {
    fn leave_room(&mut self) {
        self.host = None;
        self.client = None;
        self.room = None;
        self.local_member = None;
        self.invite = None;
        self.selected_endpoint = None;
        self.show_join_dialog = false;
        self.selected_room = "未选择房间".to_owned();
        self.room_status = "尚未加入房间".to_owned();
        self.status = "已退出房间".to_owned();
        self.ping_status = "选择信号后，将广播给房间成员".to_owned();
        self.capture_status = "尚未采集画面".to_owned();
    }

    fn create_room(&mut self, cx: &mut Context<Self>) {
        let nickname = self.nickname.read(cx).value().trim().to_owned();
        let nickname = if nickname.is_empty() {
            "玩家".to_owned()
        } else {
            nickname
        };
        let invite = party_engine::InviteCode::generate();
        match party_engine::LanHost::start(&nickname, invite.as_str(), 0) {
            Ok((host, port)) => {
                let discovery_available = host.discovery_available();
                let room = Arc::clone(&host.room);
                self.local_member = room.lock().ok().and_then(|room| {
                    room.members()
                        .find(|member| matches!(member.role, party_engine::MemberRole::Host))
                        .map(|member| member.id.clone())
                });
                self.selected_room = format!("{nickname} 的房间");
                self.room = Some(room);
                self.host = Some(host);
                self.client = None;
                self.invite = Some(invite.as_str().to_owned());
                self.room_status = if discovery_available {
                    format!(
                        "房间已创建 · {}:{port}",
                        local_lan_address().unwrap_or_else(|| "Bonjour".to_owned())
                    )
                } else {
                    format!(
                        "房间已创建 · {}:{port} · Bonjour 不可用",
                        local_lan_address().unwrap_or_else(|| "请查看本机局域网地址".to_owned())
                    )
                };
                self.status = if discovery_available {
                    "房间已创建，可以邀请朋友加入".to_owned()
                } else {
                    "房间已创建；请把本机局域网地址、端口和邀请码发给朋友".to_owned()
                };
                self.ping_status = "房间已就绪，可以发送 Ping".to_owned();
                self.capture_status = "尚未采集画面".to_owned();
            }
            Err(error) => self.status = format!("创建失败：{error:?}"),
        }
    }

    pub(crate) fn joined_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let invite = self.invite.clone().unwrap_or_else(|| "------".to_owned());
        let members = self.member_views(cx);
        let member_count = members.len();
        let capture_status = if self.capture_status.is_empty() {
            "尚未采集画面".to_owned()
        } else {
            self.capture_status.clone()
        };
        let assets = cx.global::<PingController>().assets();

        div()
            .flex()
            .size_full()
            .bg(rgb(BACKGROUND))
            .text_color(rgb(TEXT))
            .child(
                div()
                    .w(px(84.))
                    .h_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .p(px(12.))
                    .bg(rgb(SIDEBAR))
                    .border_r_1()
                    .border_color(rgb(BORDER))
                    .child(div().mt(px(6.)).child(brand_mark()))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            .mt(px(28.))
                            .children(members.iter().map(|member| {
                                let color =
                                    if member.is_local && capture_status.starts_with("已采集") {
                                        ACCENT
                                    } else if member.online {
                                        SUCCESS
                                    } else {
                                        TEXT_MUTED
                                    };
                                avatar(&member.name, color, 44.0)
                            })),
                    )
                    .child(div().mt_auto().child(
                        secondary_button("joined-open-ping", "信号").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.page = MainPage::Ping;
                                cx.notify();
                            },
                        )),
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .h(px(72.))
                            .px(px(24.))
                            .bg(rgb(SIDEBAR))
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(
                                div()
                                    .child(
                                        div()
                                            .text_xl()
                                            .font_weight(FontWeight::BOLD)
                                            .child(self.selected_room.clone()),
                                    )
                                    .child(
                                        div()
                                            .mt(px(3.))
                                            .text_xs()
                                            .text_color(rgb(TEXT_SECONDARY))
                                            .child(self.room_status.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(status_badge(format!("{member_count} 位成员"), SUCCESS))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_3()
                                            .h(px(40.))
                                            .px(px(14.))
                                            .rounded_md()
                                            .bg(rgb(ACCENT_DARK))
                                            .border_1()
                                            .border_color(rgb(BORDER_BRIGHT))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(TEXT_SECONDARY))
                                                    .child("邀请码"),
                                            )
                                            .child(
                                                div()
                                                    .text_lg()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(ACCENT))
                                                    .child(invite),
                                            ),
                                    ),
                            ),
                    )
                    .child(member_grid(&members, &capture_status))
                    .child(
                        div()
                            .mx(px(20.))
                            .mb(px(16.))
                            .px(px(14.))
                            .py(px(12.))
                            .flex()
                            .items_center()
                            .gap_3()
                            .rounded_lg()
                            .bg(rgb(PANEL))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .child(primary_button("joined-capture", "共享屏幕").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.capture_screen();
                                    cx.notify();
                                }),
                            ))
                            .child(div().w(px(1.)).h(px(28.)).bg(rgb(BORDER)))
                            .child(voice_ping_button(
                                PingKind::OnMyWay,
                                "joined-ping-way",
                                assets.clone(),
                                cx,
                            ))
                            .child(voice_ping_button(
                                PingKind::Assist,
                                "joined-ping-help",
                                assets.clone(),
                                cx,
                            ))
                            .child(voice_ping_button(
                                PingKind::Missing,
                                "joined-ping-missing",
                                assets.clone(),
                                cx,
                            ))
                            .child(voice_ping_button(
                                PingKind::EnemyVision,
                                "joined-ping-ward",
                                assets,
                                cx,
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(6.))
                                    .text_xs()
                                    .text_color(rgb(TEXT_SECONDARY))
                                    .child(self.ping_status.clone()),
                            )
                            .child(danger_button("joined-leave-room", "退出").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.leave_room();
                                    cx.notify();
                                }),
                            )),
                    ),
            )
    }

    fn member_views(&self, cx: &Context<Self>) -> Vec<RoomMemberView> {
        if let Some(room) = self.room.as_ref() {
            let members = room.lock().ok().map(|room| {
                room.members()
                    .map(|member| RoomMemberView {
                        name: member.display_name.clone(),
                        online: matches!(member.status, party_engine::MemberStatus::Online),
                        is_local: self.local_member.as_ref() == Some(&member.id),
                        is_host: matches!(member.role, party_engine::MemberRole::Host),
                    })
                    .collect::<Vec<_>>()
            });
            if let Some(members) = members.filter(|members| !members.is_empty()) {
                return members;
            }
        }
        let name = self.nickname.read(cx).value().trim().to_owned();
        vec![RoomMemberView {
            name: if name.is_empty() {
                "玩家".to_owned()
            } else {
                name
            },
            online: true,
            is_local: true,
            is_host: false,
        }]
    }

    pub(crate) fn landing(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_room = self.selected_room.clone();
        let show_join_dialog = self.show_join_dialog;
        let discovered_count = self.discovered_rooms.len();

        div()
            .flex()
            .size_full()
            .bg(rgb(BACKGROUND))
            .text_color(rgb(TEXT))
            .child(
                div()
                    .w(px(310.))
                    .h_full()
                    .flex()
                    .flex_col()
                    .p(px(28.))
                    .bg(rgb(SIDEBAR))
                    .border_r_1()
                    .border_color(rgb(BORDER))
                    .child(brand())
                    .child(
                        div()
                            .mt(px(76.))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(ACCENT))
                                    .child("YOUR LOCAL PARTY"),
                            )
                            .child(
                                div()
                                    .mt(px(14.))
                                    .text_3xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child("和朋友，\n保持在同一频道。"),
                            )
                            .child(
                                div()
                                    .mt(px(16.))
                                    .text_sm()
                                    .text_color(rgb(TEXT_SECONDARY))
                                    .child("看见彼此正在做什么，用一个信号代替一段解释。无需开麦，也不打扰。"),
                            ),
                    )
                    .child(
                        div()
                            .mt_auto()
                            .flex()
                            .gap_3()
                            .p(px(14.))
                            .rounded_lg()
                            .bg(rgb(PANEL))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .child(div().text_color(rgb(GOLD)).child("◇"))
                            .child(
                                div()
                                    .child(div().text_sm().font_weight(FontWeight::BOLD).child("只属于你的局域网"))
                                    .child(
                                        div()
                                            .mt(px(5.))
                                            .text_xs()
                                            .text_color(rgb(TEXT_SECONDARY))
                                            .child("不保存屏幕画面与 Ping 历史"),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .h_full()
                    .p(px(30.))
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
                                            .child("开始你的房间"),
                                    )
                                    .child(
                                        div()
                                            .mt(px(6.))
                                            .text_sm()
                                            .text_color(rgb(TEXT_SECONDARY))
                                            .child("创建一个新空间，或加入朋友已经在的房间。"),
                                    ),
                            )
                            .child(
                                secondary_button("landing-open-ping", "桌面信号").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.page = MainPage::Ping;
                                        cx.notify();
                                    }),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .mt(px(22.))
                            .child(
                                action_card(
                                    "landing-create",
                                    "＋",
                                    "创建房间",
                                    "生成邀请码，等待朋友加入",
                                    true,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.create_room(cx);
                                    cx.notify();
                                })),
                            )
                            .child(
                                action_card(
                                    "landing-discover",
                                    "⌁",
                                    "发现房间",
                                    "搜索同一局域网内的房间",
                                    false,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.discover_rooms();
                                    this.status = this.discovery_status.clone();
                                    this.show_join_dialog = false;
                                    cx.notify();
                                })),
                            )
                            .child(
                                action_card(
                                    "landing-manual-join",
                                    "↗",
                                    "手动加入",
                                    "填写朋友显示的地址和端口",
                                    false,
                                )
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.selected_room = "手动加入房间".to_owned();
                                    this.selected_endpoint = None;
                                    this.address.update(cx, |input, input_cx| {
                                        input.set_value(String::new(), window, input_cx);
                                    });
                                    this.port.update(cx, |input, input_cx| {
                                        input.set_value(String::new(), window, input_cx);
                                    });
                                    this.show_join_dialog = true;
                                    this.status = "请输入房主的局域网地址、端口和邀请码".to_owned();
                                    cx.notify();
                                })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .mt(px(14.))
                            .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(SUCCESS)))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_SECONDARY))
                                    .child(self.status.clone()),
                            ),
                    )
                    .when(!self.discovered_rooms.is_empty(), |this| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .mt(px(20.))
                                .mb(px(10.))
                                .child(div().text_sm().font_weight(FontWeight::BOLD).child("附近的房间"))
                                .child(status_badge(format!("{discovered_count} 个可用"), ACCENT)),
                        )
                        .child(
                            div()
                                .id("landing-room-list")
                                .flex()
                                .flex_col()
                                .gap_2()
                                .h(if show_join_dialog { px(138.) } else { px(224.) })
                                .overflow_y_scroll()
                                .children(self.discovered_rooms.iter().enumerate().map(
                                    |(index, room)| {
                                        let room_host = room.host.clone();
                                        let room_port = room.port;
                                        let room_name = room.name.clone();
                                        let is_selected = show_join_dialog
                                            && selected_room.as_str() == room.name;
                                        div()
                                            .id(format!("landing-room-{index}"))
                                            .cursor_pointer()
                                            .h(px(52.))
                                            .px(px(13.))
                                            .flex()
                                            .items_center()
                                            .gap_3()
                                            .rounded_md()
                                            .bg(if is_selected { rgb(ACCENT_DARK) } else { rgb(PANEL) })
                                            .border_1()
                                            .border_color(if is_selected {
                                                rgb(ACCENT)
                                            } else {
                                                rgb(BORDER)
                                            })
                                            .hover(|style| style.bg(rgb(PANEL_HOVER)))
                                            .child(
                                                div()
                                                    .w(px(28.))
                                                    .h(px(28.))
                                                    .rounded_full()
                                                    .bg(if is_selected { rgb(ACCENT) } else { rgb(PANEL_RAISED) })
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .text_xs()
                                                    .text_color(if is_selected { rgb(BACKGROUND) } else { rgb(ACCENT) })
                                                    .child("●"),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .child(div().text_sm().font_weight(FontWeight::BOLD).child(room.name.clone()))
                                                    .child(
                                                        div()
                                                            .mt(px(2.))
                                                            .text_xs()
                                                            .text_color(rgb(TEXT_MUTED))
                                                            .child(format!("{}:{}", room.host, room.port)),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(if is_selected { rgb(ACCENT) } else { rgb(TEXT_MUTED) })
                                                    .child(if is_selected { "已选择" } else { "→" }),
                                            )
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.selected_room = room_name.clone();
                                                this.selected_endpoint = Some((room_host.clone(), room_port));
                                                this.address.update(cx, |input, input_cx| {
                                                    input.set_value(room_host.clone(), window, input_cx);
                                                });
                                                this.port.update(cx, |input, input_cx| {
                                                    input.set_value(room_port.to_string(), window, input_cx);
                                                });
                                                this.show_join_dialog = true;
                                                this.status = format!("已选择：{}", this.selected_room);
                                                cx.notify();
                                            }))
                                    },
                                )),
                        )
                    })
                    .when(self.show_join_dialog, |this| {
                        this.child(
                            panel()
                                .mt(px(14.))
                                .p(px(14.))
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::BOLD)
                                                .child(format!("加入 {}", self.selected_room)),
                                        )
                                        .child(div().text_xs().text_color(rgb(TEXT_MUTED)).child("输入房间信息")),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap_3()
                                        .child(
                                            div()
                                                .w(px(160.))
                                                .child(field_label("你的昵称"))
                                                .child(
                                                    Input::new(&self.nickname)
                                                        .id("landing-nickname")
                                                        .appearance(false)
                                                        .h(px(38.))
                                                        .mt(px(6.))
                                                        .px(px(10.))
                                                        .rounded_md()
                                                        .bg(rgb(BACKGROUND))
                                                        .border_1()
                                                        .border_color(rgb(BORDER)),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .child(field_label("6 位邀请码"))
                                                .child(
                                                    Input::new(&self.invite_input)
                                                        .id("landing-invite")
                                                        .appearance(false)
                                                        .h(px(38.))
                                                        .mt(px(6.))
                                                        .px(px(10.))
                                                        .rounded_md()
                                                        .bg(rgb(BACKGROUND))
                                                        .border_1()
                                                        .border_color(rgb(BORDER)),
                                                ),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap_3()
                                        .child(
                                            div()
                                                .flex_1()
                                                .child(field_label("主机地址（发现失败时手填）"))
                                                .child(
                                                    Input::new(&self.address)
                                                        .id("landing-address")
                                                        .appearance(false)
                                                        .h(px(38.))
                                                        .mt(px(6.))
                                                        .px(px(10.))
                                                        .rounded_md()
                                                        .bg(rgb(BACKGROUND))
                                                        .border_1()
                                                        .border_color(rgb(BORDER)),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .w(px(130.))
                                                .child(field_label("端口"))
                                                .child(
                                                    Input::new(&self.port)
                                                        .id("landing-port")
                                                        .appearance(false)
                                                        .h(px(38.))
                                                        .mt(px(6.))
                                                        .px(px(10.))
                                                        .rounded_md()
                                                        .bg(rgb(BACKGROUND))
                                                        .border_1()
                                                        .border_color(rgb(BORDER)),
                                                ),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap_2()
                                        .child(
                                            secondary_button("landing-cancel-join", "取消").on_click(
                                                cx.listener(|this, _, _, cx| {
                                                    this.show_join_dialog = false;
                                                    cx.notify();
                                                }),
                                            ),
                                        )
                                        .child(
                                            primary_button("landing-join", "加入房间").on_click(
                                                cx.listener(|this, _, _, cx| {
                                                    this.join_selected(cx);
                                                }),
                                            ),
                                        ),
                                ),
                        )
                    })
                    .when(self.discovered_rooms.is_empty(), |this| {
                        this.child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .gap_2()
                                        .text_color(rgb(TEXT_MUTED))
                                        .child(div().text_2xl().text_color(rgb(BORDER_BRIGHT)).child("⌁"))
                                        .child(div().text_sm().child("发现的房间会显示在这里")),
                                ),
                        )
                    }),
            )
    }

    fn discover_rooms(&mut self) {
        self.discovery_status = "正在搜索局域网房间…".to_owned();
        self.discovered_rooms = party_engine::browse_bonjour_rooms(Duration::from_millis(1500));
        self.discovery_status = if self.discovered_rooms.is_empty() {
            "未发现房间；可在加入面板手填主机地址和端口".to_owned()
        } else {
            format!("已发现 {} 个房间", self.discovered_rooms.len())
        };
    }

    fn join_selected(&mut self, cx: &mut Context<Self>) {
        let address = self.address.read(cx).value().to_string();
        let port_text = self.port.read(cx).value().to_string();
        let invite = self.invite_input.read(cx).value().to_string();
        let nickname = self.nickname.read(cx).value().to_string();
        let endpoint = port_text
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .map(|port| (address.clone(), port))
            .filter(|(host, _)| !host.trim().is_empty())
            .or_else(|| self.selected_endpoint.clone());
        let Some((address, port)) = endpoint else {
            self.status = "请选择房间或填写端口".to_owned();
            return;
        };
        match party_engine::lan_connect(&address, port, &invite, &nickname) {
            Ok(client) => {
                self.status = "加入完成".to_owned();
                self.room_status = format!("已加入 · 成员 ID {}", client.member_id);
                self.invite = Some(invite);
                self.ping_status = "已连接到房间，可以发送 Ping".to_owned();
                self.client = Some(client);
            }
            Err(error) => self.status = format!("加入失败：{error:?}"),
        }
        cx.notify();
    }

    pub(crate) fn route_ping(&mut self, kind: u16, route: party_engine::PingRoute) {
        if let Some(client) = self.client.as_mut() {
            match client.send_ping(kind) {
                Ok(()) => self.ping_status = format!("Ping {kind} 已发送到房间"),
                Err(error) => self.ping_status = format!("Ping 发送失败：{error:?}"),
            }
            return;
        }
        let Some(room) = self.room.as_ref().cloned() else {
            self.ping_status = "当前没有活动的房间连接".to_owned();
            return;
        };
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
                    if let Some(host) = self.host.as_ref() {
                        if let Err(error) = host.send_ping(kind) {
                            self.ping_status = format!("Ping 网络发送失败：{error:?}");
                            return;
                        }
                    }
                    self.ping_status = format!(
                        "Ping 已发送：{} 个接收者，{} 个被拒绝",
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
