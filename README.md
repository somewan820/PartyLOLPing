# PartyLOLPing · Mac 桌面信号

一个原生 Swift / AppKit 菜单栏软件。在 Mac 上按住组合键，移动鼠标选择英雄联盟风格的信号，松开发送。

## 项目说明与归属

PartyLOLPing 从 [AllenTHT/LoLPing-Mac](https://github.com/AllenTHT/LoLPing-Mac) 的固定提交 `0b9d67221f5501b6e5e660ea6d452220df4be5ac` 导入。本仓库保留上游的 PingCore、轮盘、覆盖层动画、音效、测试和素材说明，并将可执行 target、应用 bundle 和用户界面名称调整为 PartyLOLPing；当前分支已接入房间状态、Ping 路由、窗口采集/JPEG 媒体管线、远程 HUD 和分层权限界面。

游戏素材归 Riot Games 所有，音效和图像来源、版本与许可边界记录在 `Resources/asset-sources.json`。PartyLOLPing 是独立的个人桌面工具，非 Riot 官方产品。当前仓库未发布独立安装包，请按下方构建步骤从源码生成本地 App。

## 局域网 MVP 进度

房间窗口支持创建/加入、邀请码、成员卡片、定向或广播 Ping、成员静音、指定窗口共享和媒体端点的票据校验。房主通过 Bonjour 发布随机房间标识，访客使用 X25519 建立控制会话；入房、成员事件、Ping、共享状态和媒体票据都在 ChaChaPoly 加密后传输。控制连接有严格序列号与双向心跳，媒体连接使用独立 Bonjour 服务和短期票据。自动化测试使用内存 transport 跑同一套握手、加密消息、背压和房间生命周期；不会保存屏幕帧、邀请码或 Ping 历史。

当前安全边界是受信任的家庭或团队局域网。6 位邀请码会在加密通道内提交，并绑定握手 transcript；连续失败会触发 30 秒房间级锁定。它不是设备身份认证或 PAKE，恶意局域网服务仍可能干扰发现，房间级锁定也可能被滥用来短时阻止新成员加入。

真实两机/三机网络、权限拒绝、AP 隔离、网络切换和 1→5 持续运行仍按 [Verification/LAN-MATRIX.md](Verification/LAN-MATRIX.md) 手工验收。集成脚本只报告脱敏的 fake/in-memory 检查结果，不把它当作真实 Wi‑Fi 延迟或首帧指标。

## 安装

- 适用设备：Apple Silicon（M 系列芯片）Mac。
- 系统要求：macOS 13 或更新版本。
- 构建需要 Swift 命令行工具；不需要额外第三方运行库。

## 使用

1. 打开 PartyLOLPing，在「信号试用」中点选九种信号。窗口内预览不需要系统权限，也不受全局开关限制。
2. 开启「启用 Ping」。如果提示授权，进入 **系统设置 → 隐私与安全性 → 辅助功能**，允许 **PartyLOLPing**。这一步需要你亲自操作；应用不会替你更改系统权限。
3. 将鼠标放在要标记的位置，按住 **Control + Option + Command** 约 0.18 秒，移动鼠标选择，松开任意一键发送。无需按住鼠标按钮。
4. 中央是普通蓝色信号；Esc / 右键取消。信号出现在最初呼出的位置。下一次操作前，请完全松开三个修饰键。
5. 关闭总开关会立刻取消轮盘、停止动画与声音。关闭窗口仍可从菜单栏打开或暂停；「退出软件」彻底退出。

6. 在「局域网房间」中创建或加入房间，使用当前房间显示的 6 位邀请码邀请同一局域网内的成员。房间控制面和 Ping 不需要屏幕录制权限。
7. 需要共享指定窗口时，在房间窗口中点击「指定窗口共享」，允许 **系统设置 → 隐私与安全性 → 屏幕录制** 中的 **PartyLOLPing**，然后重新刷新窗口列表。屏幕录制权限只影响窗口共享，不影响入房、观看房间状态或发送 Ping。

首次运行默认关闭。启用状态、组合键、音量和大小自动保存。软件不会添加登录启动项，不联网同步信号，也不采集键盘输入内容。

菜单栏和主窗口共用一个状态。为避免快捷键冲突，在轮盘打开前后按下其他普通按键或额外修饰键，都会取消当前手势并放行原快捷键。

如果辅助功能已开启但无法触发，请关闭并重新运行应用，再检查权限帮助。只有系统确实要求时才额外开启输入监控；全局 Ping 不需要麦克风权限或屏幕录制权限。重新构建会改变本地签名，macOS 可能要求重新添加辅助功能和屏幕录制授权条目。

## 素材与效果

- 官方布局与视觉参照：<https://support.riotgames.com/en-us/league-of-legends/gameplay/smart-ping>
- 图标依据：CommunityDragon 15.24 的 `assets/ux/menu/` 轮盘图集。它是社区维护的游戏素材镜像，不是 Riot 官方服务。按轮盘图标轮廓重建可缩放矢量路径，修整压缩纹理的边缘噪点；不再放大小地图图片。
- 轮盘悬浮时图标从 22 点平滑放大到 30 点，并改变颜色与扇区高亮。中心圆半径为 66 点，实际判定区域同步缩放；金色尖角、青色短弧随指针方向旋转，功能名称显示在中心。
- 音效来源：`alibaztomars/lol-ping-overlay` 的九个音频文件，锁定具体提交；详见 `Resources/asset-sources.json`。与 Riot 原录音完全一致尚未核实，不标称官方原声。
- 游戏素材归 Riot Games 所有。PartyLOLPing 是独立的个人桌面工具，非 Riot 官方产品。
- 动画使用原生 AppKit / Quartz 绘制，包含独立变化的浮起、辉光、扩散光圈、短暂光点和约两秒淡出。蓝色普通 Ping 对照用户提供的实拍视频调整；其余八种落点动画是同风格重建，未经逐帧原版对照。
- 图标、文字和光圈按屏幕像素倍率绘制，支持 Retina。效果预览高度随大小调整，最大 150% 时也能容纳完整图标。

## 构建与检查

需要 macOS 13 或更新版本及 Swift 命令行工具。当前交付针对 Apple Silicon；无需额外第三方运行库或完整 Xcode。

```sh
zsh scripts/build.sh
zsh scripts/test.sh
dist/PartyLOLPing.app/Contents/MacOS/PartyLOLPing --check-assets
dist/PartyLOLPing.app/Contents/MacOS/PartyLOLPing --visual-check "$PWD/Verification/Visuals"
```

构建脚本生成并本地签名 `dist/PartyLOLPing.app`。首次取得素材或恢复缺失素材时可运行 `python3 scripts/fetch_assets.py`，正常构建与软件运行均无需网络。

`--visual-check` 使用与应用相同的绘制和动画插值，输出九种状态的 75% / 100% / 150% Retina 对照图、1x 图、动画阶段图及无声演示视频。它检查九种效果的动态变化和立即停止行为，不创建全局输入监听、不改偏好设置、不捕获桌面。生成视频需要系统编码器，受限沙箱可能无法调用。

轮廓数据随软件离线打包；仅维护素材时才需要 `scripts/trace_wheel_icons.py`（Pillow + NumPy），普通构建与使用不依赖 Python。

全局手势状态机有单元测试，包括所有释放顺序、重复触发、取消、其他快捷键、现有拖动、八向和屏幕边缘坐标。实际辅助功能授权、外接屏及不同应用的全屏覆盖仍需实机检查；系统受保护画面不保证覆盖。

测试使用独立的 `PingCoreChecks` 程序，因为仅安装 Command Line Tools 的 Mac 不一定带 XCTest；失败时返回非零退出状态。

## Rust + GPUI 开发模式

当前桌面前端和后端全部使用 Rust：GPUI 负责窗口/UI，`party-engine` 负责房间、局域网、Ping 和媒体传输。Swift/Electron 不参与构建。

首次开发需要完整 Xcode 和 Metal Toolchain：

```sh
sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
xcodebuild -downloadComponent MetalToolchain
```

直接增量构建并启动开发版本：

```sh
zsh scripts/dev.sh
```

只构建 Debug binary，不启动：

```sh
zsh scripts/build-dev.sh
```

运行 Rust 测试和 GPUI 编译检查：

```sh
zsh scripts/test.sh
```

构建并签名本地 Release App：

```sh
zsh scripts/build.sh
```

Cargo 缓存和构建产物位于 `rust/party-engine/target/`，不需要手动复制到 `/Applications`。开发时修改 Rust 文件后重新运行 `scripts/dev.sh` 即可增量编译。
