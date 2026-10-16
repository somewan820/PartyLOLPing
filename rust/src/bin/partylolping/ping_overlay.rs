use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Instant;

use cocoa::base::id;
use core_graphics::display::{CGDisplayBounds, CGMainDisplayID};
use gpui_kit::{
    canvas, div, point, prelude::*, px, rgb, size, AnyWindowHandle, App, Bounds, Context,
    DisplayId, Div, Entity, FontWeight, Hsla, Path, PathBuilder, Pixels, Render, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
};
use objc::{msg_send, runtime::YES, sel, sel_impl};
use party_engine::{PingKind, PingPoint, PingRect, WheelGeometry};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::ping_assets::PingAssets;

const WHEEL_GOLD: u32 = 0xc7a866;
const WHEEL_CYAN: u32 = 0x05cfc4;

pub struct PingOverlays {
    assets: Arc<PingAssets>,
    wheel: Option<WheelOverlay>,
    effects: Vec<AnyWindowHandle>,
}

struct WheelOverlay {
    window: AnyWindowHandle,
    view: Entity<WheelView>,
}

impl PingOverlays {
    pub fn new(assets: Arc<PingAssets>) -> Self {
        Self {
            assets,
            wheel: None,
            effects: Vec::new(),
        }
    }

    pub fn show_wheel(
        &mut self,
        cx: &mut App,
        anchor: PingPoint,
        scale: f64,
    ) -> Result<PingPoint, String> {
        self.hide_wheel(cx, false);
        let display = display_for_anchor(cx, anchor);
        let side = 300.0 * scale as f32;
        let visible_global = display.visible_global();
        let center = WheelGeometry::clamped_center(anchor, visible_global, side as f64 / 2.0);
        let relative_center = display.relative(center);
        let bounds = Bounds::new(
            point(
                px(relative_center.x as f32 - side / 2.0),
                px(relative_center.y as f32 - side / 2.0),
            ),
            size(px(side), px(side)),
        );
        let assets = Arc::clone(&self.assets);
        let mut view = None;
        let window = cx
            .open_window(overlay_options(display.id, bounds), |native, cx| {
                configure_overlay_window(native);
                let created = cx.new(|_| WheelView::new(Arc::clone(&assets), scale));
                view = Some(created.clone());
                created
            })
            .map_err(|error| format!("无法创建轮盘窗口：{error}"))?;
        let view = view.expect("wheel view was created");
        view.update(cx, |_, cx| {
            cx.notify();
        });
        let window = window.into();
        self.wheel = Some(WheelOverlay { window, view });
        Ok(center)
    }

    pub fn select(&mut self, cx: &mut App, kind: PingKind, angle: Option<f64>) {
        let Some(wheel) = self.wheel.as_ref() else {
            return;
        };
        wheel.view.update(cx, |view, cx| {
            view.set_selection(kind, angle);
            cx.notify();
        });
    }

    pub fn hide_wheel(&mut self, cx: &mut App, animated: bool) {
        if let Some(wheel) = self.wheel.take() {
            if animated {
                wheel.view.update(cx, |view, cx| {
                    view.stop();
                    cx.notify();
                });
                cx.spawn(async move |cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(100))
                        .await;
                    let _ = wheel
                        .window
                        .update(cx, |_, window, _| window.remove_window());
                })
                .detach();
            } else {
                let _ = wheel
                    .window
                    .update(cx, |_, window, _| window.remove_window());
            }
        }
    }

    pub fn show_ping(
        &mut self,
        cx: &mut App,
        kind: PingKind,
        anchor: PingPoint,
        scale: f64,
        sender: Option<String>,
    ) -> Result<(), String> {
        if self.effects.len() >= 16 {
            if let Some(oldest) = self.effects.first().copied() {
                let _ = oldest.update(cx, |_, window, _| window.remove_window());
            }
            self.effects.remove(0);
        }

        let display = display_for_anchor(cx, anchor);
        let side = 220.0 * scale as f32;
        let relative_anchor = display.relative(anchor);
        let bounds = Bounds::new(
            point(
                px(relative_anchor.x as f32 - side / 2.0),
                px(relative_anchor.y as f32 - side / 2.0 - 13.0 * scale as f32),
            ),
            size(px(side), px(side)),
        );
        let assets = Arc::clone(&self.assets);
        let mut view = None;
        let window = cx
            .open_window(overlay_options(display.id, bounds), |native, cx| {
                configure_overlay_window(native);
                let created =
                    cx.new(|_| WheelView::new_effect(Arc::clone(&assets), kind, scale, sender));
                view = Some(created.clone());
                created
            })
            .map_err(|error| format!("无法创建 Ping 覆盖层：{error}"))?;
        view.expect("ping effect view was created")
            .update(cx, |_, cx| {
                cx.notify();
            });
        let window = window.into();
        self.effects.push(window);
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(2050))
                .await;
            let _ = window.update(cx, |_, window, _| window.remove_window());
        })
        .detach();
        Ok(())
    }

    pub fn show_remote_ping(
        &mut self,
        cx: &mut App,
        kind: PingKind,
        sender: String,
        scale: f64,
    ) -> Result<(), String> {
        let display = primary_display(cx);
        let visible = display.visible_global();
        let point = PingPoint::new(
            visible.max_x() - 110.0 * scale,
            visible.min_y() + 120.0 * scale,
        );
        self.show_ping(cx, kind, point, scale, Some(sender))
    }

    pub fn clear(&mut self, cx: &mut App) {
        self.hide_wheel(cx, false);
        for window in self.effects.drain(..) {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        }
    }
}

pub struct WheelView {
    assets: Arc<PingAssets>,
    scale: f64,
    selected: PingKind,
    direction: Option<f64>,
    weights: [f64; 8],
    origins: [f64; 8],
    transition_start: Instant,
    opening_start: Instant,
    closing_start: Option<Instant>,
    effect: Option<(PingKind, Instant, Option<String>)>,
}

impl WheelView {
    fn new(assets: Arc<PingAssets>, scale: f64) -> Self {
        let now = Instant::now();
        Self {
            assets,
            scale,
            selected: PingKind::Generic,
            direction: None,
            weights: [0.0; 8],
            origins: [0.0; 8],
            transition_start: now,
            opening_start: now,
            closing_start: None,
            effect: None,
        }
    }

    fn new_effect(
        assets: Arc<PingAssets>,
        kind: PingKind,
        scale: f64,
        sender: Option<String>,
    ) -> Self {
        let mut view = Self::new(assets, scale);
        view.effect = Some((kind, Instant::now(), sender));
        view
    }

    fn set_selection(&mut self, kind: PingKind, angle: Option<f64>) {
        self.update_weights();
        self.direction = if kind == PingKind::Generic {
            None
        } else {
            angle
        };
        if self.selected != kind {
            self.origins = self.weights;
            self.selected = kind;
            self.transition_start = Instant::now();
        }
    }

    fn stop(&mut self) {
        self.closing_start = Some(Instant::now());
    }

    fn update_weights(&mut self) {
        let progress = ease(self.transition_start.elapsed().as_secs_f64() / 0.12);
        for (index, kind) in PingKind::WHEEL.into_iter().enumerate() {
            let target = if kind == self.selected { 1.0 } else { 0.0 };
            self.weights[index] = self.origins[index] + (target - self.origins[index]) * progress;
        }
    }
}

impl Render for WheelView {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if let Some((kind, started, sender)) = self.effect.clone() {
            let elapsed = started.elapsed().as_secs_f64();
            if elapsed < 2.0 {
                window.request_animation_frame();
            }
            let sender_color = mix_hex(0x000000, 0xededed, effect_alpha(elapsed));
            return effect_elements(Arc::clone(&self.assets), kind, self.scale, elapsed)
                .size_full()
                .opacity(1.0)
                .when_some(sender, |this, sender| {
                    this.child(
                        div()
                            .absolute()
                            .left(px(0.))
                            .top(px(161.0 * self.scale as f32))
                            .w(px(220.0 * self.scale as f32))
                            .flex()
                            .justify_center()
                            .text_size(px(11.0 * self.scale as f32))
                            .text_color(rgb(sender_color))
                            .child(sender),
                    )
                });
        }
        self.update_weights();
        let opening = if let Some(start) = self.closing_start {
            1.0 - ease(start.elapsed().as_secs_f64() / 0.09) as f32
        } else {
            ease(self.opening_start.elapsed().as_secs_f64() / 0.09) as f32
        };
        if self.closing_start.is_some()
            || opening < 1.0
            || self
                .weights
                .iter()
                .zip(self.origins)
                .any(|(weight, origin)| (weight - origin).abs() > 0.001)
        {
            window.request_animation_frame();
        }
        let assets = Arc::clone(&self.assets);
        let weights = self.weights;
        let selected = self.selected;
        let direction = self.direction;
        let scale = self.scale;
        let title = if selected == PingKind::Generic {
            "信号"
        } else {
            selected.title()
        };
        let side = 300.0 * scale as f32;
        div()
            .size_full()
            .opacity(opening)
            .child(
                canvas(
                    move |_, _, _| {},
                    move |bounds, _, window, _| {
                        paint_wheel(window, bounds, assets, weights, selected, direction, scale);
                    },
                )
                .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .left(px(side / 2.0 - 70.0 * scale as f32))
                    .top(px(side / 2.0 - 8.0 * scale as f32))
                    .w(px(140.0 * scale as f32))
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(13.0 * scale as f32))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(selected.tint()))
                            .child(title),
                    )
                    .child(
                        div()
                            .mt(px(25.0 * scale as f32))
                            .text_size(px(8.5 * scale as f32))
                            .text_color(rgb(0x7c858c))
                            .child("右键取消"),
                    ),
            )
    }
}

pub struct PingEffectView {
    assets: Arc<PingAssets>,
    kind: PingKind,
    scale: f64,
    sender: Option<String>,
    started: Instant,
    stopped: bool,
}

impl PingEffectView {
    pub fn new(
        assets: Arc<PingAssets>,
        kind: PingKind,
        scale: f64,
        sender: Option<String>,
    ) -> Self {
        Self {
            assets,
            kind,
            scale,
            sender,
            started: Instant::now(),
            stopped: false,
        }
    }

    pub fn stop(&mut self) {
        self.stopped = true;
    }
}

impl Render for PingEffectView {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let elapsed = if self.stopped {
            2.0
        } else {
            self.started.elapsed().as_secs_f64()
        };
        if elapsed < 2.0 {
            window.request_animation_frame();
        }
        let assets = Arc::clone(&self.assets);
        let kind = self.kind;
        let scale = self.scale;
        let side = 220.0 * scale as f32;
        let sender = self.sender.clone();
        div()
            .size_full()
            .child(
                canvas(
                    move |_, _, _| {},
                    move |bounds, _, window, _| {
                        paint_effect(window, bounds, assets, kind, scale, elapsed);
                    },
                )
                .size_full(),
            )
            .when_some(sender, |this, sender| {
                this.child(
                    div()
                        .absolute()
                        .left(px(0.))
                        .top(px(side / 2.0 + 47.0 * scale as f32))
                        .w(px(side))
                        .flex()
                        .justify_center()
                        .text_size(px(11.0 * scale as f32))
                        .text_color(rgb(mix_hex(0x000000, 0xededed, effect_alpha(elapsed))))
                        .child(sender),
                )
            })
    }
}

fn effect_elements(assets: Arc<PingAssets>, kind: PingKind, scale: f64, elapsed: f64) -> Div {
    let s = scale;
    let ground_x = 110.0 * s;
    let ground_y = 123.0 * s;
    let alpha = effect_alpha(elapsed);
    let ground_fade = 1.0 - ease(elapsed / 1.2);
    let tint = kind.tint();
    let mut root = div().size_full().child(div().size_full());

    for (radius, intensity) in [
        (48.0, 0.10),
        (39.0, 0.22 * ground_fade),
        (30.0, 0.32 * ground_fade),
        (12.0, 0.32 * ground_fade),
    ] {
        let color = mix_hex(0x000000, tint, intensity);
        root = root.child(effect_ellipse(
            ground_x,
            ground_y,
            radius * s,
            radius * 0.52 * s,
            rgb(color).into(),
            0.0,
        ));
    }

    for index in 0..2 {
        let age = elapsed - index as f64 * 0.24;
        if !(0.0..1.15).contains(&age) {
            continue;
        }
        let progress = age / 1.15;
        let radius = (10.0 + 49.0 * (1.0 - (1.0 - progress).powi(2))) * s;
        let ring_alpha = ease(age / 0.045) * (1.0 - progress).powf(1.5);
        let shadow_color = mix_hex(0x000000, tint, ring_alpha * 0.65);
        root = root.child(effect_ellipse(
            ground_x,
            ground_y,
            radius + 2.0 * s,
            (radius + 2.0 * s) * 0.52,
            rgb(shadow_color).into(),
            1.0,
        ));
        let color = mix_hex(0x000000, tint, ring_alpha * 0.88);
        root = root.child(effect_ellipse(
            ground_x,
            ground_y,
            radius,
            radius * 0.52,
            rgb(color).into(),
            1.0,
        ));
    }

    if elapsed < 0.65 {
        let progress = elapsed / 0.65;
        for index in 0..5 {
            let angle = index as f64 * 2.0 * PI as f64 / 5.0 + PI as f64 / 7.0;
            let distance = (10.0 + 31.0 * progress) * s;
            let spark_x = ground_x + angle.cos() * distance;
            let spark_y = ground_y + angle.sin() * distance * 0.6;
            let color = mix_hex(0x000000, tint, (1.0 - progress) * 0.7);
            root = root.child(effect_ellipse(
                spark_x,
                spark_y,
                s,
                s,
                rgb(color).into(),
                0.0,
            ));
        }
    }

    let pop = if elapsed < 0.12 {
        0.50 + 0.58 * ease(elapsed / 0.12)
    } else if elapsed < 0.25 {
        1.08 - 0.08 * ease((elapsed - 0.12) / 0.13)
    } else {
        1.0
    };
    let lift = (9.0 * ease(elapsed / 0.30) + 2.0 * (elapsed.min(1.4) * PI as f64).sin()) * s;
    let side = 43.0 * pop * s;
    let icon_x = ground_x - side / 2.0;
    let icon_y = ground_y - 8.0 * s - lift - side;
    let glow_color = mix_hex(0x000000, tint, alpha * 0.40);
    let foreground_color = mix_hex(0x000000, tint, alpha);
    root = root.child(effect_icon(
        Arc::clone(&assets),
        kind,
        icon_x,
        icon_y,
        side,
        rgb(glow_color).into(),
    ));
    root.child(effect_icon(
        assets,
        kind,
        icon_x,
        icon_y,
        side,
        rgb(foreground_color).into(),
    ))
}

fn effect_alpha(elapsed: f64) -> f64 {
    ease(elapsed / 0.08) * (1.0 - ease((elapsed - 1.25) / 0.75))
}

fn effect_ellipse(
    center_x: f64,
    center_y: f64,
    radius_x: f64,
    radius_y: f64,
    color: Hsla,
    border_width: f32,
) -> Div {
    let mut ellipse = div()
        .absolute()
        .left(px((center_x - radius_x) as f32))
        .top(px((center_y - radius_y) as f32))
        .w(px((radius_x * 2.0) as f32))
        .h(px((radius_y * 2.0) as f32))
        .rounded_full()
        .bg(color);
    if border_width > 0.0 {
        ellipse = ellipse.border_1().border_color(color);
    }
    ellipse
}

fn effect_icon(
    assets: Arc<PingAssets>,
    kind: PingKind,
    x: f64,
    y: f64,
    side: f64,
    color: Hsla,
) -> impl IntoElement {
    canvas(
        move |_, _, _| {},
        move |bounds, _, window, _| {
            assets.paint_icon(window, kind, bounds, color);
        },
    )
    .absolute()
    .left(px(x as f32))
    .top(px(y as f32))
    .w(px(side as f32))
    .h(px(side as f32))
}

#[derive(Clone, Copy)]
struct DisplayInfo {
    id: DisplayId,
    global: PingRect,
    visible_relative: PingRect,
}

impl DisplayInfo {
    fn visible_global(self) -> PingRect {
        PingRect::new(
            self.global.x + self.visible_relative.x,
            self.global.y + self.visible_relative.y,
            self.visible_relative.width,
            self.visible_relative.height,
        )
    }

    fn relative(self, point: PingPoint) -> PingPoint {
        PingPoint::new(point.x - self.global.x, point.y - self.global.y)
    }
}

fn display_for_anchor(cx: &App, anchor: PingPoint) -> DisplayInfo {
    let displays = cx.displays();
    for display in &displays {
        let raw_id: u64 = display.id().into();
        let global = display_global_bounds(raw_id);
        if global.contains(anchor) {
            return display_info(display.as_ref(), raw_id, global);
        }
    }
    primary_display(cx)
}

fn primary_display(cx: &App) -> DisplayInfo {
    let primary = unsafe { CGMainDisplayID() } as u64;
    let displays = cx.displays();
    for display in &displays {
        let raw_id: u64 = display.id().into();
        if raw_id == primary {
            return display_info(display.as_ref(), raw_id, display_global_bounds(raw_id));
        }
    }
    if let Some(display) = displays.first() {
        let raw_id: u64 = display.id().into();
        return display_info(display.as_ref(), raw_id, display_global_bounds(raw_id));
    }
    DisplayInfo {
        id: DisplayId::new(primary),
        global: PingRect::new(0.0, 0.0, 1440.0, 900.0),
        visible_relative: PingRect::new(0.0, 0.0, 1440.0, 900.0),
    }
}

fn display_info(
    display: &dyn gpui_kit::PlatformDisplay,
    raw_id: u64,
    global: PingRect,
) -> DisplayInfo {
    let visible = display.visible_bounds();
    DisplayInfo {
        id: DisplayId::new(raw_id),
        global,
        visible_relative: PingRect::new(
            visible.origin.x.as_f32() as f64,
            visible.origin.y.as_f32() as f64,
            visible.size.width.as_f32() as f64,
            visible.size.height.as_f32() as f64,
        ),
    }
}

fn display_global_bounds(raw_id: u64) -> PingRect {
    let bounds = unsafe { CGDisplayBounds(raw_id as u32) };
    PingRect::new(
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.width,
        bounds.size.height,
    )
}

fn overlay_options(display_id: DisplayId, bounds: Bounds<Pixels>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
        focus: false,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        display_id: Some(display_id),
        window_background: WindowBackgroundAppearance::Transparent,
        inactive_frame_interval: None,
        ..Default::default()
    }
}

fn configure_overlay_window(window: &Window) {
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    unsafe {
        let view = handle.ns_view.as_ptr() as id;
        let native_window: id = msg_send![view, window];
        let _: () = msg_send![native_window, setIgnoresMouseEvents: YES];
        let _: () = msg_send![native_window, setHidesOnDeactivate: objc::runtime::NO];
        let _: () = msg_send![native_window, setHasShadow: objc::runtime::NO];
        let _: () = msg_send![native_window, setLevel: 25_i64];
        let _: () = msg_send![native_window, setCollectionBehavior: 353_u64];
        let _: () = msg_send![native_window, setExcludedFromWindowsMenu: YES];
        let _: () = msg_send![native_window, orderFrontRegardless];
    }
}

fn paint_wheel(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    assets: Arc<PingAssets>,
    weights: [f64; 8],
    selected: PingKind,
    direction: Option<f64>,
    _scale: f64,
) {
    let s = bounds.size.width.as_f32() as f64 / 300.0;
    let center = (
        (bounds.origin.x + bounds.size.width / 2.0).as_f32() as f64,
        (bounds.origin.y + bounds.size.height / 2.0).as_f32() as f64,
    );
    let outer = WheelGeometry::OUTER_RADIUS * s;
    let inner = WheelGeometry::CENTER_RADIUS * s;

    for (index, kind) in PingKind::WHEEL.into_iter().enumerate() {
        let theta = -PI as f64 / 2.0 + index as f64 * PI as f64 / 4.0;
        let half = PI as f64 / 8.0;
        if let Some(path) = annular_sector(center, inner, outer, theta - half, theta + half) {
            window.paint_path(path, rgba(0x090909, 0.42));
        }
        let middle = inner + (outer - inner) * 0.55;
        if let Some(path) = annular_sector(center, inner, middle, theta - half, theta + half) {
            window.paint_path(path, rgba(0x06090d, 0.18));
        }
        let weight = weights[index];
        if weight > 0.001 {
            if let Some(path) = annular_sector(center, inner, outer, theta - half, theta + half) {
                window.paint_path(path, rgba(WHEEL_CYAN, 0.06 * weight));
            }
            let middle = inner + (outer - inner) * 0.65;
            if let Some(path) = annular_sector(center, inner, middle, theta - half, theta + half) {
                window.paint_path(path, rgba(WHEEL_CYAN, 0.32 * weight));
            }
            let inner_highlight = inner + (outer - inner) * 0.35;
            if let Some(path) =
                annular_sector(center, inner, inner_highlight, theta - half, theta + half)
            {
                window.paint_path(path, rgba(WHEEL_CYAN, 0.85 * weight));
            }
        }
        let boundary = theta - half;
        paint_line(
            window,
            polar(center, inner, boundary),
            polar(center, outer, boundary),
            0.55 * s,
            rgba(WHEEL_GOLD, 0.30),
        );
        let icon_center = polar(center, WheelGeometry::ICON_RADIUS * s, theta);
        let side = (22.0 + 8.0 * weight) * s;
        let color = mix_hex(kind.wheel_idle_tint(), kind.tint(), weight);
        assets.paint_icon(
            window,
            kind,
            centered_rect(
                (icon_center.x.as_f32() as f64, icon_center.y.as_f32() as f64),
                side,
                side,
            ),
            rgba(color, 1.0),
        );
    }

    fill_ellipse(window, center, inner, inner, rgba(0x05080b, 0.96));
    if selected == PingKind::Generic {
        fill_ellipse(
            window,
            center,
            inner * 0.88,
            inner * 0.88,
            rgba(WHEEL_CYAN, 0.19),
        );
        fill_ellipse(
            window,
            center,
            inner * 0.45,
            inner * 0.45,
            rgba(WHEEL_CYAN, 0.04),
        );
    }
    stroke_ellipse(
        window,
        center,
        inner,
        inner,
        1.2 * s,
        rgba(WHEEL_GOLD, 0.70),
    );
    stroke_ellipse(
        window,
        center,
        inner - 3.0 * s,
        inner - 3.0 * s,
        1.4 * s,
        rgba(WHEEL_GOLD, 1.0),
    );

    if let Some(angle) = direction {
        let theta = -PI as f64 / 2.0 + angle as f32 as f64;
        let mut notch = PathBuilder::stroke(px(1.7 * s as f32));
        notch.move_to(polar(center, inner, theta + 0.06));
        notch.line_to(polar(center, inner + 6.0 * s, theta));
        notch.line_to(polar(center, inner, theta - 0.06));
        if let Ok(path) = notch.build() {
            window.paint_path(path, rgba(WHEEL_GOLD, 1.0));
        }
        if let Some(path) = arc_path(
            center,
            inner - 7.0 * s,
            inner - 7.0 * s,
            theta - 18.0_f64.to_radians(),
            theta + 18.0_f64.to_radians(),
            1.8 * s,
        ) {
            window.paint_path(path, rgba(WHEEL_CYAN, 1.0));
        }
    }

    if selected == PingKind::Generic {
        assets.paint_icon(
            window,
            PingKind::Generic,
            centered_rect((center.0, center.1 - 18.0 * s), 22.0 * s, 22.0 * s),
            rgba(PingKind::Generic.tint(), 1.0),
        );
    }
}

fn paint_effect(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    assets: Arc<PingAssets>,
    kind: PingKind,
    scale: f64,
    elapsed: f64,
) {
    if !(0.0..2.0).contains(&elapsed) {
        return;
    }
    let s = scale;
    let ground = (
        (bounds.origin.x + bounds.size.width / 2.0).as_f32() as f64,
        (bounds.origin.y + bounds.size.height / 2.0).as_f32() as f64 + 13.0 * s,
    );
    let alpha = ease(elapsed / 0.08) * (1.0 - ease((elapsed - 1.25) / 0.75));
    let ground_fade = 1.0 - ease(elapsed / 1.2);
    let tint = kind.tint();
    for (radius, intensity) in [
        (48.0, 0.10),
        (39.0, 0.22 * ground_fade),
        (30.0, 0.32 * ground_fade),
        (12.0, 0.32 * ground_fade),
    ] {
        fill_ellipse(
            window,
            ground,
            radius * s,
            radius * 0.52 * s,
            rgba(tint, intensity),
        );
    }

    for index in 0..2 {
        let age = elapsed - index as f64 * 0.24;
        if !(0.0..1.15).contains(&age) {
            continue;
        }
        let progress = age / 1.15;
        let radius = (10.0 + 49.0 * (1.0 - (1.0 - progress).powi(2))) * s;
        let ring_alpha = ease(age / 0.045) * (1.0 - progress).powf(1.5);
        stroke_ellipse(
            window,
            ground,
            radius + 2.0 * s,
            (radius + 2.0 * s) * 0.52,
            2.0 * s,
            rgba(tint, ring_alpha * 0.65),
        );
        stroke_ellipse(
            window,
            ground,
            radius,
            radius * 0.52,
            (if index == 0 { 1.9 } else { 1.1 }) * s,
            rgba(tint, ring_alpha * 0.88),
        );
    }

    if elapsed < 0.65 {
        let progress = elapsed / 0.65;
        for index in 0..5 {
            let angle = index as f64 * 2.0 * PI as f64 / 5.0 + PI as f64 / 7.0;
            let distance = (10.0 + 31.0 * progress) * s;
            let spark = (
                ground.0 + angle.cos() * distance,
                ground.1 + angle.sin() * distance * 0.6,
            );
            fill_ellipse(window, spark, s, s, rgba(tint, (1.0 - progress) * 0.7));
        }
    }

    let pop = if elapsed < 0.12 {
        0.50 + 0.58 * ease(elapsed / 0.12)
    } else if elapsed < 0.25 {
        1.08 - 0.08 * ease((elapsed - 0.12) / 0.13)
    } else {
        1.0
    };
    let lift = (9.0 * ease(elapsed / 0.30) + 2.0 * (elapsed.min(1.4) * PI as f64).sin()) * s;
    let side = 43.0 * pop * s;
    let icon_center = (ground.0, ground.1 - 8.0 * s - lift - side / 2.0);
    assets.paint_icon(
        window,
        kind,
        centered_rect(icon_center, side, side),
        rgba(tint, alpha * 0.40),
    );
    assets.paint_icon(
        window,
        kind,
        centered_rect(icon_center, side, side),
        rgba(tint, alpha),
    );
}

fn ease(value: f64) -> f64 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn rgba(hex: u32, alpha: f64) -> Hsla {
    rgb(hex).opacity(alpha.clamp(0.0, 1.0) as f32).into()
}

fn mix_hex(from: u32, to: u32, amount: f64) -> u32 {
    let t = amount.clamp(0.0, 1.0);
    let channel = |shift: u32| {
        let a = ((from >> shift) & 0xff) as f64;
        let b = ((to >> shift) & 0xff) as f64;
        (a + (b - a) * t).round() as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

fn polar(center: (f64, f64), radius: f64, angle: f64) -> gpui_kit::Point<Pixels> {
    point(
        px((center.0 + angle.cos() * radius) as f32),
        px((center.1 + angle.sin() * radius) as f32),
    )
}

fn centered_rect(center: (f64, f64), width: f64, height: f64) -> Bounds<Pixels> {
    Bounds::new(
        point(
            px((center.0 - width / 2.0) as f32),
            px((center.1 - height / 2.0) as f32),
        ),
        size(px(width as f32), px(height as f32)),
    )
}

fn annular_sector(
    center: (f64, f64),
    inner: f64,
    outer: f64,
    start: f64,
    end: f64,
) -> Option<Path<Pixels>> {
    let mut builder = PathBuilder::fill();
    builder.move_to(polar(center, outer, start));
    for step in 1..=16 {
        let angle = start + (end - start) * step as f64 / 16.0;
        builder.line_to(polar(center, outer, angle));
    }
    for step in (0..=16).rev() {
        let angle = start + (end - start) * step as f64 / 16.0;
        builder.line_to(polar(center, inner, angle));
    }
    builder.close();
    builder.build().ok()
}

fn arc_path(
    center: (f64, f64),
    radius_x: f64,
    radius_y: f64,
    start: f64,
    end: f64,
    width: f64,
) -> Option<Path<Pixels>> {
    let mut builder = PathBuilder::stroke(px(width as f32));
    for step in 0..=48 {
        let angle = start + (end - start) * step as f64 / 48.0;
        let point = point(
            px((center.0 + angle.cos() * radius_x) as f32),
            px((center.1 + angle.sin() * radius_y) as f32),
        );
        if step == 0 {
            builder.move_to(point);
        } else {
            builder.line_to(point);
        }
    }
    builder.build().ok()
}

fn fill_ellipse(
    window: &mut Window,
    center: (f64, f64),
    radius_x: f64,
    radius_y: f64,
    color: Hsla,
) {
    let mut builder = PathBuilder::fill();
    for step in 0..=48 {
        let angle = step as f64 * 2.0 * PI as f64 / 48.0;
        let point = point(
            px((center.0 + angle.cos() * radius_x) as f32),
            px((center.1 + angle.sin() * radius_y) as f32),
        );
        if step == 0 {
            builder.move_to(point);
        } else {
            builder.line_to(point);
        }
    }
    builder.close();
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

fn stroke_ellipse(
    window: &mut Window,
    center: (f64, f64),
    radius_x: f64,
    radius_y: f64,
    width: f64,
    color: Hsla,
) {
    if let Some(path) = arc_path(center, radius_x, radius_y, 0.0, 2.0 * PI as f64, width) {
        window.paint_path(path, color);
    }
}

fn paint_line(
    window: &mut Window,
    from: gpui_kit::Point<Pixels>,
    to: gpui_kit::Point<Pixels>,
    width: f64,
    color: Hsla,
) {
    let mut builder = PathBuilder::stroke(px(width as f32));
    builder.move_to(from);
    builder.line_to(to);
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}
