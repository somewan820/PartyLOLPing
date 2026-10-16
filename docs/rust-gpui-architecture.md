# Rust + GPUI 当前架构

## 目标

PartyLOLPing 的桌面前端、全局 Ping、房间状态和局域网后端统一使用 Rust。最终 App 只包含 GPUI 可执行文件，项目只维护这一套运行时。

Rust crate 根目录是 `rust/`；`Cargo.toml`、`src/` 和 Cargo 生成的 `target/` 都直接位于该目录下。

## 模块分工

### Rust + GPUI

- `rust/src/ping.rs`：九种 Ping、组合键、轮盘几何和纯手势状态机，以及对应的 Rust 单元测试。
- `rust/src/bin/partylolping/ping_input.rs`：辅助功能检测、CGEventTap、0.18 秒计时、取消与吞键、异常恢复。
- `rust/src/bin/partylolping/ping_overlay.rs`：GPUI 透明轮盘和落点窗口、矢量图标、扇区高亮、动画与多屏布局。
- `rust/src/bin/partylolping/ping_controller.rs`：输入、覆盖层、音效、设置和菜单栏的统一生命周期，以及房间 Ping 提交。
- `rust/src/bin/partylolping/ping_settings.rs`：沿用原有 NSUserDefaults 设置键和系统权限设置入口。
- `rust/src/bin/partylolping/ping_sound.rs`：使用 AVAudioPlayer 播放九种现有音效。
- `rust/src/bin/partylolping/ping_status_bar.rs`：菜单栏状态、启用、打开主窗口和退出操作。
- `rust/src/bin/partylolping.rs`：GPUI 应用启动、窗口生命周期和顶层路由。
- `rust/src/bin/partylolping/app_state.rs`：房间页面、Ping 页面共用的状态模型和显示常量。
- `rust/src/bin/partylolping/room_page.rs`：创建、发现、加入、退出房间，以及房间成员和 Ping 广播页面。
- `rust/src/bin/partylolping/ping_page.rs`：桌面信号页面、设置控件和九种信号预览。
- `rust/src/bin/partylolping/widgets.rs`：房间成员卡片、按钮、标签、图标和面板等可复用 GPUI 组件。

### macOS 原生边界

GPUI 未覆盖的系统能力通过 Rust FFI 实现：

- ApplicationServices 辅助功能权限
- CoreGraphics 全局事件与屏幕坐标
- NSWindow 鼠标穿透、窗口层级和全 Space 行为
- NSUserDefaults 设置持久化
- AVAudioPlayer 音效
- NSStatusItem 菜单栏

房间、协议、Bonjour、TCP、JPEG 采集和 Ping 路由继续由 `party-engine` 提供，不因 UI 迁移而重写。

## 构建与测试

```sh
zsh scripts/dev.sh
zsh scripts/test.sh
zsh scripts/build.sh
dist/PartyLOLPing.app/Contents/MacOS/PartyLOLPing --check-assets
```

`scripts/test.sh` 运行格式检查、全部 Rust 测试和 GPUI 编译检查。`scripts/build.sh` 只构建 Release GPUI binary，复制矢量与音效，签名并检查隐私、Bonjour 和素材完整性。全局快捷键、权限、外接屏和不同应用的全屏覆盖仍需实机验收。
