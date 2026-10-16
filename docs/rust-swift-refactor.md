# Rust + Swift 重构路线

## 目标

在不牺牲当前 macOS UI、权限和系统集成能力的前提下，把协议和网络生命周期收敛到可独立测试的 Rust 深模块中。

“没有 bug”不能作为工程保证；可交付标准改为：每个迁移阶段都有可重复测试、旧实现可回退、崩溃路径有回归测试，且只有通过验证后才切换默认实现。

## 模块分工

### Swift 保留

- SwiftUI / AppKit 菜单栏应用和窗口
- Bonjour 服务发现与发布的 macOS 适配
- ScreenCaptureKit、辅助功能和屏幕录制权限
- Swift ↔ Rust Adapter，以及用户可读错误展示

### Rust 迁移

- 长度帧编解码和增量解析
- 控制协议状态机与序列号校验
- 房间状态变更和邀请码证明验证
- 连接生命周期、重连退避和超时策略
- TCP 连接、监听、读写和超时
- 房间成员、邀请码、Ping 路由和媒体票据的事实状态
- Swift 通过 `RustRoomBackend` 调用的稳定 C ABI

## 阶段

1. **基线**：先保留当前 Swift 网络实现，固化房主收到加入请求时的崩溃修复。
2. **纯逻辑 Rust**：Rust crate 先实现帧解析、协议状态和状态机；使用单元测试、属性测试和 fuzz 输入验证。
3. **双写校验**：Swift 旧实现与 Rust 实现同时处理同一测试流，只比较结果，不切换线上发送路径。
4. **Rust 默认、Swift 回退**：通过构建开关保留旧实现，集成测试失败自动回退。
5. **迁移真实网络**：Rust 管理连接和协议，Swift 负责 Bonjour/Network.framework 的平台 Adapter。
6. **删除旧实现**：只有经过两台 Mac 的真实局域网测试和稳定运行观察后，才移除 Swift 控制面网络旧代码。

## 当前进度

`rust/party-engine` 已提供无平台依赖的纯逻辑模块，并覆盖：

- 分片头部
- 分片 payload
- 单次输入多帧
- 空帧和超长帧
- 控制协议序列号是否为期望值
- 重试次数边界
- 连接状态终态不可重新打开

当前 Swift Adapter 已将以下路径接入 Rust：

- `TransportConnection` 的长度帧解析
- `ControlSecureSession` 的序列号检查
- `ReconnectPolicy` 的重试边界
- `TransportConnection` 的 Network.framework 状态迁移校验
- Host/Guest 控制握手阶段顺序校验
- Host listener 与 Guest connector 的真实 TCP 生命周期
- 媒体连接出站路径（Bonjour 解析后由 Rust 建立 TCP）
- 房主创建、访客加入/离开和 Ping 事件的 Rust 房间后端

Swift 与 Rust 的帧协议还增加了双实现一致性测试；当前完整 Swift 测试套件、`PingCoreChecks` 和 Rust 测试均已通过。

Swift 只拥有 Bonjour 发布/解析、macOS UI、系统权限和 ScreenCaptureKit Adapter；Rust 拥有房间事实、协议状态、控制面 socket 以及媒体出站 socket 生命周期。
`MediaListener` 仍使用 `NWListener` 作为 macOS 服务发布 Adapter，但连接建立和媒体会话授权由 Rust 后端驱动。每次迁移都先通过 Rust 单元测试、Swift 完整测试和 Release 构建，再提交推送。
