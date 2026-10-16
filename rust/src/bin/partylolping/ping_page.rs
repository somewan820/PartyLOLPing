use super::widgets::*;
use super::*;

impl PartyLOLPing {
    pub(crate) fn ping_page(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let (
            enabled,
            awaiting_permission,
            message,
            chord,
            volume,
            scale,
            preview_view,
            preview_kind,
        ) = {
            let ping = cx.global::<PingController>();
            (
                ping.enabled(),
                ping.awaiting_permission(),
                ping.message().to_owned(),
                ping.chord(),
                ping.volume(),
                ping.scale(),
                ping.preview_view(),
                ping.preview_kind(),
            )
        };
        let toggle_label = if enabled || awaiting_permission {
            "关闭 Ping"
        } else {
            "启用 Ping"
        };

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
                            .mt(px(70.))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(ACCENT))
                                    .child("DESKTOP SIGNALS"),
                            )
                            .child(
                                div()
                                    .mt(px(14.))
                                    .text_3xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child("把信号，\n带到桌面。"),
                            )
                            .child(
                                div()
                                    .mt(px(16.))
                                    .text_sm()
                                    .text_color(rgb(TEXT_SECONDARY))
                                    .child(
                                        "一个手势，九种提醒。无需切换窗口，也不用打断当前操作。",
                                    ),
                            ),
                    )
                    .child(div().mt_auto().child(
                        secondary_button("ping-back-room", "返回局域网房间").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.page = MainPage::Room;
                                cx.notify();
                            },
                        )),
                    )),
            )
            .child(
                div()
                    .id("desktop-ping-page")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .h_full()
                    .p(px(30.))
                    .gap_4()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child("桌面信号"),
                            )
                            .child(
                                div()
                                    .mt(px(6.))
                                    .text_sm()
                                    .text_color(rgb(TEXT_SECONDARY))
                                    .child(
                                        "按住组合键 0.18 秒，移动选择，松开发送。Esc / 右键取消。",
                                    ),
                            ),
                    )
                    .child(
                        panel()
                            .p(px(16.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::BOLD)
                                                    .child("全局 Ping"),
                                            )
                                            .child(status_badge(
                                                if enabled {
                                                    "已开启"
                                                } else if awaiting_permission {
                                                    "等待授权"
                                                } else {
                                                    "已关闭"
                                                },
                                                if enabled { SUCCESS } else { GOLD },
                                            )),
                                    )
                                    .child(primary_button("ping-toggle", toggle_label).on_click(
                                        cx.listener(|_, _, _, cx| {
                                            let enable = {
                                                let ping = cx.global::<PingController>();
                                                !(ping.enabled() || ping.awaiting_permission())
                                            };
                                            PingController::update_global(cx, |ping, cx| {
                                                ping.set_enabled(enable, cx)
                                            });
                                            cx.notify();
                                        }),
                                    )),
                            )
                            .child(
                                div()
                                    .mt(px(10.))
                                    .text_xs()
                                    .text_color(rgb(TEXT_SECONDARY))
                                    .child(message),
                            )
                            .when(awaiting_permission, |this| {
                                this.child(
                                    div()
                                        .flex()
                                        .gap_2()
                                        .mt(px(12.))
                                        .child(
                                            secondary_button(
                                                "ping-open-accessibility",
                                                "打开辅助功能设置",
                                            )
                                            .on_click(
                                                cx.listener(|_, _, _, _| {
                                                    ping_settings::open_accessibility_settings();
                                                }),
                                            ),
                                        )
                                        .child(
                                            secondary_button(
                                                "ping-check-permission",
                                                "已授权，重新检查",
                                            )
                                            .on_click(
                                                cx.listener(|_, _, _, cx| {
                                                    PingController::update_global(
                                                        cx,
                                                        |ping, cx| ping.check_permission(cx),
                                                    );
                                                    cx.notify();
                                                }),
                                            ),
                                        ),
                                )
                            }),
                    )
                    .child(
                        panel()
                            .p(px(16.))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .child("触发组合键"),
                            )
                            .child(div().flex().gap_2().mt(px(10.)).children(
                                TriggerChord::ALL.into_iter().map(|item| {
                                    let labels = ["⌃ ⌥ ⌘", "⌃ ⌥ ⇧", "⌃ ⌘ ⇧"];
                                    let index = TriggerChord::ALL
                                        .iter()
                                        .position(|candidate| *candidate == item)
                                        .unwrap();
                                    choice_button(
                                        labels[index],
                                        item == chord,
                                        match item {
                                            TriggerChord::ControlOptionCommand => {
                                                "chord-control-option-command"
                                            }
                                            TriggerChord::ControlOptionShift => {
                                                "chord-control-option-shift"
                                            }
                                            TriggerChord::ControlCommandShift => {
                                                "chord-control-command-shift"
                                            }
                                        },
                                    )
                                    .on_click(cx.listener(
                                        move |_, _, _, cx| {
                                            PingController::update_global(cx, |ping, cx| {
                                                ping.set_chord(item, cx)
                                            });
                                            cx.notify();
                                        },
                                    ))
                                }),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .gap_4()
                                    .mt(px(14.))
                                    .child(setting_stepper(
                                        "音量",
                                        format!("{}%", (volume * 100.0).round() as i32),
                                        "volume-down",
                                        "volume-up",
                                        cx,
                                        volume,
                                        0.1,
                                        false,
                                    ))
                                    .child(setting_stepper(
                                        "效果大小",
                                        format!("{}%", (scale * 100.0).round() as i32),
                                        "scale-down",
                                        "scale-up",
                                        cx,
                                        scale,
                                        0.05,
                                        true,
                                    )),
                            ),
                    )
                    .child(
                        panel()
                            .p(px(16.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::BOLD)
                                            .child("信号试用"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(TEXT_MUTED))
                                            .child("仅在窗口内预览 · 无需授权"),
                                    ),
                            )
                            .child(
                                div()
                                    .h(px((120.0 * scale).max(120.0) as f32))
                                    .mt(px(12.))
                                    .rounded_lg()
                                    .bg(rgb(BACKGROUND))
                                    .border_1()
                                    .border_color(rgb(BORDER))
                                    .when(preview_view.is_none(), |this| {
                                        this.flex()
                                            .items_center()
                                            .justify_center()
                                            .text_sm()
                                            .text_color(rgb(TEXT_MUTED))
                                            .child("点选下方信号，感受一下")
                                    })
                                    .when_some(preview_view, |this, view| this.child(view)),
                            )
                            .child(div().flex().flex_col().gap_2().mt(px(12.)).children(
                                PingKind::ALL.chunks(3).map(|row| {
                                    div().flex().gap_2().children(
                                        row.iter()
                                            .map(|kind| preview_button(*kind, preview_kind, cx)),
                                    )
                                }),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("仅监听操作所需按键，不读取或保存输入内容。"),
                            )
                            .child(
                                secondary_button("ping-input-monitoring", "输入监控设置").on_click(
                                    cx.listener(|_, _, _, _| {
                                        ping_settings::open_input_monitoring_settings();
                                    }),
                                ),
                            ),
                    ),
            )
    }
}
