# macOS 防火墙与局域网发现：正式分发方案

## 问题概述

LanLink 的局域网发现（UDP 42211）和短码查询依赖**入站 UDP**。在 macOS 上，如果系统应用防火墙（ALF）开启，且应用未签名或不在白名单里，入站 UDP 会被系统拦截，表现为：

- 本机 LQ 自测正常（`127.0.0.1` 回环不受防火墙限制）
- 其他机器能看到本机（出站不拦）
- 本机看不到其他机器、别人也连不进来（入站被拦）

这不是代码 bug，也不是要求用户关防火墙能“修复”的问题——**正式分发绝不能要求用户关闭防火墙**。下面给出三条正式路径。

## 判定方法

```bash
# 防火墙是否开启（State = 1 表示开启）
/usr/libexec/ApplicationFirewall/socketfilterfw --getglobalstate
# 是否处于"阻止所有连接"
/usr/libexec/ApplicationFirewall/socketfilterfw --getblockall
```

注意：绝大多数 Mac 用户防火墙默认关闭，此问题只影响主动开启防火墙的用户。

## 方案一（推荐）：Developer ID 签名 + 公证，系统弹窗授权

macOS 的标准机制是：**已签名且经公证的应用，首次监听端口时系统会自动弹出“是否允许传入网络连接”对话框**，用户点“允许”即永久放行（记住的是代码签名标识，不是一次性路径）。用户只需点一次“允许”，不用关防火墙，也不用改系统设置。

落地步骤：

1. 加入 Apple Developer Program（$99/年），申请 `Developer ID Application` 证书；
2. Tauri 配置启用签名与公证（`crates/lanlink-desktop/tauri.conf.json` 的 `bundle.macOS`）：

```json
{
  "bundle": {
    "active": true,
    "targets": ["dmg"],
    "macOS": {
      "signingIdentity": "Developer ID Application: Your Name (TEAMID)",
      "hardenedRuntime": true,
      "entitlements": "Entitlements.plist",
      "providerShortName": "TEAMID"
    }
  }
}
```

3. CI 里用 `xcrun notarytool` 公证 + `xcrun stapler staple` 装订票据；
4. 用户首次启动点“允许传入连接”即可。

优点：体验最正规，Gatekeeper 也不会拦截（无签名应用会被“无法打开，因为来自身份不明的开发者”拦截）。缺点：需要付费开发者账号与 CI 证书配置。

## 方案二（零成本，内测/开源分发）：首次启动引导用户加白名单

无法签名时，退而求其次：应用检测到防火墙开启且自己不在白名单时，**主动弹窗引导用户把应用加入允许列表**（不是关防火墙）。

检测：

```bash
# 查看应用是否被阻止
/usr/libexec/ApplicationFirewall/socketfilterfw --getblockall
# 列出所有例外
/usr/libexec/ApplicationFirewall/socketfilterfw --listapps
```

引导文案应明确：“系统防火墙会拦截局域网发现，请点击‘允许’/在系统设置→网络→防火墙中把 LanLink 加入允许列表”，并提供一键打开设置：

```bash
open "x-apple.systempreferences:com.apple.preference.security?Firewall"
```

优点：零成本、不关防火墙、用户可自行完成。缺点：需要用户操作一次；部分 macOS 版本不接受命令行直接添加（`socketfilterfw --add` 需要 sudo），只能引导手动。

## 方案三（架构规避）：改用系统允许的发现通道

若希望彻底绕开入站 UDP 限制：

- 设备发现改用 **Apple Bonjour / NSNetService**（系统服务，防火墙通常默认放行 mDNS 与 Bonjour）；
- 或者改用**双向出站**：双方都主动向一个已知 rendezvous 地址注册，避免裸 UDP 监听。

当前实现用的是自写 UDP 42211 广播（见 `discovery.rs`），改成 Bonjour 需要较大重构，且 Windows/Linux 端要另行适配，不作为短期方案。

## 推荐组合

- **正式产品**：方案一（签名 + 公证 + 系统授权弹窗）；
- **当前内测/开源**：方案二（引导加白名单），README 写明“无需关闭防火墙，只需允许 LanLink 传入连接”。

## 与 LAN 短码的关系

短码链路（LQ/LA）完全跑在 UDP 42211 上，因此同样受防火墙影响。文件传输本身走 TCP 42210，若发现阶段被拦，用户仍可用外网序列号直连（TCP 出站不受限）——这也是 PRD 里“跨网用序列号、同网段用短码”的设计意图。
