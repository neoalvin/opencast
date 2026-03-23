# OpenCast

开源跨平台媒体投屏方案，对标乐播投屏。

**核心理念：只投内容，不投屏幕** — 将视频/音频推送到电视播放，手机依然自由可用。

## 特性

- **媒体投屏（非屏幕镜像）** — 电视直接拉取视频流播放，手机仅作遥控器
- **跨平台兼容** — 支持 iOS (AirPlay)、Android、鸿蒙 (DLNA) 设备投屏
- **零配置发现** — 自动发现同一局域网内的投屏设备
- **完整播放控制** — 播放/暂停/停止/快进/音量调节
- **无广告、无付费、无隐私采集** — 纯本地局域网，不需要云服务

## 架构

```
手机/PC (发送端)                     电视/盒子 (接收端)
┌──────────────┐                    ┌────────────────────┐
│  SSDP/mDNS   │──── 设备发现 ────→│  SSDP + Bonjour    │
│  设备发现     │                    │                    │
│              │                    │  DLNA DMR          │
│  DLNA DMC    │── SOAP 控制指令 ──→│  (HTTP + SOAP)     │
│  (控制端)     │                    │        │           │
│              │                    │  AirPlay Receiver  │
│  iPhone/iPad │── AirPlay HTTP ──→│  (HTTP + mDNS)     │
│              │                    │        │           │
└──────────────┘                    │        ▼           │
                                    │  媒体播放器 (mpv)   │
内容服务器 ─── HTTP 视频流 ────────→│        ▲           │
                                    │        │           │
                                    │  电视直接下载播放    │
                                    └────────────────────┘
```

## 项目结构

```
crates/
├── opencast-core/       # 核心类型与 trait (Device, MediaInfo, RendererCallback)
├── opencast-discovery/  # 设备发现 (SSDP M-SEARCH / NOTIFY)
├── opencast-dlna/       # DLNA/UPnP 协议 (DMC 控制端 + DMR 渲染端)
├── opencast-airplay/    # AirPlay 媒体接收 (HTTP + mDNS 广播)
├── opencast-player/     # 统一播放引擎 (mpv 后端)
├── opencast-server/     # 电视端接收应用 (DLNA + AirPlay)
└── opencast-cli/        # 命令行投屏工具
```

## 快速开始

### 编译

```bash
# 安装 Rust (如未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 编译
cargo build --release
```

### 电视端 — 启动接收器

```bash
# 启动接收器（同时支持 DLNA + AirPlay），其他设备可以发现并投屏到此设备
cargo run --release --bin opencast-server -- --name "客厅电视"

# 自定义端口
cargo run --release --bin opencast-server -- --name "客厅电视" --port 9000 --airplay-port 7001
```

### 手机/PC端 — 命令行投屏

```bash
# 发现局域网内的 DLNA 设备
cargo run --release --bin opencast-cli -- discover

# 投屏视频到电视
cargo run --release --bin opencast-cli -- cast "http://example.com/video.mp4" --device "客厅电视"

# 投屏时指定标题和格式
cargo run --release --bin opencast-cli -- cast "http://example.com/movie.mkv" \
  --device "客厅电视" \
  --title "我的电影" \
  --mime "video/x-matroska"
```

### 播放控制

```bash
# 暂停
cargo run --release --bin opencast-cli -- control pause --device "客厅电视"

# 继续播放
cargo run --release --bin opencast-cli -- control play --device "客厅电视"

# 快进到 2 分钟处
cargo run --release --bin opencast-cli -- control seek --device "客厅电视" --position 120

# 调节音量 (0-100)
cargo run --release --bin opencast-cli -- control volume --device "客厅电视" --volume 80

# 查看播放状态
cargo run --release --bin opencast-cli -- control status --device "客厅电视"

# 停止播放
cargo run --release --bin opencast-cli -- control stop --device "客厅电视"
```

## 支持的协议

| 协议 | 状态 | 说明 |
|------|------|------|
| DLNA/UPnP | ✅ 已实现 | 媒体投屏核心协议，兼容 Android/鸿蒙/智能电视 |
| AirPlay | ✅ 已实现 | 苹果设备投屏支持 (iPhone/iPad/Mac) |
| Miracast | 📋 远期 | 屏幕镜像（回退模式） |

## 支持的媒体格式

视频: MP4, MKV, WebM, AVI, M3U8 (HLS)
音频: MP3, FLAC, WAV, AAC, M4A
图片: JPEG, PNG

## 路线图

- [x] **Phase 1.1** — DLNA 媒体投屏 (DMC + DMR + SSDP)
- [x] **Phase 1.2** — AirPlay 接收端 (HTTP + mDNS)
- [ ] **Phase 1.3** — Android TV 接收端 APP
- [ ] **Phase 1.4** — 手机端 APP (iOS + Android + 鸿蒙)
- [ ] **Phase 2.0** — 屏幕镜像模式 (回退方案)
- [ ] **Phase 3.0** — 投屏码配对 / 多设备同时投屏

## 技术栈

- **语言:** Rust
- **异步运行时:** Tokio
- **HTTP:** Hyper
- **协议:** SSDP (UDP 多播) + SOAP over HTTP + UPnP/DLNA + AirPlay (HTTP + mDNS)
- **播放器:** mpv (通过 JSON IPC 控制)

## 依赖

运行接收端需要安装 [mpv](https://mpv.io/)：

```bash
# macOS
brew install mpv

# Ubuntu/Debian
sudo apt install mpv

# Arch Linux
sudo pacman -S mpv
```

## 许可证

MIT OR Apache-2.0
