# 0.1.0 验证记录

环境：macOS ARM64，Rust 1.98.0，Node 26.9.0；依赖由 Cargo.lock / package-lock.json 锁定。

## 已执行

- Rust 单元/文件事务测试 9 项通过：域名注入、目录校验、规则幂等、未知策略、路径穿越、过期预览、备份还原、外部编辑回滚保护、GitHub 地址限制。
- 前端纯逻辑测试 5 项通过：个人优先、跟随订阅、远端差异、HTML 转义。
- `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、TypeScript/Vite 生产构建通过。
- Playwright 隔离浏览器界面测试通过：服务搜索/编辑、个人规则添加删除、非法域名、语言切换和页面导航。使用已安装的 Chrome 的全新隔离上下文；未访问用户浏览器配置。
- 本机生产 APP/DMG 构建成功；原生 WebView 启动、读取实际 Clash 当前订阅成功（只读）。
- 原生 WebView 针对仓库 `tests/fixtures/clash` 的预览 → 备份写入 → 回滚成功，`git diff` 确认夹具原始字节还原。没有对正式 Clash 配置写入。
- 原生客户端成功下载已推送的 GitHub 规则目录，展示版本、SHA-256、零差异预览。
- npm audit 未报告漏洞；cargo-audit 0.22.2 扫描 449 个锁定依赖，未报告已登记漏洞，但有 3 项告警：RUSTSEC-2024-0370（proc-macro-error 停止维护）、RUSTSEC-2024-0449（unsafe-libyaml 停止维护）、RUSTSEC-2024-0429（Linux glib 0.18.5 VariantStrIter 不健全实现）。业务代码未直接调用后者，但不宣称传递调用完全不可达；保留预览版风险提示。
- 最终 Mac APP 的 `codesign --verify --deep --strict` 通过（ad-hoc）；动态依赖为系统框架，没有开发机依赖路径。

## 构建中发现并修复

- 初始 Linux AppImage 打包缺少显式方形图标配置；已在 Tauri bundle 中声明 PNG/ICO/ICNS 图标，修复后的 CI 已通过。
- 初始 Mac 包未显式签署外层资源；已改为最终包 ad-hoc 签名并通过完整性检查。
- 国内网络下开发依赖下载曾超时；构建命令使用临时本机代理重试，未改系统配置。规则同步提供独立可选本机代理。

## 未验证边界

- Windows/Linux 和 Intel Mac 无本地真机安装/用户业务操作验证，CI 构建不等于真机验收。
- 无 Apple Developer ID/公证、Windows Authenticode、软件自动更新签名；不发布为稳定强制升级。
- 尚无历史版本升级迁移测试（首版）。
- 不验证 DNS 冲突、休眠恢复、特定 AI 服务登录、第三方节点稳定性；本工具只管理路由配置。
- 规则写入后的 Clash 热重载由用户执行；实际全局增强/脚本最终顺序需要 Clash 内核中复核。

## 发布构建

- 应用源码提交：`1fd323c791cd852dcddcfd68c02453cb775f5028`。
- [CI 38412685085](https://github.com/luckySpro/clashvergerulls/actions/runs/38412685085)：macOS ARM64、macOS Intel、Windows x64、Linux x64 全部通过测试、Clippy、生产打包。
- 产物：两个 Mac DMG、一个 Windows NSIS EXE、Linux DEB 与 AppImage。
- 下载 CI 生成的两个 DMG 并验证磁盘镜像及 ad-hoc 签名完整性；分别确认 arm64/x86_64。ARM64 CI 包内只读诊断在隔离夹具通过。测试挂载已卸载。
- Linux 包验证为 amd64，DEB 声明 WebKitGTK 4.1/GTK3 依赖。未在 Mac 上执行 Linux/Windows 安装包。
- 所有公开附件的校验清单与已知风险以 Release 的 SHA256SUMS.txt 和发布说明为准。
