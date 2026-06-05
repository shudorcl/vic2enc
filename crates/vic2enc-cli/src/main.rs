//! `vic2enc` — command-line encoding converter for Victoria 2 localisation CSV.
//!
//! A dependency-light, pure-Rust fallback for platforms where the Tauri desktop
//! app (which needs a WebView2 runtime) cannot run.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use vic2enc_core::{convert_dir, convert_file, Codepage, ConvertOptions, Direction};

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

    /// Disable `<...>` safe tokens for `$VAR$`/`§`/`£`/`¤` (enabled by default).
    #[arg(long)]
    no_safe_tokens: bool,

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
            safe_tokens: !self.no_safe_tokens,
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
        convert_dir(input, &args.output, dir, &opts)?
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
