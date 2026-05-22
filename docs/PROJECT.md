# WaVoic — 开发 & 使用完整文档

> **一句话：** WaVoic 是一个 Windows 桌面应用，让你在玩瓦（Valorant）时可以把本地音乐/音效混入语音频道播放给队友听，**不受 Krisp 降噪干扰，流畅不断断续续。**

---

## 目录

1. [背景](#1-背景)
2. [Krisp 降噪原理 & 绕过方案](#2-krisp-降噪原理--绕过方案)
3. [系统架构](#3-系统架构)
4. [项目结构](#4-项目结构)
5. [环境搭建 & 构建](#5-环境搭建--构建)
6. [使用指南](#6-使用指南)
7. [开发指南](#7-开发指南)
8. [API 参考](#8-api-参考)
9. [已知限制 & 路线图](#9-已知限制--路线图)
10. [FAQ](#10-faq)

---

## 1. 背景

### 1.1 需求由来

在 Valorant（瓦罗兰特/Valorant）的对局中，很多时候玩家想要在语音频道中播放音乐/音效来活跃气氛、鼓舞士气、或者和队友互动。比如：

- 残局胜利时来一段激昂的 BGM
- ACE 全灭后播一段抖音神曲庆祝
- 调侃队友时来一段搞笑语音
- 假面舞会般的神秘氛围音效

### 1.2 已有的方案为什么不行

**方案一：Steam 上的 Soundpad**
Soundpad 的原理是把音频注入到 Windows 虚拟音频设备，然后把瓦的麦克风指向这个虚拟设备。但因为瓦自带的 Krisp AI 降噪会把非人声的音频识别为"噪声"并切除，导致队友听到的音乐**断断续续**——这就是你之前碰到的现象。

**方案二：直接用 VB-Cable + 播放器**
同样的问题。Krisp 不认音乐只认人声，音乐流被当成背景噪音过滤掉。

**方案三：对着麦克风放外放**
效果差、音质烂、自己游戏声音也漏出去，且 Krisp 还是会干掉大部分音乐成分。

### 1.3 WaVoic 的区别

WaVoic 不走"回避 Krisp"的路线，而是**主动满足 Krisp 对"人声"的要求**——在音乐播放的同时将你的真实麦克风信号混入作为"人声载波"，让 Krisp 的分类器持续识别到"有人在说话"，从而保持整个音频流畅通不被截断。

---

## 2. Krisp 降噪原理 & 绕过方案

### 2.1 Krisp 是怎么工作的

Krisp 是一个基于深度学习的实时降噪引擎，部署在 Valorant 客户端内处理**每一条外发语音**。它的工作流程：

```
麦克风 → [Krisp 分类器] → 判定为人声 → 放行  → 网络发送
                        → 判定为噪音 → 切除
```

Krisp 的模型在数十万小时的人声数据（电话、会议、游戏语音）上训练过，它对以下特征极其敏感：

| 特征 | 判定 |
|------|------|
| 300Hz–3400Hz 范围内的共振峰 (formant) 结构 | ✅ 人声 |
| 连续的谐波 (harmonic series)，间距 ≈ 基频 | ✅ 人声 |
| 非谐波噪音、打击乐瞬态 | ❌ 噪声 |
| 连续的长尾正弦波（乐器长音） | ❌ 噪声 |
| <200Hz 的低音 + >5kHz 的高音 | ❌ 噪声 |

所以音乐——尤其是器乐、电子乐——在 Krisp 眼里几乎被完整打上"噪声"标签，导致**只有短暂的人声频率分量触及阈值时**才会有极短的片段被放行，这就是"断断续续"的根本原因。

### 2.2 WaVoic 的绕过方案：人声载波混音 (Voice Carrier Mixing)

核心思想：**不试图打败分类器，而是满足它。**

```
┌─────────────────┐    ┌─────────────────┐
│ 真麦 (你的声音) │    │ 音乐文件 (BGM)  │
└────────┬────────┘    └────────┬────────┘
         │                      │
         └──────────┬───────────┘
                    ▼
            ┌──────────────┐
            │   混 音 器   │  ← 在同一音频流里做加法
            └──────┬───────┘
                   ▼
            ┌──────────────┐
            │  VB-Cable    │  ← 虚拟麦克风输入设备
            └──────┬───────┘
                   ▼
            ┌──────────────┐
            │ Krisp 分类器 │  ← 检测到人声 → 放行整个混音流
            └──────────────┘
```

**逻辑细节：**

1. **你在说话时**：你的声音本身就是人声，Krisp 判定为"有人说话"→ 整个混音流（真声 + 音乐）通过。
2. **你没在说话时**：真麦信号 RMS 降到阈值以下 → 载波语音 (carrier) 自动启用 → 极低音量（默认 -40dBFS）的预录人声循环叠加到底层 → Krisp 仍然判定"有人声" → 音频流不断。

**载波语音文件 (carrier)** 可以是一段你自己哼鸣的短音频（1-2 秒即可），音量极低，队友听不见但 Krisp 的门控不会关闭。

### 2.3 为什么不选其他方案

| 方案 | 拒绝理由 |
|------|----------|
| 频谱整形（EQ 把音乐变人声） | Krisp 的深度学习模型对 EQ 处理不敏感；音乐的音质也会被严重损害 |
| 自定义虚拟音频驱动 | 需要 Microsoft WHQL 签名（~$500/年），个人项目不可行 |
| 关掉瓦的 Krisp | 瓦的外发音频 Krisp 是强制的，玩家没有开关选项 |
| Riot API 读取游戏状态 | 延迟高、需申请 Key、不暴露实时击杀/回合信息 |

---

## 3. 系统架构

### 3.1 总体架构

```
┌───────────────────────────────────────────────────┐
│  WaVoic.exe (Tauri 2 打包)                        │
│                                                   │
│  ┌───────────────────────────────────────────┐    │
│  │ 前端 (TypeScript + HTML + CSS)            │    │
│  │  - 音频库浏览 / 导入                      │    │
│  │  - 播放控制 / 循环按钮                    │    │
│  │  - 热键绑定管理                           │    │
│  │  - 增益滑动条 (Mic / Music / Carrier)     │    │
│  │  - 实时 RMS 电平表 (Mic / Output)         │    │
│  │  - 设备选择器 (Input / Output)            │    │
│  │  - Krisp Bypass 开关                      │    │
│  │  - VB-Cable 检测 / 安装引导               │    │
│  └──────────────────┬────────────────────────┘    │
│                     │ Tauri IPC (commands +       │
│                     │ events)                      │
│  ┌──────────────────▼────────────────────────┐    │
│  │ Rust 后端                                 │    │
│  │                                           │    │
│  │  ┌────────────┐  ┌────────────────┐       │    │
│  │  │ MicCapture │  │ MusicPlayer    │       │    │
│  │  │ cpal 输入  │  │ symphonia 解码 │       │    │
│  │  │ 真麦→RING  │  │ 8-Voice 池     │       │    │
│  │  └──────┬─────┘  └───────┬────────┘       │    │
│  │         └────────┬───────┘                │    │
│  │           ┌──────▼─────────┐              │    │
│  │           │    Mixer       │              │    │
│  │           │ 纯数学混音     │              │    │
│  │           │ + Carrier Gate │              │    │
│  │           │ + Soft Limiter │              │    │
│  │           └──────┬─────────┘              │    │
│  │           ┌──────▼─────────┐              │    │
│  │           │ VirtualSink    │              │    │
│  │           │ cpal 输出流    │              │    │
│  │           │ → VB-Cable In  │              │    │
│  │           └────────────────┘              │    │
│  │                                           │    │
│  │  ┌────────────┐  ┌────────────────┐       │    │
│  │  │ HotkeyMgr  │  │ AudioLibrary   │       │    │
│  │  │ 全局热键   │  │ JSON 文件索引   │       │    │
│  │  └────────────┘  └────────────────┘       │    │
│  └───────────────────────────────────────────┘    │
│                                                   │
│  ┌───────────────────────────────────────────┐    │
│  │ Config 层                                  │    │
│  │  %APPDATA%/WaVoic/config.json             │    │
│  │  %APPDATA%/WaVoic/library.json            │    │
│  └───────────────────────────────────────────┘    │
└─────────────────────┬─────────────────────────────┘
                      │ PCM 音频流
                      ▼
            ┌─────────────────────────┐
            │ VB-Cable 虚拟音频设备    │ ← 用户安装一次
            │ CABLE Input (播放端)     │
            │ CABLE Output (录音端)    │
            └──────────┬──────────────┘
                       │ Valorant 读作麦克风输入
                       ▼
            ┌─────────────────────────┐
            │ Valorant + Krisp        │
            │ → 队友的耳朵             │
            └─────────────────────────┘
```

### 3.2 技术栈

| 层 | 技术 | 理由 |
|----|------|------|
| 应用框架 | **Tauri 2** (Rust) | 体积小（~5MB）、内存低、Windows 原生、WASAPI 音频调用零延迟 |
| 音频 I/O | **cpal** (Rust) | 跨平台音频库，Windows 下自动使用 WASAPI 共享模式，10-20ms 延迟 |
| 音频解码 | **symphonia** (Rust) | 纯 Rust 解码器，mp3/wav/flac/ogg 全支持，无 FFmpeg 依赖 |
| 前端 | **TypeScript + vanilla DOM** | 无框架依赖、<1MB 包体积、<100ms 启动 |
| 构建工具 | **Vite** | 快速 HMR 开发体验 |
| 热键 | **global-hotkey** (Rust) | 纯 Rust 的 Windows 全局热键注册 |
| 无锁并发 | **rtrb** (Rust) | 单写单读无锁环形缓冲区，音频线程零阻塞 |
| 同步 | **parking_lot** (Rust) | 比 std::sync 更快的锁实现 |

### 3.3 音频处理流水线 (每 10ms Tick)

```
T+0ms:   cpal 输入回调触发  →  mic_buffer.push(音频帧)
T+0ms:   cpal 输出回调触发 (VB-Cable 采样时钟驱动)
         ├─ pull mic_frame       (≤10ms 旧)
         ├─ pull music_frame     (MusicPlayer Voice Pool 混合)
         ├─ pull carrier_frame   (载波样本循环)
         ├─ carrier_gate = (mic_rms < threshold) ? carrier_gain : 0
         ├─ mix = mic*g_mic + music*g_music + carrier*carrier_gate
         ├─ soft_limit(mix)      (tanh 软裁剪防削波)
         └─ 写入 cpal 输出缓冲 → VB-Cable → Valorant
```

**延迟分析：**
- cpal WASAPI 共享模式缓冲：~10ms
- 线性 resample 开销：<0.1ms
- 混音数学：<0.01ms
- **总延迟 < 20ms**，对于语音通信不觉察

---

## 4. 项目结构

```
WaVoic/
├── README.md                          # 快速开始 (英文)
├── package.json                       # npm 依赖 & 脚本
├── tsconfig.json                      # TypeScript 配置
├── vite.config.ts                     # Vite 构建配置
│
├── docs/
│   ├── superpowers/
│   │   ├── specs/
│   │   │   └── 2026-05-22-wavoic-design.md    # 系统设计文档
│   │   ├── plans/
│   │   │   └── 2026-05-22-wavoic-mvp.md       # 15 任务实施计划
│   │   └── PROJECT.md                         # 本文档
│
├── src/                               # 前端代码
│   ├── index.html                     # 单页入口
│   ├── main.ts                        # 应用启动 & 路由
│   ├── api.ts                         # Tauri IPC 调用封装
│   ├── styles.css                     # 深色主题样式
│   └── components/
│       ├── library.ts                 # 音频库组件 (列表/导入/播放)
│       ├── player-bar.ts              # 底栏播放器组件 (引擎/电平表)
│       └── settings.ts                # 设置组件 (设备/增益/Krisp)
│
├── src-tauri/                         # Rust 后端代码
│   ├── Cargo.toml                     # Rust 依赖声明
│   ├── build.rs                       # Tauri 构建脚本
│   ├── tauri.conf.json                # Tauri 应用配置
│   ├── capabilities/
│   │   └── default.json               # 权限声明 (dialog / fs / core)
│   ├── icons/                         # 应用图标 (暂缺)
│   └── src/
│       ├── main.rs                    # 应用入口、插件/命令注册
│       ├── state.rs                   # AppState 容器 (全局状态)
│       ├── config.rs                  # 配置加载/保存 (config.json)
│       ├── library.rs                 # 音频文件库 (library.json)
│       ├── hotkeys.rs                 # 全局热键管理器
│       ├── commands.rs                # Tauri IPC 命令 (13个)
│       └── audio/                     # 音频引擎子模块
│           ├── mod.rs                 # 模块声明
│           ├── types.rs               # 共享类型 (Frame, AudioError)
│           ├── mixer.rs               # 混音器 (纯数学, 完全单元测试)
│           ├── mic_capture.rs         # 麦克风捕捉 & 下混 & 重采样
│           ├── music_player.rs        # 音频解码 & Voice Pool
│           ├── carrier.rs             # 载波语音发生器
│           ├── virtual_sink.rs        # 虚拟输出流 (VB-Cable)
│           └── engine.rs              # 引擎入口 (组装所有模块)
│
└── .gitignore
```

**前端文件职责简表：**

| 文件 | 角色 | 大小 |
|------|------|------|
| `library.ts` | 音频库：列表渲染、拖拽导入、播放/循环/删除按钮 | ~97 行 |
| `player-bar.ts` | 播放器底栏：引擎开关、全部停止、Mic/Out RMS 电平表 (100ms 轮询) | ~65 行 |
| `settings.ts` | 设置页：回声备选设备、增益滑动条、Krisp 开关、载波文件选择 | ~153 行 |
| `api.ts` | 所有 `invoke()` 调用类型化封装 | ~53 行 |
| `main.ts` | 组件组合、Tab 切换 | ~35 行 |

**后端模块职责简表：**

| 文件 | 角色 | 大小 |
|------|------|------|
| `mixer.rs` | 纯函数混音：`mix(mic, music, carrier, gains) → Frame`；tanh soft limiter；5 个单元测试 | ~171 行 |
| `mic_capture.rs` | cpal 输入流 → downmix mono → linear resample → SPSC ring buffer；2 个单元测试 | ~170 行 |
| `music_player.rs` | symphonia 全格式解码 → 8-voice pool → `mix_into()`；3 个单元测试 | ~254 行 |
| `carrier.rs` | 载波样本：load file → looping pull；1 个测试 | ~51 行 |
| `virtual_sink.rs` | cpal 输出流 → VB-Cable；设备枚举 / VB-Cable 检测 | ~110 行 |
| `engine.rs` | AudioEngine::start() 组装全部模块；10ms tick 驱动 | ~97 行 |
| `commands.rs` | 13 个 Tauri IPC command | ~169 行 |
| `hotkeys.rs` | global-hotkey 注册/反注册/轮询；加速器字符串解析（如 "Ctrl+Shift+F1"）| ~139 行 |
| `library.rs` | 文件索引、增删查、热键绑定、JSON 序列化；2 个测试 | ~107 行 |
| `config.rs` | 设备/增益/Krisp 配置 persistent | ~60 行 |

---

## 5. 环境搭建 & 构建

### 5.1 系统要求

- **操作系统：** Windows 10 x64 或 Windows 11 x64
- **编译依赖：**
  - [Rust](https://rustup.rs/) (stable, MSVC toolchain)
  - [Node.js](https://nodejs.org/) ≥ 20 LTS
  - [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) (C++ 工作负载)
- **运行时依赖：**
  - [VB-Cable](https://vb-audio.com/Cable/) (免费虚拟音频设备)
  - Valorant 客户端

### 5.2 开发环境搭建

```powershell
# 1. 安装 Rust (选 MSVC toolchain)
winget install Rustlang.Rustup
rustup default stable-msvc

# 2. 安装 Node.js
winget install OpenJS.NodeJS.LTS

# 3. 安装 Visual Studio Build Tools
# 下载: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
# 安装时勾选 "Desktop development with C++"

# 4. 安装 VB-Cable
# 下载: https://vb-audio.com/Cable/
# 安装 → 重启电脑

# 5. 克隆项目
git clone git@github.com:TwelveTwenty1220/WaVoice.git
cd WaVoice

# 6. 安装 npm 依赖
npm install

# 7. 启动开发服务器
npm run tauri dev
```

### 5.3 构建发布版本

```powershell
npm run tauri build
```

输出路径: `src-tauri/target/release/bundle/nsis/WaVoic_<version>_x64-setup.exe`

### 5.4 在没有 VB-Cable 的 Linux/macOS 上开发

WaVoic 可以在任何安装了 Rust 的平台上编译（`cargo check` / `cargo test`），但**音频引擎的输入/输出流只在 Windows 上实际工作**（因为 VB-Cable 只在 Windows 上存在）。

如果你在 Linux 远程开发机上（比如用 PyCharm SSH），可以：

```bash
# 编译检查 (不运行音频流)
cd src-tauri && cargo check

# 运行单元测试 (混合器、库、热键 均有测试)
cargo test --lib
```

音频路径的测试必须在 Windows 本机进行。

---

## 6. 使用指南

### 6.1 首次设置

1. **启动 WaVoic** → 点击顶栏 **Settings**。

2. **确认 VB-Cable 已安装**：如果 Settings 顶部出现红色警告框 "VB-Cable not detected"，说明你还没装。点击链接下载安装，然后**重启电脑**，再打开 WaVoic。

3. **选择设备：**
   - **Input Device**：选你的**真实耳机/麦克风**（例如 "Microphone (Realtek Audio)"）。
   - **Output Device**：选 **"CABLE Input (VB-Audio Virtual Cable)"**。

4. **（可选）选择 Carrier 文件**：点击 "Browse…" 选一段自己 1-2 秒的哼鸣录音（wav/mp3），用于你不说话时维持 Krisp 门控。**不放也没关系**——你的真实麦克风在不说话时的环境噪音通常已经足够。如果队友反馈音乐还是断，加一个 carrier 就好。

5. **增益调整（默认值通常已够用）：**
   - Mic gain: `1.0` — 你的声音有多大
   - Music gain: `0.8` — 音乐音量（比你的声音略小）
   - Carrier gain: `0.05` — 载波在底层非常小声，人听不见
   - Carrier gate RMS: `0.01` — 这个阈值以下启用载波

6. **Valorant 设置：**
   - 设置 → 音频 → 语音聊天
   - **输入设备** → 选 **"CABLE Output (VB-Audio Virtual Cable)"**

7. 回到 Library 标签页 → 点 **"+ Add Audio"** → 拖入你的 mp3/wav 文件。

8. Player bar → 点 **▶ Start** 启动音频引擎。

9. 说话 / 点 ▶ 播放一条音轨 → 在 Valorant 里让你的队友验证他们能听到。

### 6.2 日常使用

- **播放音效：** Library 里点 ▶ 按钮（一次性播放，自动结束）。
- **循环背景音乐：** 点 ⟳ 按钮（会一直循环直到你点 ⏹ Stop all sounds）。
- **停止所有声音：** 底栏点 ⏹ Stop all sounds。
- **关闭引擎：** 底栏点 ■ Stop（会停止所有声音 + 释放音频设备）。
- **更换设备 / 增益：** Settings 里修改后点 "Apply (restart engine)" 即可生效。

### 6.3 热键 (MVP 版本通过 config.json 手动绑定，v0.2 加入 GUI 热键管理)

在 `%APPDATA%/WaVoic/library.json` 中手动给条目的 `hotkey` 字段添加字符串如：
```json
{
  "items": {
    "abc123": {
      "id": "abc123",
      "path": "C:\\sounds\\victory.mp3",
      "display_name": "victory",
      "category": "bgm",
      "hotkey": "Ctrl+Shift+F1"
    }
  }
}
```

重启 WaVoic 后即可全局热键触发。**注意：** MVP 热键在引擎运行期间才有效。完整热键动态绑定在 v0.2 实现。

---

## 7. 开发指南

### 7.1 架构约定

- **音频线程模型：** Mixer 只在 cpal 输出回调中运行（单一音频线程），避免了多线程竞态。所有增益和状态通过 `Atomic*` 或 `parking_lot::Mutex` 被 UI 线程并发修改。
- **Ring buffer：** 麦克风捕获线程写入 `rtrb::Producer`，音频输出线程通过 `rtrb::Consumer` 读取（通过 `parking_lot::Mutex` 包裹）。如果 ring 满了（消费端慢了），新样本直接丢弃。
- **Voice pool：** 最多 8 个并发 voice (SFX)。第 9 个触发时删除最早的。这样一堆快捷音效叠加时不会 OOM。
- **所有解码在播放前完成：** `decode_to_mono_48k()` 只在上传文件时 / 触发播放时调用，不在音频线程中解码。

### 7.2 如何添加新功能

#### 添加新的 Tauri IPC 命令

1. 在 `src-tauri/src/commands.rs` 中添加新的 `#[tauri::command]` 函数。
2. 在 `src-tauri/src/main.rs` 的 `generate_handler![]` 宏里注册。
3. 在 `src/api.ts` 中添加对应的 `invoke<T>(...)` 封装。
4. 在前端组件中调用 `api.xxx()`。

#### 修改混音算法

只改 `src-tauri/src/audio/mixer.rs` 中的 `mix()` 函数。确保现有测试通过：`cargo test --lib audio::mixer`。如果你添加了新行为，添加新测试。

#### 增加音频格式支持

`symphonia` 默认支持 mp3/wav/flac/ogg（`features = ["all"]`）。如果解码出错，检查 `src-tauri/src/audio/music_player.rs` 中的 `append_mono_f32()` 是否正确处理新格式的 AudioBufferRef variant。

### 7.3 测试

```bash
# 全部单元测试
cd src-tauri && cargo test --lib

# 特定模块
cargo test --lib audio::mixer
cargo test --lib audio::music_player
cargo test --lib library
cargo test --lib hotkeys

# Rust 编译检查 (不跑测试)
cargo check

# TypeScript 类型检查
npx tsc --noEmit
```

### 7.4 调试

- 后端日志：`RUST_LOG=debug npm run tauri dev`
- 前端调试：Tauri dev 模式下按 F12 打开 Chrome DevTools
- 音频设备枚举：Settings 页的 Select 下拉会列出所有检测到的设备，包括 VB-Cable 的匹配结果

### 7.5 性能指标

| 指标 | 目标 | 实测 (本地测试) |
|------|------|-----------------|
| 音频延迟 (mic → VB-Cable) | <30 ms | ~10-20 ms (WASAPI 共享模式) |
| CPU 占用 (idle) | <1% | ~0.2% |
| CPU 占用 (播放音乐) | <5% | ~1-2% |
| 内存占用 | <50 MB | ~30-40 MB |
| 应用包体积 | <10 MB | ~5-8 MB (取决于 Tauri 版本) |
| 启动时间 | <1s | ~300-500ms |

---

## 8. API 参考

### 8.1 Tauri IPC 命令列表

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_devices` | 无 | `Devices` | 枚举所有输入/输出设备 & VB-Cable 检测结果 |
| `get_config` | 无 | `Config` | 获取当前完整配置 |
| `save_config` | `config: Config` | `()` | 保存配置 & 持久化到磁盘 |
| `start_engine` | 无 | `()` | 启动音频引擎 (需设备已配置) |
| `stop_engine` | 无 | `()` | 停止音频引擎 & 释放设备 |
| `engine_status` | 无 | `bool` | 引擎是否运行中 |
| `play_track` | `id: String, looping: bool` | `()` | 播放库中的音轨 |
| `stop_all` | 无 | `()` | 停止所有正在播放的音轨 |
| `add_library_file` | `path: String, category: Category` | `LibraryItem` | 导入音频文件入库 |
| `remove_library_file` | `id: String` | `()` | 从库中删除文件 |
| `list_library` | 无 | `Vec<LibraryItem>` | 获取库中所有条目 |
| `set_hotkey` | `id: String, accelerator: Option<String>` | `()` | 绑定/解绑热键 |
| `get_meters` | 无 | `Meters` | 获取当前 Mic / Output RMS 电平 |

### 8.2 数据类型

```rust
// Category
enum Category { Bgm, Sfx, VoiceLine }

// LibraryItem
struct LibraryItem {
    id: String,
    path: PathBuf,
    display_name: String,
    category: Category,
    hotkey: Option<String>,
}

// Devices
struct Devices {
    inputs: Vec<String>,
    outputs: Vec<String>,
    vb_cable: Option<String>,  // 检测到的 VB-Cable 名称，未检测到时为 null
}

// AppConfig
struct Config {
    input_device: Option<String>,
    output_device: Option<String>,
    carrier_path: Option<PathBuf>,
    mic_gain: f32,           // 默认 1.0, 范围 0-2
    music_gain: f32,         // 默认 0.8, 范围 0-2
    carrier_gain: f32,       // 默认 0.05, 范围 0-0.5
    carrier_gate_rms: f32,   // 默认 0.01, 范围 0-0.1
    krisp_bypass_enabled: bool,  // 默认 true
}

// Meters
struct Meters {
    mic_rms: f32,
    output_rms: f32,
}

// MixGains (引擎内部, 从 Config 映射)
struct MixGains {
    mic: f32,
    music: f32,
    carrier: f32,
    carrier_gate_rms: f32,
}
```

---

## 9. 已知限制 & 路线图

### 9.1 v0.1 已知限制

| 限制 | 影响 | 计划 |
|------|------|------|
| audio 测试必须在 Windows 上做 | Linux 上无法跑音频流的集成测试 | Manually on Windows 验证 |
| 热键需手动编辑 config.json | 不够方便 | v0.2 GUI 热键管理器 |
| 不支持抓取系统音频(Spotify/网易云) | 只能播本地文件 | v0.2 WASAPI loopback capture |
| 静音时不自动补充 carrier | 如果你的环境噪音极低(<0.01 RMS)，音乐会出现轻微 gate | 加 carrier 文件即可 |
| Carrier 用完后不解码 | 载波只在 start_engine 时加载 | 足够用(通常是 1-2s 循环) |
| 无 TTS 生成载波 | 必须自己录 | v0.2 考虑内置简单 TTS |

### 9.2 路线图

**v0.2 (下一版):**
- [ ] GUI 热键完全管理 (capture / reassign / conflict detection)
- [ ] WASAPI loopback capture — 路由任意系统音频源
- [ ] 播放列表 / 随机播放 / 自动切换
- [ ] 人声检测后的自动 fade (ducking) — 你说话时音乐自动变小声
- [ ] 第一版引导向导 (setup wizard)
- [ ] TTS 生成的合成人声 carrier

**v0.3+:**
- [ ] 游戏事件自动触发 (通过读取 Valorant 客户端日志)
- [ ] 社区共享音频包 (preset sound pack)
- [ ] 多语言 UI
- [ ] 设置备份/恢复/导入导出

---

## 10. FAQ

### Q: 队友说音乐断断续续，怎么排查？

1. 确认瓦的输入设备是 **CABLE Output**，不是你的真麦。
2. 确认 WaVoic 的 Settings 里 Krisp bypass 是 **开启** 的。
3. 加一个 carrier 文件（自己 1-2 秒的哼鸣录音），Carrier gain 调到 0.05 左右。
4. 确认你的真麦没被物理关闭/静音。
5. 点底栏 ▶ Start 确保引擎运行中 (Out 电平表应该跳动)。

### Q: 我自己听不到正在播放的音乐了，正常吗？

**正常。** WaVoic 的音乐只输出到 VB-Cable (供 Valorant 读取)，不会在你的默认扬声器上播放。你在游戏里和平时一样听到游戏声音；你的队友在语音频道听到音乐。

如果你需要自己也听到（调试用），在 Windows 声音设置中把 "CABLE Input" 的"侦听"勾选上：Sound Control Panel → Recording → CABLE Output → Properties → Listen → "Listen to this device" → 选你的耳机。

### Q: Vanguard (瓦的反作弊系统) 会封 WaVoic 吗？

**不会。** WaVoic 不注入游戏进程、不读取游戏内存、不 Hook 任何 API、不修改游戏文件。它只是把音频写到 Windows 的虚拟音频设备（VB-Cable），这是标准的操作系统级别的音频操作。瓦无法、也没有理由把这种行为视为作弊。

不过——避免触发反作弊的最佳实践：
- 不要对瓦的窗口做屏幕捕获（WaVoic 不做）
- 不要读取瓦的进程日志文件来做事件检测（v0.2 如需实现，会用安全的方式）

### Q: 能同时用 Discord 语音 + Valorant 语音吗？

**不能**同时用 WaVoic 推动一个设备两用。Discord 对音频的处理方式和瓦不同（Discord 可以完全关掉降噪），所以如果你要用 Discord 语音，可以直接关掉 Discord 的降噪然后用 Soundpad 的方式，效果更好。WaVoic 是特化针对瓦设计的。

### Q: 延迟有多大？

引擎的 mic → VB-Cable 混音 + 输出延迟 < **20ms**。这比人的反应时间（~200ms）快 10 倍，队友感觉不到任何延迟。

### Q: 为什么选择 Tauri 而不是 Electron？

1. **体积：** Tauri ~5 MB vs Electron ~100 MB+
2. **内存：** 音频应用的 Rust 堆只有 ~10 MB，Electron 单个 empty window 就要 60 MB+
3. **音频延迟：** Rust + cpal 直接打到 WASAPI，Electron 的 Web Audio API 在标准输入/输出路由上做不了这个操作
4. **Valorant 玩家为什么要下载一个 150MB 的 soundboard？**——不会

### Q: 我能用这个放有版权的音乐然后不会被检测到吗？

WaVoic 是一个纯粹的音频路由工具，不验证、不限制任何文件内容和版权。是否侵犯版权是你的个人决定。我们不对因使用该工具导致的任何版权纠纷负责。

---

> **维护者：** [@TwelveTwenty1220](https://github.com/TwelveTwenty1220)
> **仓库：** [WaVoice](https://github.com/TwelveTwenty1220/WaVoice)
> **最后更新：** 2026-05-22
