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
- npm audit 未报告漏洞。Rust 依赖审计结果另随 Release 记录；未完成审计不能等同安全认证。
- 最终 Mac APP 的 `codesign --verify --deep --strict` 通过（ad-hoc）；动态依赖为系统框架，没有开发机依赖路径。

## 构建中发现并修复

- 初始 Linux AppImage 打包缺少显式方形图标配置；已在 Tauri bundle 中声明 PNG/ICO/ICNS 图标，等待修复提交的 CI 验证。
- 初始 Mac 包未显式签署外层资源；已改为最终包 ad-hoc 签名并通过完整性检查。
- 国内网络下开发依赖下载曾超时；构建命令使用临时本机代理重试，未改系统配置。规则同步提供独立可选本机代理。

## 未验证边界

- Windows/Linux 和 Intel Mac 无本地真机安装/用户业务操作验证，CI 构建不等于真机验收。
- 无 Apple Developer ID/公证、Windows Authenticode、软件自动更新签名；不发布为稳定强制升级。
- 尚无历史版本升级迁移测试（首版）。
- 不验证 DNS 冲突、休眠恢复、特定 AI 服务登录、第三方节点稳定性；本工具只管理路由配置。
- 规则写入后的 Clash 热重载由用户执行；实际全局增强/脚本最终顺序需要 Clash 内核中复核。
