# Verge Route Studio

Clash Verge Rev 路由管理桌面工具 · Rust + Tauri 2 · **M&J Group** · 473276@qq.com

本地读取增强配置，把 Claude Code、Codex/ChatGPT、Cursor、GitHub 服务域名与国内直连策略分开管理。订阅节点不进入本仓库，也不会被上传。

## 功能

- 离线内置服务域名目录，搜索、编辑、按服务选择代理组 / DIRECT / REJECT / 跟随订阅。
- 个人域名覆盖与远程目录分离；同域名个人设置优先，子域名在父域名之前。
- 中国 GEOSITE/CN 与 GEOIP/CN 直连；局域网直连固定置于最前。需要 Clash 已有对应地理数据。
- 识别 `profiles.yaml` 当前订阅关联的 rules 增强文件。读取 `clash-verge.yaml` 的代理组名称，不编辑节点、DNS 或 TUN。
- 修改前后预览、同目录原始备份、临时文件原子替换、过期预览检测、拒绝覆盖后续编辑的回滚。
- GitHub `rules/catalog.json` 只读同步、版本/增删差异/SHA-256 展示，确认后接受，不执行远端脚本。
- 简体中文、繁体中文、英文、日文界面；技术日志保留原文。

## 使用

1. 安装并启动客户端，在「Clash 配置」确认目录并读取当前订阅。
2. 在「服务域名」选择实际存在的代理组。默认 `PROXY` 不存在时，必须改为你的组名。
3. 在「个人规则」添加其他域名，例如 `macked.app`；保持国内直连开启。
4. 保存工作区，预览写入内容，确认后「备份并写入」。
5. **在 Clash Verge 内重新加载/重新选择当前订阅，并检查实际规则。** 本工具不自动重启应用、不声称磁盘写入等于内核生效。全局增强和脚本可能改变最终规则顺序。
6. 「规则同步」输入 `luckySpro/clashvergerulls`、`main`，检查更新并预览后接受。可用提交 SHA 固定来源版本。同步不自动跟踪厂商网站，维护者更新目录后客户端才能取得新规则。

写入对象是当前订阅的 **rules 增强文件**，不是临时运行配置，也不是原始远程订阅。新订阅需在 Clash Verge 中创建关联的规则增强，再重新读取、应用。本工具不会把旧订阅规则误写到新订阅。

### 跨平台位置

| 系统 | Clash Verge Rev 默认数据目录 |
| --- | --- |
| macOS | `~/Library/Application Support/io.github.clash-verge-rev.clash-verge-rev` |
| Windows | `%APPDATA%\io.github.clash-verge-rev.clash-verge-rev` |
| Linux | `$XDG_DATA_HOME/io.github.clash-verge-rev.clash-verge-rev`，默认 `~/.local/share/...` |

使用与上游同类的 `dirs::data_dir`，另提供 config 目录候选。旧版本/便携布局手动填写。结构不兼容时明确报错，**不猜测文件并写入**。

工作区保存在系统应用数据目录下 `com.mjgroup.vergeroutestudio/workspace.json`，Mac 使用应用支持目录。备份位于原增强文件旁 `route-studio-*.bak`，不自动清理。YAML 注释/排版会规范化，未知字段和原有非托管规则保留；备份逐字节保留原文。

## 构建与验证

需要 Node 22+、Rust stable 和 [Tauri 平台依赖](https://v2.tauri.app/start/prerequisites/)。

```sh
npm ci
npm run dev                    # 仅前端预览，无原生文件访问
npm run tauri -- dev           # 原生开发窗口
npm run build
npm test
npx playwright install chromium
npx playwright test
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run tauri -- build
```

只读诊断：`verge-route-studio --inspect <Clash数据目录>`，返回目标规则路径、组名与前置规则，不返回订阅 URL 和节点凭据。

CI 分别构建 macOS ARM64/Intel DMG、Windows x64 NSIS EXE、Linux x64 DEB/AppImage。0.1.0 为开发预览：**无 Apple Developer ID 公证、无 Windows Authenticode 签名**；不会关闭系统安全策略。Windows 无 WebView2 时安装器可能需要联网，Linux 需要兼容 WebKitGTK/系统库。只有已有所需系统组件时才支持终端用户离线安装；安装后域名管理与配置编辑离线可用。

不提供自动软件升级；新版本从 GitHub Release 手动安装。规则目录同步与软件升级是两个不同机制。

## 安全与限制

- GitHub 只允许 `raw.githubusercontent.com/owner/repo/ref/rules/catalog.json`，HTTPS、25 秒超时、2 MiB 上限、拒绝重定向、严格结构和域名校验。不下载或执行 JS/YAML 脚本。
- SHA-256 仅作内容指纹，不替代签名。同步的可信度依赖你选择的 GitHub 仓库与 HTTPS。同步确认界面展示全目录。
- 文件大小有上限，目标必须是当前订阅的规则增强，拒绝路径穿越与目标符号链接。写前再次检查文件指纹。与外部程序之间不支持跨进程事务；操作时不要同时在 Clash 编辑相同文件。
- 只读 GitHub 公共规则，无账号、无遥测、无云端配置上传。不要把订阅或凭据加入 `rules/catalog.json`。
- 域名目录是可维护的起点，不是厂商完整防火墙白名单。第三方登录、遥测、自定义 API、MCP、下载源请按实际使用补充；不得以泛化进程规则代理本地开发服务。
- 跟随订阅仅取消本工具该条规则；父域名规则仍可能匹配。既有规则保留在托管规则之后，冲突会提示。禁用国内直连不会删除原配置中已有的中国直连规则。
- 本工具管理磁盘配置，不诊断/修复 DNS 劫持、Tailscale 冲突、休眠断网，也不保证代理节点与站点可用。

## 目录维护

编辑 `rules/catalog.json`，保留稳定 service ID，增加/删除域名并更新 version，通过测试后提交。每个服务保留官方参考 source。远端只能定义服务/域名，不能指定代理组或任意执行行为。

参考：[Clash Verge 自定义规则](https://www.clashverge.dev/guide/rules.html)、[上游目录实现](https://github.com/clash-verge-rev/clash-verge-rev/blob/dev/src-tauri/src/utils/dirs.rs)、目录内各厂商文档。
