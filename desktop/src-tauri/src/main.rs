// Hide the extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use vic2enc_core::{convert_bytes, convert_dir, convert_file, Codepage, ConvertOptions, Direction};

/// Parameters shared by the `convert` and `preview` commands, mirroring the
/// fields of the UI form. `direction`/`codepage` deserialize from the core
/// enums' serde representation (`"to_unicode"`/`"to_game"`, `"gbk"`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConvertRequest {
    input: String,
    output: String,
    direction: Direction,
    codepage: Codepage,
    safe_tokens: bool,
}

impl ConvertRequest {
    fn options(&self) -> ConvertOptions {
        ConvertOptions {
            codepage: self.codepage,
            safe_tokens: self.safe_tokens,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConvertResponse {
    files: usize,
    bytes_in: u64,
    bytes_out: u64,
    message: String,
}

/// Convert a file or directory and write the result to `output`.
#[tauri::command]
fn convert(req: ConvertRequest) -> Result<ConvertResponse, String> {
    let input = PathBuf::from(&req.input);
    let output = PathBuf::from(&req.output);
    if req.input.trim().is_empty() {
        return Err("请先选择输入路径".into());
    }
    if req.output.trim().is_empty() {
        return Err("请先选择输出路径".into());
    }
    if !input.exists() {
        return Err(format!("输入路径不存在: {}", input.display()));
    }
    let opts = req.options();
    let stats = if input.is_dir() {
        convert_dir(&input, &output, req.direction, &opts)
    } else {
        convert_file(&input, &output, req.direction, &opts)
    }
    .map_err(|e| e.to_string())?;
    Ok(ConvertResponse {
        files: stats.files,
        bytes_in: stats.bytes_in,
        bytes_out: stats.bytes_out,
        message: stats.to_string(),
    })
}

/// Convert the first `max_bytes` of a single file and return a (lossy) string
/// preview. Most useful for decode; for encode the bytes are game format and
/// will look like mojibake, which is expected.
#[tauri::command]
fn preview(
    input: String,
    direction: Direction,
    codepage: Codepage,
    safe_tokens: bool,
    max_bytes: usize,
) -> Result<String, String> {
    let path = PathBuf::from(&input);
    if !path.is_file() {
        return Err("预览仅支持单个文件".into());
    }
    let data = std::fs::read(&path).map_err(|e| e.to_string())?;
    let slice = &data[..data.len().min(max_bytes.max(1))];
    let opts = ConvertOptions {
        codepage,
        safe_tokens,
    };
    let converted = convert_bytes(slice, direction, &opts);
    Ok(String::from_utf8_lossy(&converted).into_owned())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![convert, preview])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
