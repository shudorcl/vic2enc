# Vic2Encoding · 维多利亚2 编码转换工具

把《维多利亚2》(Victoria II) 的本地化 **CSV** 与事件/决议 **脚本**，在**游戏格式**（GBK 伪 Latin-1）与**可读 UTF-8** 之间双向转换，方便汉化的翻译、校对与回填。附带一个把「PA 汉化」式 mod 迁移到现代 `assets/localisation/zh-CN` 布局的一键工具。

- 🖥 **桌面端**：Tauri v2 + Rust 图形界面，单文件免安装。
- ⌨️ **命令行**：纯 Rust CLI，给跑不动 WebView2 的老平台（Win7/旧机、32 位）兜底。
- 🖱 **一键脚本**：随 CLI 附带拖拽即用的 `.bat`，不懂命令行也能用。
- 🦀 核心逻辑参考 [`ParadoxLocalisationAssistant`](https://github.com/) 在 Rust 中按**字节级**重写，保证无损往返。

> ⚠️ 本工具只负责**编码转换**。中文能否在游戏里正常显示，仍取决于你安装了对应的**中文字体补丁**。

---

## 原理

维多利亚2 把本地化存为 `;` 分隔的 CSV，并按 **Windows-1252 (cp1252)** 读取，原生不支持中文。汉化的通行做法是把中文用 **GBK (cp936)** 编码成字节，让游戏把这些字节当作 Latin-1 字符显示，再配合**自定义字体**把字节序列映射回中文字形 —— 即所谓 “dummy Latin-1 / 伪拉丁” 技巧。

本工具直接在**字节层**实现这套往返：

- `£`(0xA3) / `¤`(0xA4) / `§`(0xA7) 等 Paradox 控制字节单独处理；
- 其余高字节按 GBK 双字节解码，能用 GBK 表示的字符（含中文、间隔号 `·` 等标点）一律走 GBK，避免单字节 Latin-1 抢走下一个字节、破坏后续内容；
- `;`、`#`、换行等结构字符都 `< 0x40`，永不会被吞进 CJK 字节对，所以整文件转码不破坏 CSV 列结构。

---

## 下载与使用（不会命令行的玩家看这里）

到 **Releases** 页面下载：

| 文件 | 说明 |
|------|------|
| `Vic2-encoding-portable.exe` | **图形界面 · 单文件免安装**，双击即用。仅需系统自带 **WebView2 运行时**（Win10 1803+/Win11 默认就有；Win7/8 装一次微软 WebView2 即可）。 |
| `Vic2-encoding-setup.exe` | 图形界面 · 安装版（带开始菜单/卸载项，内核同上）。 |
| `vic2enc-win64.zip` | 命令行 + 一键 `.bat`，64 位 Windows（绝大多数电脑）。 |
| `vic2enc-win32.zip` | 命令行 + 一键 `.bat`，32 位 / 很老的系统（Win7/XP；GUI 跑不动用这个）。 |

zip 解压后，把 `vic2enc.exe`、`一键解码.bat`、`一键编码.bat`、`使用说明.txt` 放同一文件夹：

- **解码**（游戏格式 → 可读中文）：把文件或 `localisation` 文件夹**拖到 `一键解码.bat` 图标上**。
- **编码**（翻译好的中文 → 游戏格式）：把文件/文件夹**拖到 `一键编码.bat` 上**。
- 不会拖拽就双击 `.bat`，按提示把路径拖进黑窗口再回车。

结果生成在原路径旁的 `*_decoded` / `*_encoded`，原文件不动。

---

## 功能

### 1. 编码转换

支持两类文件，走**同一条**伪 Latin-1 管线（整文件字节转码，脚本语法/标识符是 ASCII，原样通过）：

| 类型 | 说明 | 批量目录范围 |
|------|------|------|
| `*.csv` | 本地化文件（游戏按 cp1252 读取） | 所有文件夹 |
| `*.txt` | 事件/决议脚本（`change_region_name` 等动态文本的引号字符串、`§` 颜色码、`$变量$`） | 默认仅 `events` / `decisions`；可关闭或扩到全部 |

**典型汉化流程**：译者用 **UTF-8 中文**编辑英文源 → `encode` 成 `GBK 游戏格式`（`§`→单字节 `A7`，中文→GBK 字节对）→ 游戏按 GBK 读取显示。

> `decode`（游戏→可读）只能用在**已经是 GBK 游戏格式**的文件上；不要拿它去 decode UTF-8 源文件（会把 UTF-8 字节误当 GBK 解析）。每个文件需统一一种编码。

#### Safe-token（默认**关闭**）

可选开关：把 `$变量$`、`§颜色`、`£图标£`、`¤` 改写成惰性 ASCII 记号 `<VAR-…>` / `<A7-x>` / `<A3-…>` / `<A4>`。

- **默认关**：可读文本保留**原始控制码**（`$VAR$`、`§`…），encode/decode 两头对称，是大多数 mod 工作流想要的。
- **开启**：仅当你要把可读文本丢给**外部翻译/表格工具**、担心这些控制码被吃掉或改坏时再用——代价是可读文件里会带 `<…>` 记号。

### 2. PA 一键迁移（桌面端独立标签页）

参考社区 `pa迁移.py` 的迁移逻辑重写，把「PA 汉化」式 mod（中文硬编码在 localisation CSV 与 events/decisions/common 脚本里）迁移到现代 **`assets/localisation/zh-CN`**（UTF-8）布局：

1. **localisation**：把 `<mod>/localisation/` 内的游戏格式文件按字节级伪 Latin-1 解码为可读 UTF-8，镜像写入 `<mod>/assets/localisation/zh-CN/`。
2. **脚本抽取**：扫描 `events` / `decisions` / `common` 里形如 `key = "中文" # English` 的行，把中文抽成 `English;中文;x` 追加到同名 `.csv`，并把脚本行还原为 `key = "English"`（**保留缩进与换行**）。

相比原脚本的改进：用**字节级解码**替代 `gb2312` 直读（不丢控制码/特殊字符）、保留缩进换行、**迁移前先把受影响的 `localisation`/`events`/`decisions`/`common` 备份到 `<mod>/_pa_backup_<时间戳>/`**。

> 迁移是**就地**改写，务必保留备份直到确认结果无误。

---

## 命令行用法

```sh
# 解码：游戏格式 → 可读 UTF-8（文件或整个文件夹）
vic2enc decode -i localisation -o readable

# 编码：可读 UTF-8 → 游戏格式
vic2enc encode -i readable -o localisation_out
```

选项：

| 选项 | 作用 |
|------|------|
| `--safe-tokens` | 开启 safe-token（`<…>` 记号）；**默认关闭** |
| `--no-txt` | 目录模式下完全跳过 `*.txt` 脚本，只转 `*.csv` |
| `--all-txt` | 目录模式下，`events`/`decisions` 之外的 `*.txt` 也转换 |
| `--codepage gbk` | 目标编码（目前仅 `gbk`） |

`-i` 为文件夹时递归处理：`*.csv` 全部转换，`*.txt` 默认只转 `events`/`decisions` 内的脚本；输出目录保留相对结构。直接指向单个文件时无视上述范围，强制转换。

---

## 桌面端

界面分两个标签页：**编码转换** 与 **PA 一键迁移**。开发运行：

```sh
cargo install tauri-cli --version "^2"     # 首次需要
cargo tauri dev --manifest-path desktop/src-tauri/tauri.conf.json
```

出便携单文件（前端编译时已嵌入 exe）：

```sh
cargo build --release --manifest-path desktop/src-tauri/Cargo.toml
# 产物：desktop/src-tauri/target/release/vic2enc-desktop.exe
```

> ⚠️ **单文件只在 MSVC 工具链下成立**：`webview2-com-sys` 仅在 `target_env = "msvc"` 时静态链接 `WebView2LoaderStatic.lib`；用 **GNU/MinGW** 工具链构建会改为动态依赖 `WebView2Loader.dll`（需与 exe 同目录）。CI 用的就是 MSVC host，发布的 `Vic2-encoding-portable.exe` 是真·单文件。

---

## 从源码构建

```sh
# CLI（纯 Rust，可交叉编译到老 Windows / 32 位）
cargo build --release -p vic2enc-cli      # 产物：target/release/vic2enc

# 全部测试
cargo test --workspace
```

### 项目结构

```
crates/vic2enc-core/   编码核心库（无 UI 依赖，CLI 与桌面端共用）
crates/vic2enc-cli/    命令行 vic2enc
desktop/               Tauri v2 桌面端（src-tauri = Rust，ui = 前端）
scripts/               一键 .bat 启动器 + 使用说明（随 CLI 一起打包发布）
.github/workflows/     CI（构建/测试）与 Release（打 tag 自动发布）
```

---

## 持续集成与发布（CI/CD）

`.github/workflows/` 下两条 GitHub Actions 流水线（Windows runner）：

- **CI** (`ci.yml`)：每次 push / PR 跑 `cargo test --workspace` 并 `cargo check` 桌面 crate；另有一个**建议性**的 fmt + clippy 任务（不阻断）。
- **Release** (`release.yml`)：推送 `v*` tag 时触发，自动产出并上传到 GitHub Release：
  - `Vic2-encoding-portable.exe` —— 单文件免安装 GUI（MSVC 静态链接 WebView2 加载器）。
  - `vic2enc-win64.zip` / `vic2enc-win32.zip` —— CLI + 一键 `.bat` + 使用说明，`+crt-static` 静态链接，裸 Win7+ 免 VC++ 运行库（含 32 位）。
  - `Vic2-encoding-setup.exe` —— Tauri NSIS 安装包（best-effort）。

发布一个版本：

```sh
git tag v0.1.0
git push origin v0.1.0
```

---

## 致谢

- 编码往返核心逻辑参考自 [`ParadoxLocalisationAssistant`](https://github.com/)（C#/WinForms）。
- PA 迁移逻辑参考社区 `pa迁移.py`。

## 许可

[MIT](./LICENSE)
