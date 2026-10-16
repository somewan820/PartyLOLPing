use super::*;

pub(crate) fn member_grid(members: &[RoomMemberView], capture_status: &str) -> Div {
    let mut grid = div().flex().flex_col().flex_1().p(px(20.)).gap_4();
    for row in members.chunks(2) {
        grid = grid.child(
            div().flex().flex_1().gap_4().children(
                row.iter()
                    .map(|member| voice_member_tile(member, capture_status)),
            ),
        );
    }
    grid
}

pub(crate) fn voice_member_tile(member: &RoomMemberView, capture_status: &str) -> Div {
    let (status, color) = if member.is_local && capture_status.contains("失败") {
        ("共享失败", DANGER)
    } else if member.is_local && capture_status.starts_with("已采集") {
        ("最近一帧已发送", ACCENT)
    } else if member.is_local {
        ("尚未共享", TEXT_MUTED)
    } else if member.online {
        ("在线", SUCCESS)
    } else {
        ("离线", TEXT_MUTED)
    };
    panel()
        .flex_1()
        .min_h(px(140.))
        .p(px(14.))
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(status_badge(
                    if member.is_host { "房主" } else { "成员" },
                    color,
                ))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(TEXT_MUTED))
                        .child(if member.is_local { "你" } else { "" }),
                ),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(avatar(&member.name, color, 64.0)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::BOLD)
                        .child(member.name.clone()),
                )
                .child(div().text_xs().text_color(rgb(color)).child(status)),
        )
}

pub(crate) fn avatar(name: &str, color: u32, side: f32) -> Div {
    div()
        .w(px(side))
        .h(px(side))
        .rounded_full()
        .bg(rgb(PANEL_RAISED))
        .border_1()
        .border_color(rgb(color))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px((side * 0.32).max(11.0)))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(color))
        .child(initial(name))
}

pub(crate) fn initial(name: &str) -> String {
    name.chars().next().unwrap_or('玩').to_string()
}

pub(crate) fn voice_ping_button(
    kind: PingKind,
    id: &'static str,
    assets: Arc<PingAssets>,
    cx: &Context<PartyLOLPing>,
) -> Stateful<Div> {
    div()
        .id(id)
        .cursor_pointer()
        .w(px(44.))
        .h(px(44.))
        .rounded_full()
        .bg(rgb(PANEL_RAISED))
        .border_1()
        .border_color(rgb(BORDER))
        .flex()
        .items_center()
        .justify_center()
        .child(ping_icon(assets, kind, 24.0, kind.tint()))
        .hover(|style| style.bg(rgb(PANEL_HOVER)).border_color(rgb(kind.tint())))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.route_ping(kind.code(), party_engine::PingRoute::broadcast());
            cx.notify();
        }))
}

pub(crate) fn brand_mark() -> Div {
    div()
        .w(px(36.))
        .h(px(36.))
        .rounded_lg()
        .bg(rgb(ACCENT))
        .flex()
        .items_center()
        .justify_center()
        .text_lg()
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(BACKGROUND))
        .child("P")
}

pub(crate) fn choice_button(
    label: &'static str,
    selected: bool,
    id: &'static str,
) -> Stateful<Div> {
    button_base(id, label)
        .flex_1()
        .bg(if selected {
            rgb(ACCENT_DARK)
        } else {
            rgb(PANEL_RAISED)
        })
        .border_1()
        .border_color(if selected { rgb(ACCENT) } else { rgb(BORDER) })
        .text_color(if selected {
            rgb(ACCENT)
        } else {
            rgb(TEXT_SECONDARY)
        })
}

pub(crate) fn setting_stepper(
    label: &'static str,
    value: String,
    down_id: &'static str,
    up_id: &'static str,
    cx: &Context<PartyLOLPing>,
    current: f64,
    step: f64,
    is_scale: bool,
) -> Div {
    div()
        .flex_1()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_xs().text_color(rgb(TEXT_SECONDARY)).child(label))
                .child(div().text_xs().text_color(rgb(TEXT)).child(value)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .mt(px(6.))
                .child(
                    secondary_button(down_id, "−").on_click(cx.listener(move |_, _, _, cx| {
                        let next = if is_scale {
                            (current - step).max(0.75)
                        } else {
                            (current - step).max(0.0)
                        };
                        if is_scale {
                            PingController::update_global(cx, |ping, cx| ping.set_scale(next, cx));
                        } else {
                            PingController::update_global(cx, |ping, _| ping.set_volume(next));
                        }
                        cx.notify();
                    })),
                )
                .child(
                    div()
                        .flex_1()
                        .h(px(4.))
                        .rounded_full()
                        .bg(rgb(ACCENT_DARK))
                        .child(
                            div()
                                .h_full()
                                .w(px((120.0 * current).max(4.0) as f32))
                                .rounded_full()
                                .bg(rgb(ACCENT)),
                        ),
                )
                .child(
                    secondary_button(up_id, "＋").on_click(cx.listener(move |_, _, _, cx| {
                        let next = if is_scale {
                            (current + step).min(1.5)
                        } else {
                            (current + step).min(1.0)
                        };
                        if is_scale {
                            PingController::update_global(cx, |ping, cx| ping.set_scale(next, cx));
                        } else {
                            PingController::update_global(cx, |ping, _| ping.set_volume(next));
                        }
                        cx.notify();
                    })),
                ),
        )
}

pub(crate) fn preview_button(
    kind: PingKind,
    current: PingKind,
    cx: &Context<PartyLOLPing>,
) -> Stateful<Div> {
    let assets = cx.global::<PingController>().assets();
    let selected = kind == current;
    div()
        .id(kind.key())
        .cursor_pointer()
        .flex_1()
        .h(px(42.))
        .px(px(10.))
        .flex()
        .items_center()
        .gap_2()
        .rounded_md()
        .bg(if selected {
            rgb(ACCENT_DARK)
        } else {
            rgb(PANEL_RAISED)
        })
        .border_1()
        .border_color(if selected { rgb(ACCENT) } else { rgb(BORDER) })
        .child(ping_icon(assets, kind, 24.0, kind.tint()))
        .child(div().text_xs().child(kind.title()))
        .hover(|style| style.bg(rgb(PANEL_HOVER)))
        .on_click(cx.listener(move |_, _, _, cx| {
            PingController::update_global(cx, |ping, cx| ping.preview(cx, kind));
            cx.notify();
        }))
}

pub(crate) fn ping_icon(
    assets: Arc<PingAssets>,
    kind: PingKind,
    side: f32,
    color: u32,
) -> impl IntoElement {
    canvas(
        move |_, _, _| {},
        move |bounds, _, window, _| {
            assets.paint_icon(window, kind, bounds, rgb(color));
        },
    )
    .w(px(side))
    .h(px(side))
}

pub(crate) fn brand() -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(36.))
                .h(px(36.))
                .rounded_lg()
                .bg(rgb(ACCENT))
                .flex()
                .items_center()
                .justify_center()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(BACKGROUND))
                .child("P"),
        )
        .child(
            div()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::BOLD)
                        .child("PartyLOLPing"),
                )
                .child(
                    div()
                        .mt(px(1.))
                        .text_xs()
                        .text_color(rgb(TEXT_MUTED))
                        .child("LOCAL PARTY"),
                ),
        )
}

pub(crate) fn panel() -> Div {
    div()
        .rounded_lg()
        .bg(rgb(PANEL))
        .border_1()
        .border_color(rgb(BORDER))
}

pub(crate) fn field_label(label: &'static str) -> Div {
    div().text_xs().text_color(rgb(TEXT_SECONDARY)).child(label)
}

pub(crate) fn status_badge(label: impl IntoElement, color: u32) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px(px(10.))
        .py(px(5.))
        .rounded_full()
        .bg(rgb(color).opacity(0.10))
        .text_xs()
        .text_color(rgb(color))
        .child(div().w(px(5.)).h(px(5.)).rounded_full().bg(rgb(color)))
        .child(label)
}

pub(crate) fn button_base(id: &'static str, label: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .cursor_pointer()
        .h(px(38.))
        .px(px(15.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_sm()
        .font_weight(FontWeight::BOLD)
        .child(label)
}

pub(crate) fn primary_button(id: &'static str, label: &'static str) -> Stateful<Div> {
    button_base(id, label)
        .bg(rgb(ACCENT))
        .text_color(rgb(BACKGROUND))
        .hover(|style| style.bg(rgb(ACCENT_HOVER)))
}

pub(crate) fn secondary_button(id: &'static str, label: &'static str) -> Stateful<Div> {
    button_base(id, label)
        .bg(rgb(PANEL_RAISED))
        .border_1()
        .border_color(rgb(BORDER_BRIGHT))
        .text_color(rgb(TEXT))
        .hover(|style| style.bg(rgb(PANEL_HOVER)))
}

pub(crate) fn danger_button(id: &'static str, label: &'static str) -> Stateful<Div> {
    button_base(id, label)
        .bg(rgb(DANGER_BACKGROUND))
        .border_1()
        .border_color(rgb(DANGER).opacity(0.35))
        .text_color(rgb(DANGER))
        .hover(|style| style.bg(rgb(DANGER).opacity(0.16)))
}

pub(crate) fn action_card(
    id: &'static str,
    symbol: &'static str,
    title: &'static str,
    description: &'static str,
    primary: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .cursor_pointer()
        .flex_1()
        .h(px(108.))
        .p(px(15.))
        .flex()
        .flex_col()
        .justify_between()
        .rounded_lg()
        .bg(if primary {
            rgb(ACCENT_DARK)
        } else {
            rgb(PANEL)
        })
        .border_1()
        .border_color(if primary {
            rgb(ACCENT).opacity(0.65)
        } else {
            rgb(BORDER)
        })
        .hover(|style| style.bg(rgb(PANEL_HOVER)).border_color(rgb(ACCENT)))
        .child(
            div()
                .w(px(28.))
                .h(px(28.))
                .rounded_md()
                .bg(if primary {
                    rgb(ACCENT)
                } else {
                    rgb(PANEL_RAISED)
                })
                .flex()
                .items_center()
                .justify_center()
                .text_lg()
                .text_color(if primary {
                    rgb(BACKGROUND)
                } else {
                    rgb(ACCENT)
                })
                .child(symbol),
        )
        .child(
            div()
                .child(div().text_sm().font_weight(FontWeight::BOLD).child(title))
                .child(
                    div()
                        .mt(px(4.))
                        .text_xs()
                        .text_color(rgb(TEXT_SECONDARY))
                        .child(description),
                ),
        )
}
