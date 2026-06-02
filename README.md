# 🔄 SyncLite

<p align="center">
  <img src="https://img.shields.io/badge/version-1.0.0-blue.svg" alt="Version">
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License">
  <img src="https://img.shields.io/badge/rust-1.70+-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg" alt="Platform">
</p>

<p align="center">
  <b>🚀 A lightweight, fast, and intelligent file synchronization and backup tool</b>
</p>

<p align="center">
  <a href="#-english">English</a> |
  <a href="#-简体中文">简体中文</a> |
  <a href="#-繁體中文">繁體中文</a>
</p>

---

## 🇺🇸 English

### 🎉 Introduction

**SyncLite** is a high-performance file synchronization and backup tool built with Rust. It solves the pain points of cross-device file synchronization for developers, offering a lightweight yet powerful solution that "just works".

**Inspiration**: Tired of complex rsync configurations or heavyweight Syncthing setups? SyncLite provides a zero-config, single-binary solution for all your file sync needs.

**Key Differentiators**:
- ⚡ **Blazing Fast**: Rust-powered performance with incremental sync
- 🎯 **Zero Config**: Sensible defaults that work out of the box
- 🔌 **Multi-Backend**: Local, S3, and SFTP support in one tool
- 🧠 **Smart Conflict Resolution**: Automatic handling of file conflicts

### ✨ Features

| Feature | Description |
|---------|-------------|
| 🔄 **Bidirectional Sync** | Keep folders in sync both ways automatically |
| ⬆️ **Push/Pull Modes** | One-way sync options for backup scenarios |
| 📦 **Multiple Backends** | Local filesystem, S3-compatible, SFTP support |
| 👁️ **Real-time Watch** | Auto-sync on file changes with debouncing |
| 💾 **Backup & Restore** | Versioned backups with compression support |
| 🔍 **Incremental Sync** | Only transfer changed blocks using SHA256 |
| ⚡ **Parallel Transfers** | Configurable concurrency for speed |
| 🎨 **Beautiful Output** | Progress bars and colored terminal output |

### 🚀 Quick Start

#### Installation

```bash
# Download pre-built binary from Releases
# Or build from source:
git clone https://github.com/gitstq/SyncLite.git
cd SyncLite
cargo build --release
```

#### Initialize a Sync

```bash
# Initialize sync configuration
synclite init --source ~/Documents --dest /backup/Documents --mode bidirectional

# Or sync to S3
synclite init --source ~/Documents --dest s3://mybucket/documents --mode push

# Or sync via SFTP
synclite init --source ~/Documents --dest sftp://user@host:22/backup --mode push
```

#### Run Sync

```bash
# Dry run to preview changes
synclite sync --dry-run

# Execute sync
synclite sync

# Force sync (ignore conflicts)
synclite sync --force
```

#### Watch Mode

```bash
# Auto-sync when files change
synclite watch --debounce 5
```

#### Backup Commands

```bash
# Create a backup
synclite backup --tag "before-update" --compress

# List backups
synclite list

# Restore from backup
synclite restore "20250602_120000"
```

### 📖 Configuration

SyncLite uses TOML configuration files. Example `synclite.toml`:

```toml
source = "/home/user/Documents"
destination = "s3://mybucket/documents"
mode = "bidirectional"

exclude = [
    ".git",
    "node_modules",
    "*.tmp",
    ".DS_Store"
]

conflict_strategy = "timestamp"
compression = true
concurrency = 4

[backup]
enabled = true
location = ".synclite/backups"
max_backups = 10
auto_cleanup = true

[s3]
region = "us-east-1"
access_key_id = "YOUR_ACCESS_KEY"
secret_access_key = "YOUR_SECRET_KEY"
bucket = "mybucket"
```

### 📦 Building from Source

**Requirements**:
- Rust 1.70 or higher
- OpenSSL development libraries

```bash
# Clone repository
git clone https://github.com/gitstq/SyncLite.git
cd SyncLite

# Build with all features
cargo build --release --all-features

# Build minimal version (local only)
cargo build --release

# Run tests
cargo test --all-features
```

### 💡 Design Philosophy

1. **Simplicity First**: Complex tools discourage usage. SyncLite prioritizes ease of use.
2. **Performance Matters**: Rust ensures memory safety without garbage collection pauses.
3. **Progressive Enhancement**: Start simple, add complexity only when needed.

### 🔮 Roadmap

- [ ] GUI application using Tauri
- [ ] WebDAV backend support
- [ ] Delta sync for large files
- [ ] Bandwidth limiting
- [ ] Sync filters with regex
- [ ] Cloud storage: Google Drive, OneDrive

### 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🇨🇳 简体中文

### 🎉 项目介绍

**SyncLite** 是一款使用 Rust 构建的高性能文件同步与备份工具。它解决了开发者跨设备文件同步的痛点，提供了一个轻量但强大的"开箱即用"解决方案。

**灵感来源**: 厌倦了复杂的 rsync 配置或笨重的 Syncthing 设置？SyncLite 提供了一个零配置、单二进制文件的解决方案，满足您所有的文件同步需求。

**核心差异化优势**:
- ⚡ **极速同步**: Rust 驱动的高性能增量同步
- 🎯 **零配置**: 开箱即用的合理默认设置
- 🔌 **多后端支持**: 一个工具支持本地、S3 和 SFTP
- 🧠 **智能冲突解决**: 自动处理文件冲突

### ✨ 核心特性

| 特性 | 描述 |
|------|------|
| 🔄 **双向同步** | 自动保持文件夹双向同步 |
| ⬆️ **推送/拉取模式** | 适用于备份场景的单向同步选项 |
| 📦 **多后端支持** | 本地文件系统、S3 兼容、SFTP 支持 |
| 👁️ **实时监控** | 文件变更时自动同步，支持防抖 |
| 💾 **备份与恢复** | 支持压缩的版本化备份 |
| 🔍 **增量同步** | 使用 SHA256 仅传输变更块 |
| ⚡ **并行传输** | 可配置的并发数以提升速度 |
| 🎨 **美观输出** | 进度条和彩色终端输出 |

### 🚀 快速开始

#### 安装

```bash
# 从 Releases 下载预编译二进制文件
# 或从源码构建：
git clone https://github.com/gitstq/SyncLite.git
cd SyncLite
cargo build --release
```

#### 初始化同步

```bash
# 初始化同步配置
synclite init --source ~/Documents --dest /backup/Documents --mode bidirectional

# 或同步到 S3
synclite init --source ~/Documents --dest s3://mybucket/documents --mode push

# 或通过 SFTP 同步
synclite init --source ~/Documents --dest sftp://user@host:22/backup --mode push
```

#### 执行同步

```bash
# 预览变更（干运行）
synclite sync --dry-run

# 执行同步
synclite sync

# 强制同步（忽略冲突）
synclite sync --force
```

#### 监控模式

```bash
# 文件变更时自动同步
synclite watch --debounce 5
```

#### 备份命令

```bash
# 创建备份
synclite backup --tag "before-update" --compress

# 列出备份
synclite list

# 从备份恢复
synclite restore "20250602_120000"
```

### 📖 配置说明

SyncLite 使用 TOML 配置文件。示例 `synclite.toml`：

```toml
source = "/home/user/Documents"
destination = "s3://mybucket/documents"
mode = "bidirectional"

exclude = [
    ".git",
    "node_modules",
    "*.tmp",
    ".DS_Store"
]

conflict_strategy = "timestamp"
compression = true
concurrency = 4

[backup]
enabled = true
location = ".synclite/backups"
max_backups = 10
auto_cleanup = true

[s3]
region = "us-east-1"
access_key_id = "YOUR_ACCESS_KEY"
secret_access_key = "YOUR_SECRET_KEY"
bucket = "mybucket"
```

### 📦 从源码构建

**环境要求**:
- Rust 1.70 或更高版本
- OpenSSL 开发库

```bash
# 克隆仓库
git clone https://github.com/gitstq/SyncLite.git
cd SyncLite

# 构建完整功能版本
cargo build --release --all-features

# 构建最小版本（仅本地）
cargo build --release

# 运行测试
cargo test --all-features
```

### 💡 设计理念

1. **简洁优先**: 复杂的工具会阻碍使用。SyncLite 优先考虑易用性。
2. **性能至上**: Rust 确保内存安全，无垃圾回收暂停。
3. **渐进增强**: 从简单开始，仅在需要时添加复杂性。

### 🔮 路线图

- [ ] 使用 Tauri 的 GUI 应用程序
- [ ] WebDAV 后端支持
- [ ] 大文件增量同步
- [ ] 带宽限制
- [ ] 正则表达式同步过滤器
- [ ] 云存储：Google Drive、OneDrive

### 🤝 贡献指南

我们欢迎贡献！请参阅 [CONTRIBUTING.md](CONTRIBUTING.md) 了解指南。

1. Fork 本仓库
2. 创建您的功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交您的更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 打开 Pull Request

### 📄 开源协议

本项目采用 MIT 协议开源 - 详情请参阅 [LICENSE](LICENSE) 文件。



---

## 🇹🇼 繁體中文

### 🎉 項目介紹

**SyncLite** 是一款使用 Rust 構建的高性能檔案同步與備份工具。它解決了開發者跨裝置檔案同步的痛點，提供了一個輕量但強大的「開箱即用」解決方案。

**靈感來源**: 厭倦了複雜的 rsync 配置或笨重的 Syncthing 設定？SyncLite 提供了一個零配置、單二進制檔案的解決方案，滿足您所有的檔案同步需求。

**核心差異化優勢**:
- ⚡ **極速同步**: Rust 驅動的高性能增量同步
- 🎯 **零配置**: 開箱即用的合理預設設定
- 🔌 **多後端支援**: 一個工具支援本地、S3 和 SFTP
- 🧠 **智能衝突解決**: 自動處理檔案衝突

### ✨ 核心特性

| 特性 | 描述 |
|------|------|
| 🔄 **雙向同步** | 自動保持資料夾雙向同步 |
| ⬆️ **推送/拉取模式** | 適用於備份場景的單向同步選項 |
| 📦 **多後端支援** | 本地檔案系統、S3 相容、SFTP 支援 |
| 👁️ **即時監控** | 檔案變更時自動同步，支援防抖 |
| 💾 **備份與恢復** | 支援壓縮的版本化備份 |
| 🔍 **增量同步** | 使用 SHA256 僅傳輸變更塊 |
| ⚡ **並行傳輸** | 可配置的並發數以提升速度 |
| 🎨 **美觀輸出** | 進度條和彩色終端輸出 |

### 🚀 快速開始

#### 安裝

```bash
# 從 Releases 下載預編譯二進制檔案
# 或從原始碼構建：
git clone https://github.com/gitstq/SyncLite.git
cd SyncLite
cargo build --release
```

#### 初始化同步

```bash
# 初始化同步配置
synclite init --source ~/Documents --dest /backup/Documents --mode bidirectional

# 或同步到 S3
synclite init --source ~/Documents --dest s3://mybucket/documents --mode push

# 或通過 SFTP 同步
synclite init --source ~/Documents --dest sftp://user@host:22/backup --mode push
```

#### 執行同步

```bash
# 預覽變更（乾運行）
synclite sync --dry-run

# 執行同步
synclite sync

# 強制同步（忽略衝突）
synclite sync --force
```

#### 監控模式

```bash
# 檔案變更時自動同步
synclite watch --debounce 5
```

#### 備份命令

```bash
# 建立備份
synclite backup --tag "before-update" --compress

# 列出備份
synclite list

# 從備份恢復
synclite restore "20250602_120000"
```

### 📖 配置說明

SyncLite 使用 TOML 配置檔案。範例 `synclite.toml`：

```toml
source = "/home/user/Documents"
destination = "s3://mybucket/documents"
mode = "bidirectional"

exclude = [
    ".git",
    "node_modules",
    "*.tmp",
    ".DS_Store"
]

conflict_strategy = "timestamp"
compression = true
concurrency = 4

[backup]
enabled = true
location = ".synclite/backups"
max_backups = 10
auto_cleanup = true

[s3]
region = "us-east-1"
access_key_id = "YOUR_ACCESS_KEY"
secret_access_key = "YOUR_SECRET_KEY"
bucket = "mybucket"
```

### 📦 從原始碼構建

**環境要求**:
- Rust 1.70 或更高版本
- OpenSSL 開發庫

```bash
# 克隆倉庫
git clone https://github.com/gitstq/SyncLite.git
cd SyncLite

# 構建完整功能版本
cargo build --release --all-features

# 構建最小版本（僅本地）
cargo build --release

# 運行測試
cargo test --all-features
```

### 💡 設計理念

1. **簡潔優先**: 複雜的工具會阻礙使用。SyncLite 優先考慮易用性。
2. **性能至上**: Rust 確保記憶體安全，無垃圾回收暫停。
3. **漸進增強**: 從簡單開始，僅在需要時添加複雜性。

### 🔮 路線圖

- [ ] 使用 Tauri 的 GUI 應用程式
- [ ] WebDAV 後端支援
- [ ] 大檔案增量同步
- [ ] 頻寬限制
- [ ] 正規表示式同步過濾器
- [ ] 雲端儲存：Google Drive、OneDrive

### 🤝 貢獻指南

我們歡迎貢獻！請參閱 [CONTRIBUTING.md](CONTRIBUTING.md) 了解指南。

1. Fork 本倉庫
2. 建立您的功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交您的更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 開啟 Pull Request

### 📄 開源協議

本專案採用 MIT 協議開源 - 詳情請參閱 [LICENSE](LICENSE) 檔案。
