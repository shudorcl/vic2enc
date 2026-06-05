//! High-level conversion entry points: in-memory bytes, single file, and
//! recursive directory batch over `*.csv`.

use std::fmt;
use std::path::Path;

use walkdir::WalkDir;

use crate::dummy_latin1::{dummy_latin1_to_unicode, unicode_to_dummy_latin1};
use crate::safe_token::{to_safe_string, to_unsafe_string};
use crate::{ConvertOptions, Direction, Result, Vic2EncError};

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// Convert a whole file's worth of bytes in memory.
///
/// - [`Direction::ToUnicode`]: game bytes -> readable UTF-8 (with safe tokens if
///   enabled). The whole buffer is transcoded at once; CSV separators (`;`, `#`,
///   newlines) are all `< 0x40` and so are never absorbed into a CJK byte pair.
/// - [`Direction::ToGame`]: readable UTF-8 -> game bytes. A leading UTF-8 BOM on
///   the input is ignored.
pub fn convert_bytes(input: &[u8], dir: Direction, opts: &ConvertOptions) -> Vec<u8> {
    match dir {
        Direction::ToUnicode => {
            let unicode = dummy_latin1_to_unicode(input, opts.codepage);
            let text = if opts.safe_tokens {
                to_safe_string(&unicode)
            } else {
                unicode
            };
            text.into_bytes()
        }
        Direction::ToGame => {
            let stripped = input.strip_prefix(UTF8_BOM).unwrap_or(input);
            let text = String::from_utf8_lossy(stripped);
            let unicode = if opts.safe_tokens {
                to_unsafe_string(&text)
            } else {
                text.into_owned()
            };
            unicode_to_dummy_latin1(&unicode, opts.codepage)
        }
    }
}

/// Statistics describing a completed conversion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Stats {
    /// Number of files converted.
    pub files: usize,
    /// Total bytes read.
    pub bytes_in: u64,
    /// Total bytes written.
    pub bytes_out: u64,
}

impl Stats {
    fn add_file(&mut self, bytes_in: usize, bytes_out: usize) {
        self.files += 1;
        self.bytes_in += bytes_in as u64;
        self.bytes_out += bytes_out as u64;
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} file(s), {} bytes in, {} bytes out",
            self.files, self.bytes_in, self.bytes_out
        )
    }
}

fn io_err(path: &Path) -> impl Fn(std::io::Error) -> Vic2EncError + '_ {
    move |source| Vic2EncError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Convert a single file, writing the result to `output`.
pub fn convert_file(
    input: &Path,
    output: &Path,
    dir: Direction,
    opts: &ConvertOptions,
) -> Result<Stats> {
    let data = std::fs::read(input).map_err(io_err(input))?;
    let converted = convert_bytes(&data, dir, opts);
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(io_err(parent))?;
        }
    }
    std::fs::write(output, &converted).map_err(io_err(output))?;
    let mut stats = Stats::default();
    stats.add_file(data.len(), converted.len());
    Ok(stats)
}

fn is_csv(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("csv"))
        .unwrap_or(false)
}

/// Recursively convert every `*.csv` file under `input_dir`, mirroring the
/// directory structure into `output_dir`.
pub fn convert_dir(
    input_dir: &Path,
    output_dir: &Path,
    dir: Direction,
    opts: &ConvertOptions,
) -> Result<Stats> {
    let mut total = Stats::default();
    for entry in WalkDir::new(input_dir).into_iter() {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type().is_file() || !is_csv(path) {
            continue;
        }
        let rel = path.strip_prefix(input_dir).unwrap_or(path);
        let out_path = output_dir.join(rel);
        let stats = convert_file(path, &out_path, dir, opts)?;
        total.files += stats.files;
        total.bytes_in += stats.bytes_in;
        total.bytes_out += stats.bytes_out;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Codepage;

    fn opts(safe: bool) -> ConvertOptions {
        ConvertOptions {
            codepage: Codepage::Gbk,
            safe_tokens: safe,
        }
    }

    #[test]
    fn bytes_roundtrip_with_safe_tokens() {
        // With safe tokens on, the human-editable form holds `<...>` tokens; the
        // round-trip identity is decode(encode(safe_form)) == safe_form.
        let editable = "PROV_BEIJING;北京 <VAR-PROVINCE> <A7-Y>重要<A7-!>;x\n".as_bytes();
        let game = convert_bytes(editable, Direction::ToGame, &opts(true));
        // The game bytes must contain the raw control byte, not the token text.
        assert!(game.contains(&0xA7));
        let back = convert_bytes(&game, Direction::ToUnicode, &opts(true));
        assert_eq!(back, editable);
    }

    #[test]
    fn bytes_roundtrip_without_safe_tokens() {
        let original = "KEY;南斯拉夫王国;x\n大公国;x\n".as_bytes();
        let game = convert_bytes(original, Direction::ToGame, &opts(false));
        let back = convert_bytes(&game, Direction::ToUnicode, &opts(false));
        assert_eq!(back, original);
    }

    #[test]
    fn game_output_is_not_utf8_for_chinese() {
        let game = convert_bytes("中国".as_bytes(), Direction::ToGame, &opts(false));
        // GBK encodes each Chinese char to two high bytes, never the 6 UTF-8 bytes.
        assert_eq!(game.len(), 4);
    }

    #[test]
    fn to_game_strips_utf8_bom() {
        let mut with_bom = UTF8_BOM.to_vec();
        with_bom.extend_from_slice("KEY;test;x".as_bytes());
        let game = convert_bytes(&with_bom, Direction::ToGame, &opts(false));
        assert_eq!(game, b"KEY;test;x");
    }
}
