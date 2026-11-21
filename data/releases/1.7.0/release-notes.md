# Agent Minibar 1.7.0

## 中文说明

- 新增 WorkBuddy 提供商，豆包 Agent 可在提供商设置中切换为豆包图标。
- Windows 新用户首次启动即显示悬浮额度圈。悬浮圈和迷你额度卡的背景透明度可分别调节，额度数字增加自动对比描边。
- 检测到独立登录的 Codex 桌面版和 CLI 时，可分配给两个悬浮圈，各自查看剩余额度。
- Codex 通过共享 App Server 报告明确的模型重路由时亮红灯，详情显示请求模型、实际路由和原因。未接入该事件来源时显示无法检测。
- 左键点击另一个悬浮圈切换详情，再点击当前悬浮圈收起详情。窗口重叠时，设置和详情显示在悬浮圈上方。

### Windows 使用方法

1. 下载 `agent-minibar-1.7.0-x64-portable.zip`。
2. 用 `SHA256SUMS` 核对 ZIP 的 SHA-256。
3. 完整解压并运行其中的 `agent-minibar.exe`。

已有旧版时，先退出正在运行的 Agent Minibar，再启动新版本。

## English

- Added WorkBuddy and a selectable Doubao icon for Doubao Agent.
- New Windows installations show the floating circle by default. The circle and compact quota card have separate surface opacity controls, and quota values have contrast-aware outlines.
- Independently signed-in Codex desktop and CLI profiles can use separate floating circles. Confirmed model reroutes reported through a shared Codex App Server show a red indicator and the requested model, route, and reason in details.
- Clicking a different circle switches the detail card. Clicking the same circle again dismisses it. Settings and details appear above overlapping circles.

Download `agent-minibar-1.7.0-x64-portable.zip`, verify it against `SHA256SUMS`, extract the complete archive, and run `agent-minibar.exe`.
