use std::sync::Arc;

use gpui_kit::{App, AppContext, Entity, Global};
use party_engine::{GestureAction, PingKind, TriggerChord};

use crate::ping_assets::PingAssets;
use crate::ping_input::{PingInput, PingInputEvent};
use crate::ping_overlay::{PingEffectView, PingOverlays};
use crate::ping_settings::PingSettings;
use crate::ping_sound::PingSoundPlayer;
use crate::ping_status_bar::{PingStatusBar, StatusBarAction};

#[derive(Clone, Copy, Debug)]
pub enum PingCommand {
    OpenWindow,
    Quit,
    Commit(PingKind),
}

pub struct PingController {
    assets: Arc<PingAssets>,
    settings: PingSettings,
    input: PingInput,
    overlays: PingOverlays,
    sound: PingSoundPlayer,
    status_bar: Option<PingStatusBar>,
    awaiting_permission: bool,
    message: String,
    preview_kind: PingKind,
    preview_view: Option<Entity<PingEffectView>>,
}

impl Global for PingController {}

impl PingController {
    pub fn new(assets: Arc<PingAssets>) -> Self {
        let settings = PingSettings::load();
        let input = PingInput::new(settings.chord, settings.scale);
        let overlays = PingOverlays::new(Arc::clone(&assets));
        let sound = PingSoundPlayer::new(Arc::clone(&assets));
        let status_bar = PingStatusBar::new().ok();
        Self {
            assets,
            settings,
            input,
            overlays,
            sound,
            status_bar,
            awaiting_permission: false,
            message: "全局信号已关闭。你仍可在下方预览效果。".to_owned(),
            preview_kind: PingKind::Missing,
            preview_view: None,
        }
    }

    pub fn restore_state(&mut self, cx: &mut App) {
        if self.settings.enabled {
            if PingInput::is_trusted() {
                self.set_enabled(true, cx);
            } else {
                self.message =
                    "上次已启用，但当前缺少辅助功能权限。开启开关可重新授权。".to_owned();
                self.update_status_bar();
            }
        }
        let missing = self.assets.missing_files();
        if !missing.is_empty() {
            self.message = format!("素材缺失，请重新构建应用：{}", missing.join("、"));
            self.update_status_bar();
        }
    }

    pub fn assets(&self) -> Arc<PingAssets> {
        Arc::clone(&self.assets)
    }

    pub fn enabled(&self) -> bool {
        self.settings.enabled
    }

    pub fn awaiting_permission(&self) -> bool {
        self.awaiting_permission
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn chord(&self) -> TriggerChord {
        self.settings.chord
    }

    pub fn volume(&self) -> f64 {
        self.settings.volume
    }

    pub fn scale(&self) -> f64 {
        self.settings.scale
    }

    pub fn preview_kind(&self) -> PingKind {
        self.preview_kind
    }

    pub fn preview_view(&self) -> Option<Entity<PingEffectView>> {
        self.preview_view.clone()
    }

    pub fn set_enabled(&mut self, value: bool, cx: &mut App) {
        if !value {
            self.awaiting_permission = false;
            self.input.stop();
            self.overlays.clear(cx);
            self.sound.stop_all();
            self.stop_preview(cx);
            self.settings.enabled = false;
            self.settings.save();
            self.message = "全局信号已关闭。你仍可在下方预览效果。".to_owned();
            self.update_status_bar();
            return;
        }

        if !PingInput::is_trusted() {
            self.awaiting_permission = true;
            self.message =
                "还差一步：在系统设置 → 隐私与安全性 → 辅助功能中开启 PartyLOLPing。".to_owned();
            PingInput::request_permission();
            self.update_status_bar();
            return;
        }

        match self.input.start(self.settings.chord, self.settings.scale) {
            Ok(()) => {
                self.settings.enabled = true;
                self.awaiting_permission = false;
                self.settings.save();
                self.message = format!(
                    "已就绪。按住 {}，移动选择，松开发送。",
                    self.settings.chord.symbols()
                );
            }
            Err(error) => {
                self.settings.enabled = false;
                self.awaiting_permission = false;
                self.settings.save();
                self.message = error.to_string();
            }
        }
        self.update_status_bar();
    }

    pub fn check_permission(&mut self, cx: &mut App) {
        if self.awaiting_permission && PingInput::is_trusted() {
            self.set_enabled(true, cx);
        } else if self.settings.enabled && !PingInput::is_trusted() {
            self.set_enabled(false, cx);
            self.message = "辅助功能权限已关闭，请重新授权后开启 Ping。".to_owned();
            self.update_status_bar();
        }
    }

    pub fn set_chord(&mut self, chord: TriggerChord, cx: &mut App) {
        self.settings.chord = chord;
        self.restart_input(cx);
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.settings.volume = volume.clamp(0.0, 1.0);
        self.settings.save();
    }

    pub fn set_scale(&mut self, scale: f64, cx: &mut App) {
        self.settings.scale = scale.clamp(0.75, 1.5);
        self.restart_input(cx);
    }

    pub fn preview(&mut self, cx: &mut App, kind: PingKind) {
        self.stop_preview(cx);
        self.preview_kind = kind;
        self.sound.play(kind, self.settings.volume);
        if let Some(error) = self.sound.error() {
            self.message = error.to_owned();
        }
        let view = cx.new(|_| {
            PingEffectView::new(Arc::clone(&self.assets), kind, self.settings.scale, None)
        });
        self.preview_view = Some(view);
    }

    pub fn stop_preview(&mut self, cx: &mut App) {
        if let Some(view) = self.preview_view.take() {
            view.update(cx, |view, cx| {
                view.stop();
                cx.notify();
            });
        }
    }

    pub fn show_remote_ping(
        &mut self,
        cx: &mut App,
        kind: PingKind,
        sender: String,
        play_audio: bool,
    ) {
        if let Err(error) = self
            .overlays
            .show_remote_ping(cx, kind, sender, self.settings.scale)
        {
            self.message = error;
        }
        if play_audio {
            self.sound.play(kind, self.settings.volume);
        }
    }

    pub fn cancel_effects(&mut self, cx: &mut App) {
        self.input.cancel_current();
        self.overlays.clear(cx);
        self.sound.stop_all();
        self.stop_preview(cx);
    }

    pub fn poll(&mut self, cx: &mut App) -> Vec<PingCommand> {
        self.check_permission(cx);
        let mut commands = Vec::new();
        if let Some(status_bar) = self.status_bar.as_mut() {
            for action in status_bar.drain_actions() {
                match action {
                    StatusBarAction::Toggle => {
                        let enable = !(self.settings.enabled || self.awaiting_permission);
                        self.set_enabled(enable, cx);
                    }
                    StatusBarAction::Open => commands.push(PingCommand::OpenWindow),
                    StatusBarAction::Quit => commands.push(PingCommand::Quit),
                }
            }
        }

        for event in self.input.drain_events() {
            match event {
                PingInputEvent::Unavailable => {
                    self.set_enabled(false, cx);
                    self.message = "全局输入已暂停，请检查辅助功能权限后重新开启。".to_owned();
                    self.update_status_bar();
                }
                PingInputEvent::Action(action) => {
                    if !self.settings.enabled {
                        continue;
                    }
                    match action {
                        GestureAction::Arm(_) => {}
                        GestureAction::Show(anchor) => {
                            match self.overlays.show_wheel(cx, anchor, self.settings.scale) {
                                Ok(center) => self.input.set_wheel_center(center),
                                Err(error) => self.message = error,
                            }
                        }
                        GestureAction::Hover(kind, angle) => {
                            self.overlays.select(cx, kind, angle);
                        }
                        GestureAction::Hide => {
                            self.overlays.hide_wheel(cx, false);
                        }
                        GestureAction::Dismiss => {
                            self.overlays.hide_wheel(cx, true);
                        }
                        GestureAction::Commit(kind, anchor) => {
                            if let Err(error) =
                                self.overlays
                                    .show_ping(cx, kind, anchor, self.settings.scale, None)
                            {
                                self.message = error;
                            }
                            self.sound.play(kind, self.settings.volume);
                            commands.push(PingCommand::Commit(kind));
                        }
                    }
                }
            }
        }
        commands
    }

    pub fn shutdown(&mut self, cx: &mut App) {
        self.input.stop();
        self.overlays.clear(cx);
        self.sound.stop_all();
        self.stop_preview(cx);
    }

    fn restart_input(&mut self, cx: &mut App) {
        self.cancel_effects(cx);
        self.settings.save();
        if self.settings.enabled {
            self.set_enabled(true, cx);
        }
        self.update_status_bar();
    }

    fn update_status_bar(&mut self) {
        if let Some(status_bar) = self.status_bar.as_mut() {
            status_bar.update(self.settings.enabled, self.awaiting_permission);
        }
    }
}
