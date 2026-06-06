//! `vic2enc-core` — encoding conversion for Victoria 2 localisation CSV files.
//!
//! Victoria 2 stores localisation as `;`-separated CSV read as Windows-1252.
//! Chinese (and other CJK) translations are smuggled in as GBK bytes reinterpreted
//! as Latin-1 ("dummy Latin-1"), rendered by a custom font. This crate converts
//! between that game-format byte representation and readable Unicode, optionally
//! protecting Paradox control sequences with inert `<...>` "safe tokens".
//!
//! The conversion is purely byte/string level and free of any UI framework, so it
//! is shared by both the CLI (`vic2enc-cli`) and the Tauri desktop app.

mod convert;
mod dummy_latin1;
mod pa_migrate;
mod safe_token;

pub use convert::{convert_bytes, convert_dir, convert_file, BatchScope, Stats};
pub use dummy_latin1::{dummy_latin1_to_unicode, unicode_to_dummy_latin1};
pub use pa_migrate::{pa_migrate, PaMigrateOptions, PaMigrateReport, DEFAULT_TARGET_FOLDERS};
pub use safe_token::{to_safe_string, to_unsafe_string};

/// Multi-byte codepage used for the "dummy Latin-1" layer.
///
/// Only [`Codepage::Gbk`] (Simplified Chinese, cp936) is implemented today; the
/// enum exists so other Paradox-community codepages (Big5, cp932, ...) can be
/// added without changing the public conversion API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Codepage {
    /// Simplified Chinese, GBK / cp936.
    #[default]
    Gbk,
}

impl Codepage {
    /// Parse a codepage from a CLI-friendly string.
    pub fn parse(s: &str) -> Option<Codepage> {
        match s.to_ascii_lowercase().as_str() {
            "gbk" | "gb" | "gb2312" | "cp936" | "936" => Some(Codepage::Gbk),
            _ => None,
        }
    }
}

/// Direction of a conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Direction {
    /// Game format (GBK dummy-Latin1) -> readable Unicode (UTF-8).
    ToUnicode,
    /// Readable Unicode (UTF-8) -> game format (GBK dummy-Latin1).
    ToGame,
}

/// Options controlling a conversion.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConvertOptions {
    /// Codepage for the dummy-Latin1 layer.
    pub codepage: Codepage,
    /// Protect `$VAR$`/`§`/`£`/`¤` with `<...>` safe tokens (default off).
    ///
    /// Off keeps the readable text using the raw Paradox control codes, which is
    /// the symmetric round-trip most mod workflows want. Turn it on only to
    /// shield those codes from external translation/spreadsheet tooling.
    pub safe_tokens: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        ConvertOptions {
            codepage: Codepage::default(),
            safe_tokens: false,
        }
    }
}

/// Errors produced by the file/directory conversion helpers.
#[derive(Debug, thiserror::Error)]
pub enum Vic2EncError {
    /// Underlying filesystem error, annotated with the offending path.
    #[error("{path}: {source}")]
    Io {
        /// Path being read or written when the error occurred.
        path: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// A directory walk failed.
    #[error("walk error: {0}")]
    Walk(#[from] walkdir::Error),
}

/// Convenient result alias for the crate's fallible operations.
pub type Result<T> = std::result::Result<T, Vic2EncError>;
