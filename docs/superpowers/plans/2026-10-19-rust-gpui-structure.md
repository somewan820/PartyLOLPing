# Rust GPUI Structure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split the GPUI executable into focused modules while preserving the current Rust LAN behavior and UI behavior.

**Architecture:** Keep `PartyLOLPing` as the application state owner and keep its fields private to the binary crate. Move room lifecycle and room-page rendering into `room_page.rs`, desktop Ping/settings rendering into `ping_page.rs`, and reusable visual controls into `widgets.rs`. Leave `party_engine` interfaces unchanged; this is a composition refactor at the UI seam.

**Tech Stack:** Rust 2021, GPUI, existing `party-engine` crate, Cargo tests.

**Spec:** Existing Rust/GPUI application and LAN behavior in `README.md` and `Verification/LAN-MATRIX.md`.

## Global Constraints

- Preserve the public `party_engine` interfaces and LAN protocol behavior.
- Keep the shipped runtime Rust + GPUI only.
- Do not add a second UI abstraction or a new dependency.
- Keep `zsh scripts/test.sh`, `zsh scripts/integration-check.sh`, and `zsh scripts/build.sh` passing.

## Review Focus

- Room join/create/leave state transitions remain unchanged: compile and integration checks.
- Manual address/port fallback remains wired to the same `LanClient`: focused binary check and integration test.
- Ping input and settings controls remain registered with the same global controller: compile check.
- GPUI view helper return types remain compatible across module seams: `cargo check --bin partylolping`.
- No accidental changes enter the transport implementation: `git diff` limited to UI module moves plus module declarations.

### Task 1: Extract shared UI state and module declarations

**Files:**
- Create: `rust/src/bin/partylolping/app_state.rs`
- Modify: `rust/src/bin/partylolping.rs`

- [x] Move `MainPage`, `MainAppState`, `RoomMemberView`, and `PartyLOLPing` definitions into `app_state.rs`.
- [x] Add module declarations and imports without changing field types or initial defaults.
- [x] Run `cargo check --bin partylolping`.

### Task 2: Extract room workflow and room page

**Files:**
- Create: `rust/src/bin/partylolping/room_page.rs`
- Modify: `rust/src/bin/partylolping.rs`

- [x] Move room lifecycle actions (`leave_room`, `create_room`, `discover_rooms`, `join_selected`, `route_ping`, `capture_screen`) and room rendering (`joined_view`, `member_views`, `landing`, `member_grid`, `voice_member_tile`) into the module.
- [x] Keep the existing `PartyLOLPing` field access and event handlers unchanged.
- [x] Run the network-focused integration check and binary compile check.

### Task 3: Extract Ping page and reusable widgets

**Files:**
- Create: `rust/src/bin/partylolping/ping_page.rs`
- Create: `rust/src/bin/partylolping/widgets.rs`
- Modify: `rust/src/bin/partylolping.rs`

- [x] Move `ping_page` and its settings/preview controls into `ping_page.rs`.
- [x] Move shared visual helpers (buttons, panels, labels, avatars, icons, badges) into `widgets.rs`.
- [x] Keep constants and asset/controller usage explicit at the module seams.
- [x] Run the full Rust test suite and format check.

### Task 4: Verify the refactor

**Files:**
- Modify: `docs/rust-gpui-architecture.md` only if module ownership needs documentation.

- [x] Run `zsh scripts/test.sh`.
- [x] Run `zsh scripts/integration-check.sh`.
- [x] Run `zsh scripts/build.sh`.
- [x] Confirm `git diff --check` and no Swift/SwiftPM files are reintroduced.
