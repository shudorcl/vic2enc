# Vic2Encoding — 维多利亚2 编码转换工具

把维多利亚2（Victoria 2）的本地化 CSV 在**游戏格式**与**可读 UTF-8** 之间双向转换，方便汉化的编辑与回填。

- 桌面端：Tauri v2 + Rust 图形界面。
- 命令行：纯 Rust CLI，给 WebView2 跑不起来的老平台（Win7/旧机、32 位）兜底。
- 核心逻辑参考自 [`ParadoxLocalisationAssistant`](./ParadoxLocalisationAssistant)，在 Rust 中按字节重写，保证无损往返。

## 原理

维多利亚2 把本地化存为 `;` 分隔的 CSV，并按 **Windows-1252 (cp1252)** 读取，原生不支持中文。
汉化的通行做法是把中文用 **GBK (cp936)** 编码成字节，再让游戏把这些字节当作 Latin-1 字符显示，
配合**自定义字体**把字节序列映射回中文字形 —— 这就是所谓的 “dummy Latin-1/ 伪拉丁” 技巧。

本工具做两件事：

1. **编码转换**：游戏里那串看似乱码的字节 ⇄ 真正的 Unicode 中文。
   - 直接在**字节层**实现：`£`(0xA3) / `¤`(0xA4) / `§`(0xA7) 等 Paradox 控制字节单独处理，
     其余高字节尝试按 GBK 两字节解码；`;`、`#`、换行都 `< 0x40`，不会被吞进 CJK 字节对，
     所以整文件转码不会破坏 CSV 列结构。
2. **Safe-token（可选，默认开启）**：把 `$变量$`、`§颜色`、`£图标£`、`¤` 改写成
   `<VAR-…>` / `<A7-x>` / `<A3-…>` / `<A4>` 等惰性 ASCII 记号，避免外部翻译/表格工具破坏它们。

> ⚠️ 本工具只负责编码转换。中文能否在游戏里正常显示，仍取决于你安装了对应的**中文字体补丁**。

### 支持的文件

| 类型 | 说明 | 批量目录范围 |
|------|------|------|
| `*.csv` | 本地化文件（游戏按 cp1252 读取） | 所有文件夹 |
| `*.txt` | 事件/决议脚本（含 `change_region_name` 等动态文本的引号字符串、`§` 颜色码、`$变量$`） | 默认仅 `events` / `decisions`；`--all-txt` 可扩到全部 |

`.txt` 与 `.csv` 走**同一条管线**（GBK 伪Latin1 + safe-token，整文件转码）。脚本语法/标识符是
ASCII，原样通过；只有引号内的中文与控制码受影响。

**典型汉化流程（txt 脚本）**：译者直接用 **UTF-8 中文**编辑英文源脚本（可读、好改）→ 用 `encode`
把 `UTF-8 中文 → GBK 游戏格式`（`§`→单字节 `A7`，中文→GBK 字节对，带音标的拉丁字母→对应 cp1252
单字节）。游戏按 GBK 读取即可正常显示。

> 注意：`decode`（游戏→可读）只能用在**已经是 GBK 游戏格式**的文件上；不要拿它去 decode UTF-8
> 源文件（会把 UTF-8 字节误当 GBK 解析）。每个文件需统一一种编码，不要在单个文件内混合 UTF-8 与 GBK 中文。

## 项目结构

```
crates/vic2enc-core/   编码核心库（无 UI 依赖，CLI 与桌面端共用）
crates/vic2enc-cli/    命令行 vic2enc
desktop/               Tauri v2 桌面端（src-tauri = Rust，ui = 前端）
scripts/               一键 .bat 启动器 + 使用说明（随 CLI 一起打包发布）
.github/workflows/     CI（构建/测试）与 Release（打 tag 自动发布）
ParadoxLocalisationAssistant/  参考实现（C#，未改动）
```

## 下载与使用（不会命令行的玩家看这里）

到本仓库的 **Releases** 页面下载对应压缩包：

- `vic2enc-win64.zip` —— 64 位 Windows（绝大多数电脑）。
- `vic2enc-win32.zip` —— 32 位 / 很老的系统（Win7/XP 等）。
- `Vic2-encoding-setup.exe` —— 图形界面安装版（需 WebView2，老系统用不了就选上面的 zip）。

zip 里有 `vic2enc.exe`、`一键解码.bat`、`一键编码.bat`、`使用说明.txt`，解压到同一文件夹后：

- **解码**（游戏格式 → 可读中文）：把文件或 `localisation` 文件夹**拖到 `一键解码.bat` 图标上**。
- **编码**（翻译好的中文 → 游戏格式）：把文件/文件夹**拖到 `一键编码.bat` 上**。
- 不会拖拽就直接双击 `.bat`，按提示把路径拖进黑窗口再回车。

结果生成在原路径旁的 `*_decoded` / `*_encoded`，原文件不动。详见压缩包内 `使用说明.txt`。

## 命令行用法

```sh
# 构建
cargo build --release -p vic2enc-cli      # 产物：target/release/vic2enc

# 解码：游戏格式 → 可读 UTF-8（文件或整个文件夹）
vic2enc decode -i localisation -o readable

# 编码：可读 UTF-8 → 游戏格式
vic2enc encode -i readable -o localisation_out

# 选项
#   --no-safe-tokens     关闭 safe-token，做原始编码转换
#   --no-txt             目录模式下完全跳过 *.txt 脚本，只转 *.csv
#   --all-txt            目录模式下，events/decisions 之外的 *.txt 也转换
#   --codepage gbk       目标编码（目前仅 gbk）
```

`-i` 为文件夹时递归处理：`*.csv` 全部转换，`*.txt` 默认只转 `events` / `decisions` 内的脚本
（`--no-txt` 完全关闭 txt、`--all-txt` 扩到全部）；输出目录保留相对目录结构。
直接指向单个文件时则无视上述范围，强制转换。

典型流程：`decode` 原版 mod → 用任意编辑器翻译/校对 → `encode` 回游戏格式覆盖；
txt 脚本则译者用 UTF-8 中文编辑后直接 `encode` 成 GBK。

## 桌面端

```sh
cargo install tauri-cli --version "^2"     # 首次需要
cargo tauri dev --manifest-path desktop/src-tauri/tauri.conf.json
# 或直接：  cd desktop/src-tauri && cargo tauri dev
```

界面分两个标签页：

- **编码转换**：选择方向（解码/编码）、输入/输出（文件或文件夹）、safe-token 开关、
  「转换 .txt 脚本」开关与「所有文件夹的 .txt」范围、编码选择、转换与输入预览（前 4 KB）。
- **PA 一键迁移**：把「PA 汉化」式 mod 迁移到现代 `assets/localisation/zh-CN`（UTF-8）布局。

### PA 一键迁移

参考社区 `pa迁移.py` 的迁移逻辑重写，做两件事：

1. **localisation**：把 `<mod>/localisation/` 内的游戏格式文件按字节级伪 Latin-1 解码为可读
   UTF-8，镜像写入 `<mod>/assets/localisation/zh-CN/`。
2. **脚本抽取**：扫描 `events` / `decisions` / `common` 里形如 `key = "中文" # English` 的行——
   把中文抽成 `English;中文;x` 追加到同名 `.csv`（位于 zh-CN/ 下），并把脚本行还原为
   `key = "English"`（**保留缩进与换行**），让游戏改走本地化 key 机制。

相比原脚本的改进：用字节级解码替代 `gb2312` 直读（不丢控制码/特殊字符）、保留缩进与换行、
**迁移前先把受影响的 `localisation`/`events`/`decisions`/`common` 备份到 `<mod>/_pa_backup_<时间戳>/`**。
迁移是**就地**改写，务必保留备份直到确认结果无误。

> 桌面端依赖系统的 WebView2 运行时（Win7+）。更老的平台请使用 CLI。
> 如需打包安装包，先用 `cargo tauri icon` 生成图标并在 `tauri.conf.json` 中开启 `bundle.active`。

## 测试

```sh
cargo test --workspace
```

覆盖：中文/控制字符/ASCII 的无损往返、GBK trail byte 落在 cp1252 未定义位、
以及用真实 GB2312 字节向量做的黄金样例（`crates/vic2enc-core/tests/roundtrip.rs`）。
如有真实的 Vic2 汉化 `localisation/*.csv`，可加入测试以扩大覆盖。

## 持续集成与发布（CI/CD）

`.github/workflows/` 下两条流水线（GitHub Actions，Windows runner）：

- **CI**（`ci.yml`）：每次 push / PR 跑 `cargo test --workspace` 并 `cargo check` 桌面 crate；
  另有一个**建议性**的 fmt + clippy 任务（`continue-on-error`，不阻断，跑过 `cargo fmt`
  后可改成强制）。
- **Release**（`release.yml`）：推送 `v*` tag 时触发，自动产出并上传到 GitHub Release：
  - `vic2enc-win64.zip` / `vic2enc-win32.zip` —— CLI + 一键 `.bat` + 使用说明，
    静态链接 CRT（`+crt-static`），裸 Win7+ 免装 VC++ 运行库即可跑（含 32 位老平台）。
  - `Vic2-encoding-setup.exe` —— Tauri NSIS 安装包（best-effort，失败不影响 CLI 产物）。

发布一个版本：

```sh
# 先把仓库推到 GitHub（仓库当前还没配置 remote）
git remote add origin <your-github-repo-url>
git push -u origin master

# 打 tag 即触发自动发布
git tag v0.1.0
git push origin v0.1.0
```

## 许可

MIT
