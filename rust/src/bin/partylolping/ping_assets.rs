use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use gpui_kit::{point, px, Bounds, Path as GpuiPath, PathBuilder, Pixels, Window};
use party_engine::PingKind;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Vector {
    contours: Vec<Vec<Vec<f32>>>,
}

pub struct PingAssets {
    root: PathBuf,
    vectors: HashMap<PingKind, Vector>,
}

impl PingAssets {
    pub fn load() -> Result<Self, String> {
        let root = resource_root();
        let path = root.join("Artwork/ping-vectors.json");
        let bytes =
            fs::read(&path).map_err(|error| format!("无法读取 {}：{error}", path.display()))?;
        let mut source: HashMap<String, Vector> = serde_json::from_slice(&bytes)
            .map_err(|error| format!("无法解析 {}：{error}", path.display()))?;
        let vectors = PingKind::ALL
            .into_iter()
            .filter_map(|kind| source.remove(kind.key()).map(|vector| (kind, vector)))
            .collect();
        Ok(Self { root, vectors })
    }

    pub fn sound_path(&self, kind: PingKind) -> PathBuf {
        self.root
            .join("Sounds")
            .join(format!("{}.mp3", kind.sound_name()))
    }

    pub fn missing_files(&self) -> Vec<String> {
        let mut missing = Vec::new();
        for kind in PingKind::ALL {
            if !self.vectors.contains_key(&kind) {
                missing.push(format!("{} 的矢量轮廓", kind.title()));
            }
            if !self.sound_path(kind).is_file() {
                missing.push(format!("{}.mp3", kind.sound_name()));
            }
        }
        missing
    }

    pub fn paint_icon(
        &self,
        window: &mut Window,
        kind: PingKind,
        bounds: Bounds<Pixels>,
        color: impl Into<gpui_kit::Background>,
    ) {
        if let Some(path) = self.icon_path(kind, bounds) {
            window.paint_path(path, color);
        }
    }

    pub fn icon_path(&self, kind: PingKind, bounds: Bounds<Pixels>) -> Option<GpuiPath<Pixels>> {
        let vector = self.vectors.get(&kind)?;
        let x0 = bounds.origin.x.as_f32();
        let y0 = bounds.origin.y.as_f32();
        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32();
        let mut builder = PathBuilder::fill();
        for contour in &vector.contours {
            if contour.len() < 3 || contour.iter().any(|point| point.len() < 2) {
                continue;
            }
            let point_at = |index: usize| {
                let source = &contour[index];
                point(
                    px(x0 + source[0] * width),
                    px(map_icon_y(y0, source[1], height)),
                )
            };
            let entry = |index: usize| {
                let current = &contour[index];
                let previous = &contour[(index + contour.len() - 1) % contour.len()];
                let x = x0 + (current[0] * 0.88 + previous[0] * 0.12) * width;
                let y = map_icon_y(y0, current[1] * 0.88 + previous[1] * 0.12, height);
                point(px(x), px(y))
            };
            builder.move_to(entry(0));
            for index in 0..contour.len() {
                let current = &contour[index];
                let next = &contour[(index + 1) % contour.len()];
                let exit_x = x0 + (current[0] * 0.88 + next[0] * 0.12) * width;
                let exit_y = map_icon_y(y0, current[1] * 0.88 + next[1] * 0.12, height);
                builder.curve_to(point(px(exit_x), px(exit_y)), point_at(index));
                builder.line_to(entry((index + 1) % contour.len()));
            }
            builder.close();
        }
        builder.build().ok()
    }
}

fn map_icon_y(origin: f32, source_y: f32, height: f32) -> f32 {
    origin + (1.0 - source_y) * height
}

pub fn resource_root() -> PathBuf {
    if let Ok(executable) = std::env::current_exe() {
        if let Some(contents) = executable.parent().and_then(Path::parent) {
            let bundled = contents.join("Resources");
            if bundled.join("Artwork/ping-vectors.json").is_file() {
                return bundled;
            }
        }
    }
    if let Ok(directory) = std::env::current_dir() {
        let local = directory.join("Resources");
        if local.join("Artwork/ping-vectors.json").is_file() {
            return local;
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../Resources")
}

#[cfg(test)]
mod tests {
    use super::map_icon_y;

    #[test]
    fn icon_y_flips_appkit_coordinates_for_gpui() {
        assert_eq!(map_icon_y(10.0, 0.0, 20.0), 30.0);
        assert_eq!(map_icon_y(10.0, 1.0, 20.0), 10.0);
        assert_eq!(map_icon_y(10.0, 0.25, 20.0), 25.0);
    }
}
