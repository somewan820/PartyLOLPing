use std::sync::Arc;

use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{class, msg_send, sel, sel_impl};
use party_engine::PingKind;

use crate::ping_assets::PingAssets;

pub struct PingSoundPlayer {
    assets: Arc<PingAssets>,
    players: Vec<id>,
    error: Option<String>,
}

impl PingSoundPlayer {
    pub fn new(assets: Arc<PingAssets>) -> Self {
        Self {
            assets,
            players: Vec::new(),
            error: None,
        }
    }

    pub fn play(&mut self, kind: PingKind, volume: f64) {
        if volume <= 0.0 {
            return;
        }
        self.prune_finished();
        let path = self.assets.sound_path(kind);
        let Some(path) = path.to_str() else {
            self.error = Some(format!("音效路径无效：{}", kind.title()));
            return;
        };
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);
            let path = NSString::alloc(nil).init_str(path);
            let url: id = msg_send![class!(NSURL), fileURLWithPath: path];
            let error: id = nil;
            let player: id = msg_send![class!(AVAudioPlayer), alloc];
            let player: id = msg_send![player, initWithContentsOfURL: url error: &error];
            if player.is_null() {
                self.error = Some(format!("无法读取音效：{}", kind.title()));
                return;
            }
            let _: () = msg_send![player, setVolume: volume.clamp(0.0, 1.0) as f32];
            let _: () = msg_send![player, prepareToPlay];
            let started: bool = msg_send![player, play];
            if !started {
                self.error = Some(format!("音效播放失败：{}", kind.title()));
                let _: () = msg_send![player, release];
                return;
            }
            if self.players.len() >= 12 {
                let oldest = self.players.remove(0);
                let _: () = msg_send![oldest, stop];
                let _: () = msg_send![oldest, release];
            }
            self.players.push(player);
        }
    }

    pub fn stop_all(&mut self) {
        for player in self.players.drain(..) {
            unsafe {
                let _: () = msg_send![player, stop];
                let _: () = msg_send![player, release];
            }
        }
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn prune_finished(&mut self) {
        let mut playing = Vec::with_capacity(self.players.len());
        for player in self.players.drain(..) {
            unsafe {
                let is_playing: bool = msg_send![player, isPlaying];
                if is_playing {
                    playing.push(player);
                } else {
                    let _: () = msg_send![player, release];
                }
            }
        }
        self.players = playing;
    }
}

impl Drop for PingSoundPlayer {
    fn drop(&mut self) {
        self.stop_all();
    }
}

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}
