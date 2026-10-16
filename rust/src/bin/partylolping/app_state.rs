use super::*;

pub(crate) const BACKGROUND: u32 = 0x0b1016;
pub(crate) const SIDEBAR: u32 = 0x0e151d;
pub(crate) const PANEL: u32 = 0x121c26;
pub(crate) const PANEL_RAISED: u32 = 0x172430;
pub(crate) const PANEL_HOVER: u32 = 0x1c2d3a;
pub(crate) const BORDER: u32 = 0x243442;
pub(crate) const BORDER_BRIGHT: u32 = 0x355064;
pub(crate) const TEXT: u32 = 0xedf4f7;
pub(crate) const TEXT_SECONDARY: u32 = 0x94a8b5;
pub(crate) const TEXT_MUTED: u32 = 0x657b89;
pub(crate) const ACCENT: u32 = 0x55d6e6;
pub(crate) const ACCENT_HOVER: u32 = 0x7ae2ed;
pub(crate) const ACCENT_DARK: u32 = 0x12343d;
pub(crate) const GOLD: u32 = 0xe7c674;
pub(crate) const SUCCESS: u32 = 0x6ed7ac;
pub(crate) const DANGER: u32 = 0xef8492;
pub(crate) const DANGER_BACKGROUND: u32 = 0x352129;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainPage {
    Room,
    Ping,
}

pub(crate) struct MainAppState {
    pub(crate) window: Option<AnyWindowHandle>,
}

impl Global for MainAppState {}

#[derive(Clone)]
pub(crate) struct RoomMemberView {
    pub(crate) name: String,
    pub(crate) online: bool,
    pub(crate) is_local: bool,
    pub(crate) is_host: bool,
}

pub(crate) struct PartyLOLPing {
    pub(crate) page: MainPage,
    pub(crate) selected_room: String,
    pub(crate) status: String,
    pub(crate) room_status: String,
    pub(crate) discovery_status: String,
    pub(crate) ping_status: String,
    pub(crate) capture_status: String,
    pub(crate) invite: Option<String>,
    pub(crate) host: Option<party_engine::LanHost>,
    pub(crate) client: Option<party_engine::LanClient>,
    pub(crate) room: Option<Arc<Mutex<party_engine::Room>>>,
    pub(crate) local_member: Option<party_engine::MemberId>,
    pub(crate) discovered_rooms: Vec<party_engine::DiscoveredRoom>,
    pub(crate) selected_endpoint: Option<(String, u16)>,
    pub(crate) show_join_dialog: bool,
    pub(crate) next_ping_id: u64,
    pub(crate) nickname: Entity<InputState>,
    pub(crate) address: Entity<InputState>,
    pub(crate) port: Entity<InputState>,
    pub(crate) invite_input: Entity<InputState>,
}
