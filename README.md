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

## 项目结构

```
crates/vic2enc-core/   编码核心库（无 UI 依赖，CLI 与桌面端共用）
crates/vic2enc-cli/    命令行 vic2enc
desktop/               Tauri v2 桌面端（src-tauri = Rust，ui = 前端）
ParadoxLocalisationAssistant/  参考实现（C#，未改动）
```

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
#   --codepage gbk       目标编码（目前仅 gbk）
```

`-i` 为文件夹时递归处理所有 `*.csv`，并在输出目录保留相对目录结构。

典型流程：`decode` 原版 mod → 用任意编辑器翻译/校对 → `encode` 回游戏格式覆盖。

## 桌面端

```sh
cargo install tauri-cli --version "^2"     # 首次需要
cargo tauri dev --manifest-path desktop/src-tauri/tauri.conf.json
# 或直接：  cd desktop/src-tauri && cargo tauri dev
```

界面支持：选择方向（解码/编码）、输入/输出（文件或文件夹）、safe-token 开关、编码选择、
转换以及输入预览（前 4 KB）。

> 桌面端依赖系统的 WebView2 运行时（Win7+）。更老的平台请使用 CLI。
> 如需打包安装包，先用 `cargo tauri icon` 生成图标并在 `tauri.conf.json` 中开启 `bundle.active`。

## 测试

```sh
cargo test --workspace
```

覆盖：中文/控制字符/ASCII 的无损往返、GBK trail byte 落在 cp1252 未定义位、
以及用真实 GB2312 字节向量做的黄金样例（`crates/vic2enc-core/tests/roundtrip.rs`）。
如有真实的 Vic2 汉化 `localisation/*.csv`，可加入测试以扩大覆盖。

## 许可

MIT
