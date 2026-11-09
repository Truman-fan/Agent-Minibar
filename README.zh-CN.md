<p align="center">
  <img src="assets/app-icon.png" alt="Agent Minibar 图标" width="256" height="256">
</p>

<p align="center">
  <a href="README.md">English</a> · <strong>简体中文</strong>
</p>

<h1 align="center">Agent Minibar</h1>

<p align="center">
  面向 Windows 和 Linux 的开源 AI Agent 额度助手：在托盘、弹出窗口和悬浮圈中查看提供商的剩余额度、重置时间及账户状态。
</p>

<p align="center">
  <a href="https://github.com/Truman-fan/Agent-Minibar/releases/latest">GitHub 下载</a>
  ·
  <a href="https://gitee.com/Truman-fan/Agent-Minibar/releases">Gitee 下载</a>
  ·
  <a href="https://github.com/Truman-fan/Agent-Minibar/issues">问题反馈</a>
</p>

> Agent Minibar 是由 [Truman_show](https://github.com/Truman-fan) 独立维护的社区项目，基于 Pavel Leontyev 的
> [Codex Minibar](https://github.com/vertopolkaLF/codex-minibar) 开发。本项目不是任何 AI 提供商的官方产品，也不代表相关提供商的认可。

## 这是什么

Agent Minibar 用于汇总本机已经登录或由用户明确配置的 AI 服务账户信息。主要功能包括：

- Windows 悬浮额度圈：默认显示一个，用户可按需开启第二个，并为每个悬浮圈分别选择提供商；
- 点击悬浮圈展开额度信息卡，再次点击或点击卡片以外区域即可收起；
- 托盘弹出窗口、通知、重置倒计时、多个账户和提供商排序；
- 普通颜色或渐变色、自定义额度颜色、深色/浅色页面可读性优化；
- Windows 任务栏、PowerToys Command Palette 和 Stream Deck 集成；
- 本地用量历史与多个 CLI Agent 的只读发现。

当前提供商包括 Codex、Claude、Cursor、OpenCode、OpenRouter、Antigravity、Grok、Kiro、Command Code，以及
DeepSeek、Qwen Code、DeepSeek Harness、Kimi Code、豆包 / 火山方舟、智谱 GLM、MiniMax、百度千帆 / ERNIE、腾讯混元 / CodeBuddy。

不同提供商公开的数据并不相同。只有能够可靠取得“剩余额度或余额”的提供商才显示相应圆环；只有本地消耗记录、但没有官方剩余额度接口的提供商，会显示状态说明，而不会虚构百分比。

## 直接下载并使用

### Windows 11 / Windows 10

1. 打开 [GitHub 最新发行版](https://github.com/Truman-fan/Agent-Minibar/releases/latest)。如果 GitHub 访问不便，也可以查看 [Gitee Releases](https://gitee.com/Truman-fan/Agent-Minibar/releases)。
2. 当前首个社区版本先提供 Windows x64 便携包，适用于绝大多数 Intel 或 AMD 电脑。下载：

   ```text
   agent-minibar-<版本号>-x64-portable.zip
   ```

3. 对照同一发行版中的 `SHA256SUMS` 核对文件，再完整解压 ZIP。
4. 运行解压目录中的 `agent-minibar.exe`。不要直接在压缩包内运行，也不要只复制 EXE。
5. 在系统托盘中找到应用图标；若没有直接显示，请展开任务栏右侧的 `^` 隐藏图标区域。

不知道电脑架构时，打开 Windows 的“设置 → 系统 → 系统信息”，查看“系统类型”。绝大多数普通 Windows 11 电脑应选择 `x64`。

Windows 安装包和 ARM64 包将在后续版本加入。当前程序尚未进行代码签名，Windows 可能显示 SmartScreen 提示；请确认下载来源并核对 SHA-256 后再运行。

### 免安装便携版

当前提供的文件为：

```text
agent-minibar-<版本号>-x64-portable.zip
```

解压整个压缩包，然后运行其中的 `agent-minibar.exe`。不要只把 EXE 单独复制出来，随包的运行文件和许可文档应保留在同一目录。

### Linux

在发行版页面选择以下格式之一：

- Debian / Ubuntu：`agent-minibar_<版本号>_amd64.deb`
- 通用免安装包：`agent-minibar-<版本号>-linux-x64.AppImage`
- 其他发行版：`agent-minibar-<版本号>-linux-x64.tar.gz`

Debian / Ubuntu 安装示例：

```bash
sudo apt install ./agent-minibar_<版本号>_amd64.deb
```

AppImage 使用示例：

```bash
chmod +x agent-minibar-<版本号>-linux-x64.AppImage
./agent-minibar-<版本号>-linux-x64.AppImage
```

## 首次配置

1. 从托盘菜单打开“设置”。
2. 进入“提供商”，启用需要查看的 Agent 或服务，并根据页面提示选择本机 CLI、登录状态或填写 API 凭据。
3. 进入“自定义”，启用悬浮圈并选择第一个悬浮圈所显示的提供商。
4. 如有需要，在第一个悬浮圈的信息卡中打开第二个悬浮圈，再为它选择另一个提供商。
5. 根据喜好调整额度颜色、渐变、透明度、圆环和信息卡样式。

Agent Minibar 不会要求你把令牌提交到仓库。不要在 Issue、截图、日志或聊天中公开 API Key、Cookie、OAuth Token 或其他账户凭据。

## 如何使用这个源码仓库

仓库有两种常见用途：

- **只想使用软件：** 前往 Releases 下载安装程序或便携包，不需要下载源码；
- **想开发或自行编译：** 克隆仓库，安装 Rust 与 Windows 构建工具后执行构建命令。

点击 GitHub 的 `Code → Download ZIP` 得到的是源码压缩包，不是可直接运行的 Windows 安装包。

### 克隆源码

GitHub：

```powershell
git clone https://github.com/Truman-fan/Agent-Minibar.git
cd Agent-Minibar
```

国内网络也可以使用 Gitee：

```powershell
git clone https://gitee.com/Truman-fan/Agent-Minibar.git
cd Agent-Minibar
```

GitHub 是主要开发仓库，Gitee 是同步镜像。为避免分支冲突，建议只向 GitHub 提交 Issue 和 Pull Request。

### Windows 从源码检查与构建

准备环境：

- Windows 10 或 Windows 11；
- Git；
- `rust-toolchain.toml` 指定的 Rust 工具链；
- Visual Studio Build Tools，并安装“使用 C++ 的桌面开发”；
- 生成安装程序时需要 NSIS，`build.ps1` 可自动下载项目指定版本。

先检查代码：

```powershell
.\cargo-worktree.ps1 check --locked
.\cargo-worktree.ps1 test --all-targets --all-features --locked
.\cargo-worktree.ps1 clippy --all-targets --all-features --locked '--' -D warnings
```

只构建当前常用的 Windows x64 安装包与便携包：

```powershell
.\build.ps1 -Arch x64
```

输出位于：

```text
dist\<版本号>\
```

只需要便携包、不生成安装程序时：

```powershell
.\build.ps1 -Arch x64 -SkipInstaller
```

构建所有 Windows 架构：

```powershell
.\build.ps1
```

构建脚本不会启动 Agent Minibar；生成的程序需要你自行打开测试。

### Linux 从源码构建

Debian / Ubuntu 示例：

```bash
sudo apt install build-essential pkg-config libxkbcommon-dev libxkbcommon-x11-dev \
  libwayland-dev libx11-dev libx11-xcb-dev libxcb1-dev libxcb-xkb-dev \
  libvulkan-dev libfontconfig-dev libfreetype-dev libdbus-1-dev libssl-dev \
  libzstd-dev patchelf
cargo test --all-targets --all-features --locked
./build-linux.sh
```

产物同样位于 `dist/<版本号>/`。

## 更新与发布来源

应用首先检查 [GitHub Releases](https://github.com/Truman-fan/Agent-Minibar/releases)，GitHub 无法访问时回退到 [Gitee Releases](https://gitee.com/Truman-fan/Agent-Minibar/releases)。自动安装必须通过发布的 SHA-256 校验；缺少校验值的安装包不会被自动安装。

仓库中的 `main` 分支代表最新开发代码，不一定等同于稳定版本。普通用户应优先下载 Releases 中的正式版本。

## 问题反馈

请在 [GitHub Issues](https://github.com/Truman-fan/Agent-Minibar/issues) 提交问题，并附上：

- Windows / Linux 版本和系统架构；
- Agent Minibar 版本；
- 所使用的提供商或 CLI；
- 可重复的操作步骤；
- 已移除密钥、令牌、Cookie、邮箱和账户标识的日志或截图。

## 许可证与署名

项目采用 Apache License 2.0。详细版权、上游署名和本项目修改范围参见：

- [`LICENSE`](LICENSE)
- [`NOTICE`](NOTICE)
- [`MODIFICATIONS.md`](MODIFICATIONS.md)
- [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)
