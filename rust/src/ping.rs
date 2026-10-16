use std::f64::consts::PI;
use std::ops::{BitAnd, BitOr, BitOrAssign, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PingKind {
    Retreat,
    Push,
    OnMyWay,
    AllIn,
    Assist,
    NeedVision,
    Missing,
    EnemyVision,
    Generic,
}

impl PingKind {
    pub const ALL: [Self; 9] = [
        Self::Retreat,
        Self::Push,
        Self::OnMyWay,
        Self::AllIn,
        Self::Assist,
        Self::NeedVision,
        Self::Missing,
        Self::EnemyVision,
        Self::Generic,
    ];
    pub const WHEEL: [Self; 8] = [
        Self::Retreat,
        Self::Push,
        Self::OnMyWay,
        Self::AllIn,
        Self::Assist,
        Self::NeedVision,
        Self::Missing,
        Self::EnemyVision,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Retreat => "retreat",
            Self::Push => "push",
            Self::OnMyWay => "onMyWay",
            Self::AllIn => "allIn",
            Self::Assist => "assist",
            Self::NeedVision => "needVision",
            Self::Missing => "missing",
            Self::EnemyVision => "enemyVision",
            Self::Generic => "generic",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Retreat => "撤退",
            Self::Push => "推进",
            Self::OnMyWay => "正在路上",
            Self::AllIn => "全力进攻",
            Self::Assist => "请求协助",
            Self::NeedVision => "需要视野",
            Self::Missing => "敌人消失",
            Self::EnemyVision => "敌方视野",
            Self::Generic => "普通信号",
        }
    }

    pub fn icon_name(self) -> &'static str {
        match self {
            Self::Retreat => "retreat",
            Self::Push => "push",
            Self::OnMyWay => "on_my_way_new",
            Self::AllIn => "all_in",
            Self::Assist => "assist",
            Self::NeedVision => "need_ward",
            Self::Missing => "mia_new",
            Self::EnemyVision => "area_is_warded_small_red_new",
            Self::Generic => "ping",
        }
    }

    pub fn sound_name(self) -> &'static str {
        match self {
            Self::Retreat => "retreat",
            Self::Push => "push",
            Self::OnMyWay => "on_my_way",
            Self::AllIn => "all_in",
            Self::Assist => "assist_me",
            Self::NeedVision => "need_vision",
            Self::Missing => "q_mark",
            Self::EnemyVision => "enemy_vision",
            Self::Generic => "alert",
        }
    }

    pub fn code(self) -> u16 {
        match self {
            Self::Generic => 0,
            Self::OnMyWay => 1,
            Self::Assist => 2,
            Self::Missing => 3,
            Self::EnemyVision => 4,
            Self::Retreat => 5,
            Self::Push => 6,
            Self::AllIn => 7,
            Self::NeedVision => 8,
        }
    }

    pub fn from_code(code: u16) -> Option<Self> {
        Some(match code {
            0 => Self::Generic,
            1 => Self::OnMyWay,
            2 => Self::Assist,
            3 => Self::Missing,
            4 => Self::EnemyVision,
            5 => Self::Retreat,
            6 => Self::Push,
            7 => Self::AllIn,
            8 => Self::NeedVision,
            _ => return None,
        })
    }

    pub fn tint(self) -> u32 {
        match self {
            Self::Retreat | Self::EnemyVision => 0xff4d4d,
            Self::Missing | Self::AllIn => 0xffcc05,
            Self::Assist | Self::Push | Self::NeedVision => 0x00e399,
            Self::OnMyWay | Self::Generic => 0x1ab8f2,
        }
    }

    pub fn wheel_idle_tint(self) -> u32 {
        match self {
            Self::Push | Self::AllIn | Self::NeedVision | Self::EnemyVision => 0xccc191,
            _ => self.tint(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const EMPTY: Self = Self(0);
    pub const CONTROL: Self = Self(1);
    pub const OPTION: Self = Self(2);
    pub const COMMAND: Self = Self(4);
    pub const SHIFT: Self = Self(8);

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub fn subtracting(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub fn bits(self) -> u8 {
        self.0
    }
}

impl BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for Modifiers {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Sub for Modifiers {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.subtracting(rhs)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TriggerChord {
    ControlOptionCommand,
    ControlOptionShift,
    ControlCommandShift,
}

impl TriggerChord {
    pub const ALL: [Self; 3] = [
        Self::ControlOptionCommand,
        Self::ControlOptionShift,
        Self::ControlCommandShift,
    ];

    pub fn raw_value(self) -> &'static str {
        match self {
            Self::ControlOptionCommand => "controlOptionCommand",
            Self::ControlOptionShift => "controlOptionShift",
            Self::ControlCommandShift => "controlCommandShift",
        }
    }

    pub fn from_raw_value(value: &str) -> Option<Self> {
        Some(match value {
            "controlOptionCommand" => Self::ControlOptionCommand,
            "controlOptionShift" => Self::ControlOptionShift,
            "controlCommandShift" => Self::ControlCommandShift,
            _ => return None,
        })
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::ControlOptionCommand => "⌃ ⌥ ⌘  Control + Option + Command",
            Self::ControlOptionShift => "⌃ ⌥ ⇧  Control + Option + Shift",
            Self::ControlCommandShift => "⌃ ⌘ ⇧  Control + Command + Shift",
        }
    }

    pub fn symbols(self) -> &'static str {
        match self {
            Self::ControlOptionCommand => "⌃  ⌥  ⌘",
            Self::ControlOptionShift => "⌃  ⌥  ⇧",
            Self::ControlCommandShift => "⌃  ⌘  ⇧",
        }
    }

    pub fn modifiers(self) -> Modifiers {
        match self {
            Self::ControlOptionCommand => {
                Modifiers::CONTROL | Modifiers::OPTION | Modifiers::COMMAND
            }
            Self::ControlOptionShift => Modifiers::CONTROL | Modifiers::OPTION | Modifiers::SHIFT,
            Self::ControlCommandShift => Modifiers::CONTROL | Modifiers::COMMAND | Modifiers::SHIFT,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PingPoint {
    pub x: f64,
    pub y: f64,
}

impl PingPoint {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PingRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl PingRect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn min_x(self) -> f64 {
        self.x
    }

    pub fn max_x(self) -> f64 {
        self.x + self.width
    }

    pub fn min_y(self) -> f64 {
        self.y
    }

    pub fn max_y(self) -> f64 {
        self.y + self.height
    }

    pub fn contains(self, point: PingPoint) -> bool {
        point.x >= self.min_x()
            && point.x <= self.max_x()
            && point.y >= self.min_y()
            && point.y <= self.max_y()
    }
}

pub struct WheelGeometry;

impl WheelGeometry {
    pub const CENTER_RADIUS: f64 = 66.0;
    pub const ICON_RADIUS: f64 = 101.0;
    pub const OUTER_RADIUS: f64 = 140.0;

    pub fn direction(point: PingPoint, center: PingPoint, dead_zone: f64) -> Option<f64> {
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        if dx.hypot(dy) <= dead_zone {
            None
        } else {
            Some(dx.atan2(-dy))
        }
    }

    pub fn selection(point: PingPoint, center: PingPoint, dead_zone: f64) -> PingKind {
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        if dx.hypot(dy) <= dead_zone {
            return PingKind::Generic;
        }
        let clockwise_from_north = dx.atan2(-dy);
        let index = ((clockwise_from_north + PI / 8.0) / (PI / 4.0)).floor() as i32;
        PingKind::WHEEL[index.rem_euclid(8) as usize]
    }

    pub fn clamped_center(anchor: PingPoint, frame: PingRect, radius: f64) -> PingPoint {
        fn clamp(value: f64, low: f64, high: f64) -> f64 {
            if low > high {
                (low + high) / 2.0
            } else {
                value.max(low).min(high)
            }
        }
        PingPoint::new(
            clamp(anchor.x, frame.min_x() + radius, frame.max_x() - radius),
            clamp(anchor.y, frame.min_y() + radius, frame.max_y() - radius),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GesturePhase {
    Idle,
    Arming,
    Open,
    Blocked,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GestureAction {
    Arm(PingPoint),
    Show(PingPoint),
    Hover(PingKind, Option<f64>),
    Hide,
    Dismiss,
    Commit(PingKind, PingPoint),
}

#[derive(Debug)]
pub struct GestureMachine {
    pub phase: GesturePhase,
    pub selected: PingKind,
    pub anchor: PingPoint,
    pub current_modifiers: Modifiers,
    pub chord: TriggerChord,
    pub dead_zone: f64,
    center: PingPoint,
    pointer: PingPoint,
}

impl Default for GestureMachine {
    fn default() -> Self {
        Self {
            phase: GesturePhase::Idle,
            selected: PingKind::Generic,
            anchor: PingPoint::default(),
            current_modifiers: Modifiers::EMPTY,
            chord: TriggerChord::ControlOptionCommand,
            dead_zone: WheelGeometry::CENTER_RADIUS,
            center: PingPoint::default(),
            pointer: PingPoint::default(),
        }
    }
}

impl GestureMachine {
    pub fn flags_changed(
        &mut self,
        flags: Modifiers,
        point: PingPoint,
        other_input_held: bool,
    ) -> Vec<GestureAction> {
        self.current_modifiers = flags;
        if self.phase == GesturePhase::Blocked {
            if flags.intersection(self.chord.modifiers()).is_empty() {
                self.phase = GesturePhase::Idle;
            }
            return Vec::new();
        }
        if self.phase == GesturePhase::Idle {
            if flags != self.chord.modifiers() {
                return Vec::new();
            }
            if other_input_held {
                self.phase = GesturePhase::Blocked;
                return Vec::new();
            }
            self.anchor = point;
            self.center = point;
            self.pointer = point;
            self.selected = PingKind::Generic;
            self.phase = GesturePhase::Arming;
            return vec![GestureAction::Arm(point)];
        }
        if !flags.subtracting(self.chord.modifiers()).is_empty() {
            return self.cancel();
        }
        if flags != self.chord.modifiers() {
            let was_open = self.phase == GesturePhase::Open;
            self.phase = if flags.intersection(self.chord.modifiers()).is_empty() {
                GesturePhase::Idle
            } else {
                GesturePhase::Blocked
            };
            return if was_open {
                vec![
                    GestureAction::Dismiss,
                    GestureAction::Commit(self.selected, self.anchor),
                ]
            } else {
                vec![GestureAction::Hide]
            };
        }
        Vec::new()
    }

    pub fn delay_elapsed(&mut self) -> Vec<GestureAction> {
        if self.phase != GesturePhase::Arming || self.current_modifiers != self.chord.modifiers() {
            return Vec::new();
        }
        self.phase = GesturePhase::Open;
        vec![GestureAction::Show(self.anchor)]
    }

    pub fn set_wheel_center(&mut self, point: PingPoint) -> Vec<GestureAction> {
        self.center = point;
        if self.pointer.distance(self.anchor) <= 4.0 {
            return Vec::new();
        }
        self.moved_to(self.pointer)
    }

    pub fn moved_to(&mut self, point: PingPoint) -> Vec<GestureAction> {
        self.pointer = point;
        if self.phase != GesturePhase::Open {
            return Vec::new();
        }
        let next = WheelGeometry::selection(point, self.center, self.dead_zone);
        self.selected = next;
        vec![GestureAction::Hover(
            next,
            WheelGeometry::direction(point, self.center, self.dead_zone),
        )]
    }

    pub fn cancel(&mut self) -> Vec<GestureAction> {
        let had_gesture = self.phase == GesturePhase::Arming || self.phase == GesturePhase::Open;
        self.phase = if self
            .current_modifiers
            .intersection(self.chord.modifiers())
            .is_empty()
        {
            GesturePhase::Idle
        } else {
            GesturePhase::Blocked
        };
        self.selected = PingKind::Generic;
        if had_gesture {
            vec![GestureAction::Hide]
        } else {
            Vec::new()
        }
    }

    pub fn reset(&mut self, block_until_release: bool) {
        self.phase = if block_until_release {
            GesturePhase::Blocked
        } else {
            GesturePhase::Idle
        };
        self.selected = PingKind::Generic;
        self.current_modifiers = Modifiers::EMPTY;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHORD: Modifiers = Modifiers(7);
    const ANCHOR: PingPoint = PingPoint::new(600.0, 400.0);

    fn open(machine: &mut GestureMachine) {
        assert_eq!(
            machine.flags_changed(CHORD, ANCHOR, false),
            vec![GestureAction::Arm(ANCHOR)]
        );
        assert_eq!(machine.delay_elapsed(), vec![GestureAction::Show(ANCHOR)]);
    }

    #[test]
    fn eight_directions_and_central_dead_zone() {
        let vectors = [
            PingPoint::new(0.0, -100.0),
            PingPoint::new(100.0, -100.0),
            PingPoint::new(100.0, 0.0),
            PingPoint::new(100.0, 100.0),
            PingPoint::new(0.0, 100.0),
            PingPoint::new(-100.0, 100.0),
            PingPoint::new(-100.0, 0.0),
            PingPoint::new(-100.0, -100.0),
        ];
        for (index, vector) in vectors.into_iter().enumerate() {
            assert_eq!(
                WheelGeometry::selection(
                    vector,
                    PingPoint::default(),
                    WheelGeometry::CENTER_RADIUS
                ),
                PingKind::WHEEL[index]
            );
        }
        assert_eq!(
            WheelGeometry::selection(PingPoint::new(66.0, 0.0), PingPoint::default(), 66.0),
            PingKind::Generic
        );
        assert_eq!(
            WheelGeometry::selection(PingPoint::new(67.0, 0.0), PingPoint::default(), 66.0),
            PingKind::OnMyWay
        );
    }

    #[test]
    fn short_press_never_pings_and_late_timer_does_nothing() {
        let mut machine = GestureMachine::default();
        let _ = machine.flags_changed(CHORD, ANCHOR, false);
        assert_eq!(
            machine.flags_changed(Modifiers::EMPTY, ANCHOR, false),
            vec![GestureAction::Hide]
        );
        assert!(machine.delay_elapsed().is_empty());
    }

    #[test]
    fn release_commits_exactly_once_at_original_anchor() {
        let mut machine = GestureMachine::default();
        open(&mut machine);
        assert_eq!(
            machine.moved_to(PingPoint::new(500.0, 400.0)),
            vec![GestureAction::Hover(PingKind::Missing, Some(-PI / 2.0))]
        );
        assert_eq!(
            machine.flags_changed(
                Modifiers::CONTROL | Modifiers::OPTION,
                PingPoint::default(),
                false
            ),
            vec![
                GestureAction::Dismiss,
                GestureAction::Commit(PingKind::Missing, ANCHOR)
            ]
        );
        assert!(machine
            .flags_changed(Modifiers::OPTION, PingPoint::default(), false)
            .is_empty());
        assert!(machine
            .flags_changed(Modifiers::EMPTY, PingPoint::default(), false)
            .is_empty());
        assert_eq!(machine.phase, GesturePhase::Idle);
    }

    #[test]
    fn all_release_orders_commit_exactly_once() {
        let keys = [Modifiers::CONTROL, Modifiers::OPTION, Modifiers::COMMAND];
        for first in keys {
            for second in keys.into_iter().filter(|key| *key != first) {
                let mut machine = GestureMachine::default();
                open(&mut machine);
                let mut remaining = CHORD;
                let mut commits = 0;
                let third = keys
                    .into_iter()
                    .find(|key| *key != first && *key != second)
                    .unwrap();
                for key in [first, second, third] {
                    remaining = remaining.subtracting(key);
                    for action in machine.flags_changed(remaining, ANCHOR, false) {
                        if matches!(action, GestureAction::Commit(_, _)) {
                            commits += 1;
                        }
                    }
                }
                assert_eq!(commits, 1);
                assert_eq!(machine.phase, GesturePhase::Idle);
            }
        }
    }

    #[test]
    fn repressing_one_key_does_not_create_second_ping() {
        let mut machine = GestureMachine::default();
        open(&mut machine);
        let _ = machine.flags_changed(Modifiers::CONTROL | Modifiers::OPTION, ANCHOR, false);
        assert!(machine.flags_changed(CHORD, ANCHOR, false).is_empty());
        assert!(machine.delay_elapsed().is_empty());
        let _ = machine.flags_changed(Modifiers::EMPTY, ANCHOR, false);
        open(&mut machine);
    }

    #[test]
    fn cancel_and_other_shortcuts_require_full_release() {
        let mut machine = GestureMachine::default();
        open(&mut machine);
        assert_eq!(machine.cancel(), vec![GestureAction::Hide]);
        assert!(machine
            .flags_changed(Modifiers::OPTION, ANCHOR, false)
            .is_empty());
        let _ = machine.flags_changed(CHORD, ANCHOR, false);
        assert!(machine.delay_elapsed().is_empty());
        let _ = machine.flags_changed(Modifiers::EMPTY, ANCHOR, false);
        open(&mut machine);
    }

    #[test]
    fn fourth_modifier_cancels_instead_of_committing() {
        let mut machine = GestureMachine::default();
        open(&mut machine);
        assert_eq!(
            machine.flags_changed(CHORD | Modifiers::SHIFT, ANCHOR, false),
            vec![GestureAction::Hide]
        );
        assert!(machine
            .flags_changed(Modifiers::EMPTY, ANCHOR, false)
            .is_empty());
    }

    #[test]
    fn existing_drag_or_key_press_does_not_arm() {
        let mut machine = GestureMachine::default();
        assert!(machine.flags_changed(CHORD, ANCHOR, true).is_empty());
        assert!(machine.delay_elapsed().is_empty());
        assert_eq!(machine.phase, GesturePhase::Blocked);
    }

    #[test]
    fn reset_during_animation_cannot_commit() {
        let mut machine = GestureMachine::default();
        open(&mut machine);
        machine.reset(false);
        assert!(machine
            .flags_changed(Modifiers::EMPTY, ANCHOR, false)
            .is_empty());
        assert!(machine.delay_elapsed().is_empty());
    }

    #[test]
    fn edge_clamping_preserves_actual_ping_location() {
        let mut machine = GestureMachine::default();
        let edge = PingPoint::new(-1915.0, 5.0);
        let display = PingRect::new(-1920.0, 0.0, 1920.0, 1080.0);
        let center = WheelGeometry::clamped_center(edge, display, 150.0);
        assert_eq!(center, PingPoint::new(-1770.0, 150.0));
        let _ = machine.flags_changed(CHORD, edge, false);
        let _ = machine.delay_elapsed();
        assert!(machine.set_wheel_center(center).is_empty());
        let _ = machine.moved_to(PingPoint::new(center.x + 90.0, center.y));
        assert_eq!(
            machine.flags_changed(Modifiers::EMPTY, edge, false),
            vec![
                GestureAction::Dismiss,
                GestureAction::Commit(PingKind::OnMyWay, edge)
            ]
        );
    }

    #[test]
    fn movement_during_arming_is_applied_after_wheel_opens() {
        let mut machine = GestureMachine::default();
        let _ = machine.flags_changed(CHORD, ANCHOR, false);
        let _ = machine.moved_to(PingPoint::new(600.0, 500.0));
        let _ = machine.delay_elapsed();
        assert_eq!(
            machine.set_wheel_center(ANCHOR),
            vec![GestureAction::Hover(PingKind::Assist, Some(PI))]
        );
    }

    #[test]
    fn every_configurable_chord() {
        for trigger in TriggerChord::ALL {
            let mut machine = GestureMachine::default();
            machine.chord = trigger;
            assert_eq!(
                machine.flags_changed(trigger.modifiers(), ANCHOR, false),
                vec![GestureAction::Arm(ANCHOR)]
            );
            let _ = machine.delay_elapsed();
            assert_eq!(
                machine.flags_changed(Modifiers::EMPTY, ANCHOR, false),
                vec![
                    GestureAction::Dismiss,
                    GestureAction::Commit(PingKind::Generic, ANCHOR)
                ]
            );
        }
    }

    #[test]
    fn continuous_direction_and_center_reset() {
        let mut machine = GestureMachine::default();
        open(&mut machine);
        let first = PingPoint::new(600.0, 300.0);
        let second = PingPoint::new(610.0, 300.0);
        assert_eq!(
            machine.moved_to(first),
            vec![GestureAction::Hover(PingKind::Retreat, Some(0.0))]
        );
        assert_eq!(
            machine.moved_to(second),
            vec![GestureAction::Hover(
                PingKind::Retreat,
                Some(10.0_f64.atan2(100.0))
            )]
        );
        assert_eq!(machine.selected, PingKind::Retreat);
        assert_eq!(
            machine.moved_to(PingPoint::new(600.0, 334.0)),
            vec![GestureAction::Hover(PingKind::Generic, None)]
        );
        assert_eq!(
            machine.flags_changed(Modifiers::EMPTY, ANCHOR, false),
            vec![
                GestureAction::Dismiss,
                GestureAction::Commit(PingKind::Generic, ANCHOR)
            ]
        );
        assert!(machine.moved_to(first).is_empty());
    }

    #[test]
    fn scaled_center_matches_visual_circle() {
        for scale in [0.75, 1.0, 1.5] {
            let mut machine = GestureMachine::default();
            machine.dead_zone = WheelGeometry::CENTER_RADIUS * scale;
            open(&mut machine);
            assert_eq!(
                machine.moved_to(PingPoint::new(ANCHOR.x + 66.0 * scale, ANCHOR.y)),
                vec![GestureAction::Hover(PingKind::Generic, None)]
            );
            assert_eq!(
                machine.moved_to(PingPoint::new(ANCHOR.x + 66.0 * scale + 0.01, ANCHOR.y)),
                vec![GestureAction::Hover(PingKind::OnMyWay, Some(PI / 2.0))]
            );
        }
    }

    #[test]
    fn ping_codes_round_trip() {
        for kind in PingKind::ALL {
            assert_eq!(PingKind::from_code(kind.code()), Some(kind));
        }
        assert_eq!(PingKind::from_code(9), None);
    }
}
