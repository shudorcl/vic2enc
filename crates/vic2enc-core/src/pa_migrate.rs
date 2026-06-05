//! "PA migration": restructure a Victoria 2 Chinese mod that hard-codes its
//! translations into the localisation CSVs and the `events`/`decisions`/`common`
//! scripts (the "PA"汉化 layout) into the modern UTF-8 `assets/localisation/zh-CN`
//! layout.
//!
//! Ported (and tightened) from the community `pa迁移.py` script. Two phases:
//!
//! 1. **Localisation**: every file under `<mod>/localisation/` is decoded from
//!    game format (GBK dummy-Latin1) to readable UTF-8 and written, mirroring the
//!    tree, into `<mod>/assets/localisation/zh-CN/`.
//! 2. **Scripts**: in the target folders (default `events`, `decisions`,
//!    `common`), every `*.txt` line of the form `key = "中文" # English` whose
//!    quoted value contains Chinese is rewritten to `key = "English"` (the
//!    English comment becomes the value / localisation key), and the
//!    `English;中文` pair is appended to `<mod>/assets/localisation/zh-CN/<txt>.csv`.
//!
//! Improvements over the reference script: byte-level dummy-Latin1 decoding
//! instead of a lossy `gb2312` read, original indentation and line endings are
//! preserved, and the whole mod's affected folders are backed up first.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use walkdir::WalkDir;

use crate::dummy_latin1::dummy_latin1_to_unicode;
use crate::{Codepage, Result, Vic2EncError};

/// Folders scanned for inline-Chinese `*.txt` scripts by default.
pub const DEFAULT_TARGET_FOLDERS: &[&str] = &["events", "decisions", "common"];

/// Options for [`pa_migrate`].
#[derive(Debug, Clone)]
pub struct PaMigrateOptions {
    /// Codepage of the source game files (only [`Codepage::Gbk`] today).
    pub codepage: Codepage,
    /// Back up affected folders before modifying anything (default `true`).
    pub backup: bool,
    /// Script folders to scan for inline Chinese (defaults to
    /// [`DEFAULT_TARGET_FOLDERS`] when empty).
    pub target_folders: Vec<String>,
}

impl Default for PaMigrateOptions {
    fn default() -> Self {
        PaMigrateOptions {
            codepage: Codepage::default(),
            backup: true,
            target_folders: DEFAULT_TARGET_FOLDERS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Summary of a completed migration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PaMigrateReport {
    /// Where the pre-migration backup was written (`None` if backup disabled).
    pub backup_dir: Option<String>,
    /// Files copied from `localisation/` into `assets/localisation/zh-CN/`.
    pub localisation_files: usize,
    /// `*.txt` scripts scanned in the target folders.
    pub scripts_scanned: usize,
    /// `*.txt` scripts that had at least one inline-Chinese line rewritten.
    pub scripts_modified: usize,
    /// Total `English;中文` entries extracted into CSVs.
    pub entries_extracted: usize,
    /// CSV files written under `assets/localisation/zh-CN/`.
    pub csv_files_written: usize,
}

fn io_err(path: &Path) -> impl Fn(std::io::Error) -> Vic2EncError + '_ {
    move |source| Vic2EncError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Run the migration against `mod_dir` in place (after backing it up).
pub fn pa_migrate(mod_dir: &Path, opts: &PaMigrateOptions) -> Result<PaMigrateReport> {
    let mut report = PaMigrateReport::default();

    let targets: Vec<String> = if opts.target_folders.is_empty() {
        DEFAULT_TARGET_FOLDERS.iter().map(|s| s.to_string()).collect()
    } else {
        opts.target_folders.clone()
    };

    // 1. Backup the affected folders before touching anything.
    if opts.backup {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup_root = mod_dir.join(format!("_pa_backup_{stamp}"));
        let mut to_back_up: Vec<&str> = vec!["localisation"];
        to_back_up.extend(targets.iter().map(|s| s.as_str()));
        for folder in to_back_up {
            let src = mod_dir.join(folder);
            if src.is_dir() {
                copy_dir(&src, &backup_root.join(folder))?;
            }
        }
        report.backup_dir = Some(backup_root.display().to_string());
    }

    let dest_root = mod_dir.join("assets").join("localisation").join("zh-CN");

    // 2. Decode localisation/* to UTF-8 under assets/localisation/zh-CN/.
    let loc_src = mod_dir.join("localisation");
    if loc_src.is_dir() {
        for entry in WalkDir::new(&loc_src) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let rel = path.strip_prefix(&loc_src).unwrap_or(path);
            let out_path = dest_root.join(rel);
            let data = std::fs::read(path).map_err(io_err(path))?;
            let text = dummy_latin1_to_unicode(&data, opts.codepage);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(io_err(parent))?;
            }
            std::fs::write(&out_path, text.as_bytes()).map_err(io_err(&out_path))?;
            report.localisation_files += 1;
        }
    }

    // 3. Extract inline Chinese from scripts in the target folders.
    for folder in &targets {
        let folder_path = mod_dir.join(folder);
        if !folder_path.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&folder_path) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("txt"))
                != Some(true)
            {
                continue;
            }
            report.scripts_scanned += 1;

            let data = std::fs::read(path).map_err(io_err(path))?;
            let text = dummy_latin1_to_unicode(&data, opts.codepage);
            let (rewritten, entries) = extract_inline_chinese(&text);
            if entries.is_empty() {
                continue;
            }

            // Restore the English-only script (now pure ASCII) as UTF-8.
            std::fs::write(path, rewritten.as_bytes()).map_err(io_err(path))?;
            report.scripts_modified += 1;
            report.entries_extracted += entries.len();

            // Append `English;中文;x` rows to a sibling CSV under zh-CN/.
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("extracted");
            let csv_path = dest_root.join(format!("{stem}.csv"));
            let existed = csv_path.exists();
            std::fs::create_dir_all(&dest_root).map_err(io_err(&dest_root))?;
            let mut csv = String::new();
            for (english, chinese) in &entries {
                csv.push_str(english);
                csv.push(';');
                csv.push_str(chinese);
                csv.push_str(";x\n");
            }
            append_str(&csv_path, &csv)?;
            if !existed {
                report.csv_files_written += 1;
            }
        }
    }

    Ok(report)
}

/// Rewrite a decoded script, pulling inline-Chinese `key = "中文" # English`
/// lines into `(English, 中文)` pairs and replacing each with `key = "English"`.
/// Indentation and line endings are preserved. Returns the new text and the
/// extracted pairs.
fn extract_inline_chinese(text: &str) -> (String, Vec<(String, String)>) {
    let mut out = String::with_capacity(text.len());
    let mut entries = Vec::new();

    for chunk in text.split_inclusive('\n') {
        let (content, term): (&str, &str) = if let Some(c) = chunk.strip_suffix("\r\n") {
            (c, "\r\n")
        } else if let Some(c) = chunk.strip_suffix('\n') {
            (c, "\n")
        } else {
            (chunk, "")
        };

        if let Some(m) = parse_pa_line(content) {
            if contains_chinese(&m.value) {
                entries.push((m.comment.clone(), m.value.clone()));
                out.push_str(m.indent);
                out.push_str(m.key);
                out.push_str(" = \"");
                out.push_str(&m.comment);
                out.push('"');
                out.push_str(term);
                continue;
            }
        }
        out.push_str(content);
        out.push_str(term);
    }

    (out, entries)
}

struct PaLine<'a> {
    indent: &'a str,
    key: &'a str,
    value: String,
    comment: String,
}

/// Parse one line of the form `<indent>key = "value" # comment`, mirroring the
/// reference regex `(\w+)\s*=\s*"(...)"\s*#\s*(.+)`. Returns `None` if the line
/// is not an inline-translation line.
fn parse_pa_line(line: &str) -> Option<PaLine<'_>> {
    let trimmed_start = line.trim_start();
    let indent = &line[..line.len() - trimmed_start.len()];

    let eq = trimmed_start.find('=')?;
    let key = trimmed_start[..eq].trim_end();
    if key.is_empty() || !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }

    let rest = trimmed_start[eq + 1..].trim_start();
    let mut it = rest.chars();
    if it.next()? != '"' {
        return None;
    }

    // Quoted value, honouring backslash escapes.
    let mut value = String::new();
    let mut closed = false;
    while let Some(c) = it.next() {
        match c {
            '\\' => {
                value.push('\\');
                if let Some(n) = it.next() {
                    value.push(n);
                }
            }
            '"' => {
                closed = true;
                break;
            }
            _ => value.push(c),
        }
    }
    if !closed {
        return None;
    }

    // Whitespace, then `#`, then the comment (English) up to end of line.
    let mut saw_hash = false;
    let mut comment = String::new();
    for c in it.by_ref() {
        if !saw_hash {
            if c.is_whitespace() {
                continue;
            }
            if c == '#' {
                saw_hash = true;
                continue;
            }
            return None;
        }
        comment.push(c);
    }
    if !saw_hash {
        return None;
    }
    let comment = comment.trim().to_string();
    if comment.is_empty() {
        return None;
    }

    Some(PaLine {
        indent,
        key,
        value,
        comment,
    })
}

/// Whether `text` contains a CJK Unified Ideograph (same range the reference
/// `contains_chinese` checks).
fn contains_chinese(text: &str) -> bool {
    text.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

fn append_str(path: &Path, s: &str) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(io_err(path))?;
    f.write_all(s.as_bytes()).map_err(io_err(path))?;
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(src).unwrap_or(path);
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target).map_err(io_err(&target))?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(io_err(parent))?;
            }
            std::fs::copy(path, &target).map_err(io_err(&target))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inline_translation_line() {
        let m = parse_pa_line("\t\tname = \"中国\" # China").unwrap();
        assert_eq!(m.indent, "\t\t");
        assert_eq!(m.key, "name");
        assert_eq!(m.value, "中国");
        assert_eq!(m.comment, "China");
    }

    #[test]
    fn rejects_non_translation_lines() {
        assert!(parse_pa_line("name = \"plain english\"").is_none()); // no comment
        assert!(parse_pa_line("# just a comment").is_none());
        assert!(parse_pa_line("limit = { foo = bar }").is_none());
        assert!(parse_pa_line("name = \"中国\" #").is_none()); // empty comment
    }

    #[test]
    fn extract_preserves_indent_and_swaps_in_english() {
        let src = "country_event = {\n\tname = \"北京\" # Beijing\n\tdesc = \"plain\"\n}\n";
        let (out, entries) = extract_inline_chinese(src);
        assert_eq!(
            out,
            "country_event = {\n\tname = \"Beijing\"\n\tdesc = \"plain\"\n}\n"
        );
        assert_eq!(entries, vec![("Beijing".to_string(), "北京".to_string())]);
    }

    #[test]
    fn non_chinese_quoted_value_is_left_untouched() {
        let src = "name = \"Already English\" # Comment\n";
        let (out, entries) = extract_inline_chinese(src);
        assert_eq!(out, src);
        assert!(entries.is_empty());
    }

    #[test]
    fn crlf_line_endings_are_preserved() {
        let src = "name = \"上海\" # Shanghai\r\n";
        let (out, _) = extract_inline_chinese(src);
        assert_eq!(out, "name = \"Shanghai\"\r\n");
    }

    #[test]
    fn end_to_end_migration() {
        use crate::{convert_bytes, ConvertOptions, Direction};
        use std::path::PathBuf;

        let base = std::env::temp_dir().join(format!("vic2enc_pa_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let game_opts = ConvertOptions {
            codepage: Codepage::Gbk,
            safe_tokens: false,
        };
        let to_game = |s: &str| convert_bytes(s.as_bytes(), Direction::ToGame, &game_opts);

        // A mod with a GBK localisation file and an events script holding inline
        // Chinese, both stored in game format (dummy-Latin1 GBK bytes).
        std::fs::create_dir_all(base.join("localisation")).unwrap();
        std::fs::create_dir_all(base.join("events")).unwrap();
        std::fs::write(
            base.join("localisation").join("text.csv"),
            to_game("CHINA;中国;x\n"),
        )
        .unwrap();
        std::fs::write(
            base.join("events").join("e.txt"),
            to_game("\tname = \"北京\" # Beijing\n\tdesc = \"plain\"\n"),
        )
        .unwrap();

        let report = pa_migrate(&base, &PaMigrateOptions::default()).unwrap();

        assert_eq!(report.localisation_files, 1);
        assert_eq!(report.scripts_scanned, 1);
        assert_eq!(report.scripts_modified, 1);
        assert_eq!(report.entries_extracted, 1);
        assert_eq!(report.csv_files_written, 1);
        assert!(report.backup_dir.is_some());

        // Localisation decoded to readable UTF-8 under assets/localisation/zh-CN/.
        let zh = base.join("assets").join("localisation").join("zh-CN");
        assert_eq!(
            std::fs::read_to_string(zh.join("text.csv")).unwrap(),
            "CHINA;中国;x\n"
        );

        // Script now English-only; extracted CSV holds English;中文;x.
        assert_eq!(
            std::fs::read_to_string(base.join("events").join("e.txt")).unwrap(),
            "\tname = \"Beijing\"\n\tdesc = \"plain\"\n"
        );
        assert_eq!(
            std::fs::read_to_string(zh.join("e.csv")).unwrap(),
            "Beijing;北京;x\n"
        );

        // Backup preserves the original game-format script bytes.
        let backup = PathBuf::from(report.backup_dir.unwrap());
        let backed_up = std::fs::read(backup.join("events").join("e.txt")).unwrap();
        assert_eq!(backed_up, to_game("\tname = \"北京\" # Beijing\n\tdesc = \"plain\"\n"));

        let _ = std::fs::remove_dir_all(&base);
    }
}
