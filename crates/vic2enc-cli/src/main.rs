//! `vic2enc` — command-line encoding converter for Victoria 2 localisation CSV.
//!
//! A dependency-light, pure-Rust fallback for platforms where the Tauri desktop
//! app (which needs a WebView2 runtime) cannot run.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use vic2enc_core::{convert_dir, convert_file, BatchScope, Codepage, ConvertOptions, Direction};

/// Convert Victoria 2 localisation CSV files between game format (GBK
/// dummy-Latin1) and readable Unicode.
#[derive(Parser)]
#[command(name = "vic2enc", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Game format (GBK dummy-Latin1) -> readable UTF-8.
    Decode(ConvArgs),
    /// Readable UTF-8 -> game format (GBK dummy-Latin1).
    Encode(ConvArgs),
}

#[derive(Args)]
struct ConvArgs {
    /// Input file, or a directory to process recursively (`*.csv`).
    #[arg(short, long)]
    input: PathBuf,

    /// Output file, or output directory when `--input` is a directory.
    #[arg(short, long)]
    output: PathBuf,

    /// Protect `$VAR$`/`§`/`£`/`¤` with inert `<...>` safe tokens (off by
    /// default; enable when handing the readable text to external translation
    /// or spreadsheet tools that might mangle the raw control codes).
    #[arg(long)]
    safe_tokens: bool,

    /// In directory mode, skip script `*.txt` entirely and convert only `*.csv`
    /// (by default `*.txt` under `events`/`decisions` is converted too).
    #[arg(long)]
    no_txt: bool,

    /// In directory mode, also convert `*.txt` outside `events`/`decisions`
    /// folders (by default script `*.txt` is converted only there; `*.csv`
    /// is always converted). Ignored when `--no-txt` is set.
    #[arg(long)]
    all_txt: bool,

    /// Target codepage for the dummy-Latin1 layer.
    #[arg(long, default_value = "gbk")]
    codepage: String,
}

impl ConvArgs {
    fn options(&self) -> Result<ConvertOptions> {
        let codepage = Codepage::parse(&self.codepage)
            .with_context(|| format!("unsupported codepage: {}", self.codepage))?;
        Ok(ConvertOptions {
            codepage,
            safe_tokens: self.safe_tokens,
        })
    }
}

fn run(args: &ConvArgs, dir: Direction) -> Result<()> {
    let opts = args.options()?;
    let input: &Path = &args.input;
    if !input.exists() {
        bail!("input path does not exist: {}", input.display());
    }
    let stats = if input.is_dir() {
        let scope = BatchScope {
            convert_txt: !args.no_txt,
            txt_all_folders: args.all_txt,
        };
        convert_dir(input, &args.output, dir, &opts, scope)?
    } else {
        convert_file(input, &args.output, dir, &opts)?
    };
    println!("Done: {stats}");
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::Decode(args) => run(args, Direction::ToUnicode),
        Command::Encode(args) => run(args, Direction::ToGame),
    }
}
