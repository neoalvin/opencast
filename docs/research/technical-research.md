# OpenCast 技术调研报告

## 一、核心概念：媒体投屏 vs 屏幕镜像

### 1. 媒体投屏（Media Casting）—— 乐播投屏的核心模式

**工作原理**：手机（控制端）告诉电视（渲染端）一个媒体URL，电视自行下载并播放内容。

```
手机(DMC) ---发送媒体URL---> 电视(DMR) ---HTTP下载---> 内容服务器
手机此时完全自由，仅作为遥控器
```

**技术特征**：
- 手机 CPU/电量消耗极低，仅发送控制指令
- 视频质量为源文件原始质量，无重编码损失
- 手机完全可用，可切换APP、锁屏、接电话
- 支持协议：DLNA/UPnP、AirPlay（视频投屏模式）、Google Cast

### 2. 屏幕镜像（Screen Mirroring）—— 传统投屏模式

**工作原理**：手机实时捕获整个屏幕，编码为 H.264 视频流，持续传输到电视。

```
手机(源) ---H.264编码像素流(持续)--> 电视(接收端)
手机屏幕被锁定，无法独立使用
```

**技术特征**：
- 手机 CPU/电量消耗大（实时 H.264 编码）
- 质量降低（重编码+有损压缩）
- 手机不可独立使用
- 延迟 50-200ms
- 支持协议：Miracast、AirPlay（镜像模式）

### 3. 对比总结

| 特性 | 媒体投屏 (DLNA/Cast) | 屏幕镜像 (Miracast) |
|------|----------------------|---------------------|
| 发送到电视的内容 | 媒体URL（引用） | 编码像素流 |
| 谁获取内容 | 电视直接下载 | 手机捕获+编码 |
| 手机可用性 | 完全可用（仅作遥控器） | 锁定到镜像内容 |
| CPU/电量 | 极低 | 高（实时H.264编码） |
| 视频质量 | 原始质量 | 重编码，质量降低 |
| 延迟 | 无（缓冲播放） | 50-200ms |
| DRM内容 | 有限（URL需可访问） | 任何可见内容 |

---

## 二、核心协议详解

### 1. DLNA/UPnP —— 媒体投屏的基础协议

#### 三端架构模型

| 角色 | 名称 | 功能 |
|------|------|------|
| 媒体服务器 | DMS (Digital Media Server) | 暴露内容库，响应浏览/搜索请求 |
| 媒体渲染器 | DMR (Digital Media Renderer) | 接收媒体URL并播放，提供传输控制和音量控制 |
| 控制点 | DMC (Digital Media Controller) | 编排器（通常是手机APP），发现DMS/DMR，推送URL |

#### 协议栈

```
应用层:     DIDL-Lite 元数据, SOAP 操作
控制:       SOAP over HTTP (SetAVTransportURI, Play, Pause, Stop, Seek)
事件:       GENA (通用事件通知) over HTTP
描述:       XML 设备/服务描述
发现:       SSDP (简单服务发现协议) UDP 多播 239.255.255.250:1900
传输:       TCP/IP (HTTP)
```

#### 投屏流程

1. **发现**: DMC 发送 SSDP M-SEARCH 多播，发现局域网上的 DMR 设备
2. **描述**: DMC 获取 DMR 的 XML 服务描述，获取 AVTransport 控制URL
3. **推送**: DMC 发送 SOAP `SetAVTransportURI` 到 DMR（包含媒体URL）
4. **播放**: DMC 发送 `Play` 指令，DMR 独立获取URL并播放
5. **控制**: DMC 可发送 Pause/Stop/Seek/GetPositionInfo 等指令
6. **事件**: DMC 通过 GENA 订阅 LastChange 事件，异步接收状态更新

### 2. AirPlay —— 苹果投屏协议

#### 两种模式

**模式A - 媒体投屏（URL推送）**：
- 发送端通过 HTTP 提供内容URL给接收端
- Apple TV 自行获取并播放内容
- 手机完全可用（类似DLNA）

**模式B - 屏幕镜像**：
- 发送端实时编码屏幕为 H.264 视频 + AAC-ELD 音频
- 持续传输像素数据到接收端

#### 服务发现

使用 **Bonjour (mDNS/DNS-SD)**，广播两个服务：
- `_raop._tcp` (Remote Audio Output Protocol) - 音频，端口 49152
- `_airplay._tcp` (AirPlay) - 视频/镜像，端口 7000

#### 协议栈

```
发现:       mDNS 多播 224.0.0.251:5353
照片:       HTTP PUT (JPEG) 到端口 7000
视频投屏:   HTTP POST /play (内容URL) 到端口 7000
音频流:     RTSP (控制) + RTP (媒体) -- AirTunes/RAOP 协议
屏幕镜像:   H.264 over TCP (端口 7100) + AAC-ELD 音频
同步:       NTP 端口 7010-7011
```

#### AirPlay 视频投屏流程

1. 发送 `POST /play`，body 包含 `Content-Location: <URL>` 和 `Start-Position`
2. Apple TV 获取URL（支持 MP4 和 HLS）
3. 发送端轮询 `GET /playback-info` 获取状态
4. 控制：`POST /scrub?position=X`（拖动）、`POST /rate?value=1.0`（速率）、`POST /stop`（停止）

#### 认证与加密

- AirPlay 2 (2018+): HomeKit 配对 (Ed25519 + SRP)，FairPlay SAPv2.5 加密
- 多房间音频、缓冲流

### 3. Google Cast / Chromecast

#### 架构

| 组件 | 角色 |
|------|------|
| Sender | 手机/浏览器，发现接收端，发送媒体URL和控制指令 |
| Receiver | Chromecast/Android TV，运行基于Chrome的Web接收应用 |

#### 发现机制（双重）

1. **mDNS**: 广播 `_googlecast._tcp` 服务，端口 8009
2. **DIAL**: 使用 SSDP 发现 + REST API 启动应用

#### CASTV2 协议

```
传输:       TLS 1.2 on TCP 端口 8009
序列化:     Protocol Buffers (protobuf)
帧格式:     4字节大端长度前缀 + protobuf CastMessage
```

#### 核心命名空间

| 命名空间 | 用途 | 关键消息 |
|----------|------|----------|
| `com.google.cast.tp.connection` | 连接管理 | CONNECT, CLOSE |
| `com.google.cast.tp.heartbeat` | 心跳(5秒) | PING, PONG |
| `com.google.cast.receiver` | 接收端控制 | LAUNCH, STOP, GET_STATUS |
| `com.google.cast.media` | 媒体控制 | LOAD, PLAY, PAUSE, SEEK |

### 4. Miracast —— Wi-Fi Direct 屏幕镜像

**Miracast 是纯粹的屏幕镜像协议**，没有"发送URL"的概念，始终捕获、编码、传输像素。

#### 协议栈

```
会话:       RTSP (能力协商和会话管理)
传输:       RTP over UDP (MPEG2-TS 封装)
视频:       H.264 Constrained Baseline Profile
音频:       LPCM (必须), AAC (可选)
保护:       HDCP 2.x
链路层:     Wi-Fi Direct (P2P)
```

#### 连接流程

1. Wi-Fi Direct 发现（P2P IE + WFD IE）
2. P2P Group 组建（GO 协商）
3. TCP 连接到 RTSP 端口 7236
4. RTSP 能力协商（M1-M7 消息交换）
5. 开始流式传输

---

## 三、乐播投屏工作原理分析

乐播投屏通过集成多种协议提供最佳体验：

1. **对于支持的内容（视频/音频URL）**: 使用 DLNA 风格的 URL 推送。Sender SDK 从APP中提取媒体URL，推送到电视端的 Receiver SDK。电视直接获取并播放内容。手机完全可用。**这是首选模式**。

2. **对于不支持的内容（游戏/DRM内容）**: 回退到屏幕镜像。

3. **SDK 架构**: 提供 Sender SDK（集成在 8000+ 应用中）和 Receiver SDK（安装在 2.8亿+ 电视上）。Sender SDK 钩入应用的媒体播放器，拦截内容URL，然后重定向到电视。

---

## 四、现有开源项目分析

### 关键发现

> **目前不存在一个真正等同于乐播投屏的开源项目**，能在单一应用中支持所有协议（AirPlay + DLNA + Miracast + Google Cast）。

### 最有价值的参考项目

#### 第一梯队（直接可用/高度相关）

| 项目 | Stars | 语言 | 协议 | 说明 |
|------|-------|------|------|------|
| **UxPlay** | 2.6K | C/C++ | AirPlay 2 | 最佳开源 AirPlay 2 接收端，支持镜像+HLS |
| **shairport-sync** | 8.6K | C | AirPlay 1+2 | AirPlay 音频接收金标准 |
| **gmrender-resurrect** | 921 | C | DLNA | 轻量级 DLNA 渲染器，适合嵌入式 |
| **Macast** | 6.8K | Python | DLNA | PC端 DLNA 接收器，基于 mpv |
| **PyChromecast** | 2.7K | Python | Google Cast | Home Assistant 核心库 |
| **catt** | 3.6K | Python | Google Cast | Chromecast 命令行投屏 |
| **MiracleCast** | 4.2K | C | Miracast | Linux Miracast 实现 |

#### 第二梯队（库/组件级别）

| 库 | 语言 | Stars | 状态 | 说明 |
|----|------|-------|------|------|
| **pupnp (libupnp)** | C | 433 | 活跃 | UPnP 基础库 |
| **goupnp** | Go | 462 | 中等 | Go UPnP 库 |
| **UPnPCast** | Kotlin | 26 | 活跃 | Android DLNA 库，Cling 替代品 |
| **async_upnp_client** | Python | 53 | 活跃 | Python 异步 UPnP 客户端 |
| **upnp-client-rs** | Rust | 29 | 活跃 | Rust UPnP 客户端 |
| **node-castv2** | JS | - | - | Node.js Chromecast CASTV2 实现 |
| **go-chromecast** | Go | 956 | 活跃 | Go Chromecast CLI |

#### 最接近乐播投屏的项目

| 项目 | Stars | 语言 | 说明 |
|------|-------|------|------|
| **airplay_dlna_googlecast** | 63 | C/C++ | 支持 AirPlay 2 + DLNA + Google Cast + Miracast |
| **Airplay-SDK** | 3.9K | 混合 | 支持 AirPlay + DLNA，偏商业 |

### 鸿蒙（HarmonyOS）投屏

**调研结论：鸿蒙投屏目前没有任何开源实现。** 华为的 Cast+ 协议完全私有。但鸿蒙支持标准 DLNA/UPnP 进行基础媒体投屏，这意味着我们的 DLNA 实现可以覆盖鸿蒙设备。

---

## 五、OpenCast 技术方案

### 项目定位

做一个开源的、跨平台的媒体投屏方案，对标乐播投屏第一阶段功能。

### 架构设计

```
┌─────────────────────────────────────────────────────┐
│                   OpenCast 系统架构                    │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌──────────────┐          ┌──────────────────────┐ │
│  │  发送端 App   │          │     接收端 App        │ │
│  │  (手机/PC)    │          │     (电视/盒子)       │ │
│  │              │          │                      │ │
│  │ ┌──────────┐ │  控制指令  │ ┌──────────────────┐ │ │
│  │ │ 设备发现  │─│─────────│─│  协议适配层        │ │ │
│  │ │ SSDP     │ │          │ │  - DLNA DMR       │ │ │
│  │ │ mDNS     │ │          │ │  - AirPlay Server │ │ │
│  │ └──────────┘ │          │ │  - Cast Receiver  │ │ │
│  │              │          │ └────────┬─────────┘ │ │
│  │ ┌──────────┐ │          │          │           │ │
│  │ │ 投屏控制  │ │          │ ┌────────▼─────────┐ │ │
│  │ │ DMC      │ │          │ │   媒体播放器      │ │ │
│  │ │ 播放控制  │ │          │ │   (统一播放引擎)   │ │ │
│  │ └──────────┘ │          │ └──────────────────┘ │ │
│  └──────────────┘          └──────────────────────┘ │
│                                                     │
│        媒体内容直接从源服务器流向电视播放器             │
│  Content Server ──────HTTP────────> TV Player       │
└─────────────────────────────────────────────────────┘
```

### 技术选型建议

#### 方案A：Rust + 跨平台（推荐）

**理由**：
- 高性能、内存安全，适合网络协议和媒体处理
- 通过 FFI 可桥接到所有平台（Android NDK / iOS / HarmonyOS NAPI）
- 生态中已有 upnp-client-rs、crab-dlna 等可参考
- 可编译为 WASM 用于 Web 端

**核心模块划分**：

```
opencast-core/          # Rust 核心库
├── opencast-discovery/  # 设备发现 (SSDP + mDNS)
├── opencast-dlna/       # DLNA/UPnP 实现 (DMC + DMR)
├── opencast-airplay/    # AirPlay 协议实现
├── opencast-cast/       # Google Cast 协议实现
├── opencast-player/     # 统一播放引擎抽象
└── opencast-protocol/   # 通用协议抽象层

opencast-android/       # Android 应用 (Kotlin + Rust FFI)
opencast-ios/           # iOS 应用 (Swift + Rust FFI)
opencast-tv/            # TV 端接收应用
opencast-desktop/       # 桌面端 (Tauri)
```

**关键依赖**：
- 网络: `tokio` (异步运行时), `hyper` (HTTP)
- 发现: `mdns-sd` (mDNS), 自实现 SSDP
- XML: `quick-xml` (SOAP/UPnP)
- 媒体: `gstreamer-rs` 或 FFI 到平台原生播放器
- 序列化: `prost` (protobuf, 用于 Google Cast)
- TLS: `rustls` (用于 Google Cast CASTV2)

#### 方案B：Go + 跨平台

**理由**：
- 开发效率高，网络编程友好
- gomobile 可生成 Android/iOS 库
- 已有 goupnp、go-chromecast 等成熟库

**劣势**：gomobile 生态不如 Rust FFI 灵活，包体积较大

#### 方案C：各平台原生实现 + 共享协议规范

**理由**：
- Android (Kotlin), iOS (Swift), TV (Android/Linux C++)
- 各平台最佳体验

**劣势**：工作量大，维护成本高，协议实现需要多份

### 第一阶段实现路线图

#### Phase 1.1 — DLNA 媒体投屏（MVP）
- 实现 SSDP 设备发现
- 实现 DLNA DMC（控制点）— 手机端
- 实现 DLNA DMR（媒体渲染器）— 电视端
- 支持 SetAVTransportURI / Play / Pause / Stop / Seek
- 支持 GENA 事件订阅（播放状态同步）
- **效果**: Android/iOS 手机可将视频URL投屏到运行 OpenCast 的电视端

#### Phase 1.2 — AirPlay 接收
- 实现 mDNS 服务广播
- 实现 AirPlay 视频投屏接收（HTTP POST /play）
- 实现 AirPlay 音频接收（RAOP 基础功能）
- **效果**: iPhone/iPad 可投屏到 OpenCast 电视端

#### Phase 1.3 — Google Cast 接收
- 实现 mDNS `_googlecast._tcp` 服务广播
- 实现 CASTV2 协议（protobuf + TLS）
- 实现 Default Media Receiver
- **效果**: Android 手机可通过 Cast 投屏到 OpenCast 电视端

#### Phase 1.4 — 统一体验
- 统一播放器引擎（基于 GStreamer 或平台原生）
- 统一控制 UI（手机端遥控器界面）
- 多协议同时监听，自动适配
- **效果**: 达到乐播投屏第一阶段效果

### 鸿蒙兼容策略

由于鸿蒙支持标准 DLNA，我们的 DLNA 实现天然兼容鸿蒙设备。同时：
- 鸿蒙 APP 可通过 NAPI 调用 Rust 编译的 Native 库
- 鸿蒙 4.0+ 基于 OpenHarmony，支持 NDK 开发
- 后续可研究 Cast+ 协议的兼容性

### 与乐播投屏的功能对齐

| 功能 | 乐播投屏 | OpenCast Phase 1 |
|------|---------|------------------|
| DLNA 媒体投屏 | ✅ | ✅ Phase 1.1 |
| AirPlay 投屏 | ✅ | ✅ Phase 1.2 |
| Chromecast 投屏 | ✅ | ✅ Phase 1.3 |
| 手机投屏后可自由使用 | ✅ | ✅ |
| 播放控制（暂停/快进/音量）| ✅ | ✅ |
| 屏幕镜像（回退模式） | ✅ | ❌ Phase 2 |
| 多设备同时投屏 | ✅ | ❌ Phase 2 |
| 投屏码配对 | ✅ | ❌ Phase 2 |

---

## 六、风险与挑战

1. **AirPlay 加密**: AirPlay 2 使用 FairPlay SAPv2.5 加密，开源实现需要处理此问题。UxPlay 已有成熟方案可参考。

2. **DRM 内容**: 部分视频平台的内容有 DRM 保护，URL 无法直接推送。这是所有 DLNA 方案的共同限制。

3. **局域网要求**: 所有投屏协议都要求发送端和接收端在同一局域网。跨网络投屏需要额外考虑。

4. **鸿蒙 Cast+**: 华为私有协议，无法直接兼容，但标准 DLNA 可覆盖基本需求。

5. **各平台适配**: Android、iOS、TV 系统差异大，需要处理后台运行、网络权限、多播等平台特性。

---

## 七、参考资源

### 关键开源项目
- UxPlay: https://github.com/FDH2/UxPlay
- shairport-sync: https://github.com/mikebrady/shairport-sync
- gmrender-resurrect: https://github.com/hzeller/gmrender-resurrect
- Macast: https://github.com/xfangfang/Macast
- PyChromecast: https://github.com/home-assistant-libs/pychromecast
- MiracleCast: https://github.com/albfan/miraclecast
- airplay_dlna_googlecast: https://github.com/alexfansz/airplay_dlna_googlecast
- crab-dlna: https://github.com/gabrielmagno/crab-dlna
- UPnPCast: https://github.com/yinnho/UPnPCast
- pupnp: https://github.com/pupnp/pupnp

### 协议规范
- AirPlay 逆向规范: https://nto.github.io/AirPlay.html
- UPnP 规范: https://openconnectivity.org/developer/specifications/upnp-resources/upnp/
- CASTV2 protobuf: Chromium 源码中的 cast_channel.proto
- DLNA: https://www.dlna.org/
