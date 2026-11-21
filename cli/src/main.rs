use std::path::PathBuf;

use anyconvert_core::{
    all_conversions, convert_many, extension_of, format, targets_for, ConvertOptions,
};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "anyconvert",
    version,
    about = "Fast, fully offline file format converter",
    long_about = "Converts images, structured data, text/markup, audio to WAV, and PDF.\nEverything runs locally with no network access and no external tools."
)]
struct Cli {
    /// Input files (any number)
    #[arg(value_name = "FILE")]
    inputs: Vec<PathBuf>,

    /// Target extension, e.g. png, webp, json, pdf, wav
    #[arg(short = 't', long = "to", value_name = "EXT")]
    to: Option<String>,

    /// Directory for converted files (default: next to each input)
    #[arg(short = 'o', long = "out-dir", value_name = "DIR")]
    out_dir: Option<PathBuf>,

    /// Source extension override (for extensionless files)
    #[arg(long = "from", value_name = "EXT")]
    from: Option<String>,

    /// Quality for lossy image output, 1-100 (default 85)
    #[arg(short = 'q', long = "quality", value_name = "N")]
    quality: Option<u8>,

    /// Allow replacing files that already exist
    #[arg(long = "overwrite")]
    overwrite: bool,

    /// List known formats
    #[arg(long = "list-formats")]
    list_formats: bool,

    /// List every supported from -> to conversion
    #[arg(long = "list-conversions")]
    list_conversions: bool,

    /// List targets compatible with one source extension
    #[arg(long = "targets", value_name = "EXT")]
    targets: Option<String>,

    /// Emit a machine-readable JSON summary
    #[arg(long = "json")]
    json: bool,

    /// Suppress progress output
    #[arg(short = 'Q', long = "quiet")]
    quiet: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.list_formats {
        print_formats();
        return;
    }
    if cli.list_conversions {
        print_conversions();
        return;
    }
    if let Some(ext) = &cli.targets {
        print_targets(ext);
        return;
    }

    let to = match &cli.to {
        Some(t) => anyconvert_core::model::canonical(t).to_string(),
        None => {
            eprintln!("error: specify a target format with --to <EXT>");
            eprintln!("try '{} --list-formats' or '{} --targets png'", "anyconvert", "anyconvert");
            std::process::exit(2);
        }
    };

    if cli.inputs.is_empty() {
        eprintln!("error: no input files given");
        std::process::exit(2);
    }
    if format(&to).map(|f| !f.output).unwrap_or(true) {
        eprintln!("error: '{to}' is not a writable output format");
        std::process::exit(2);
    }

    if !cli.quiet {
        eprintln!("AnyConvert: {} -> .{to}", cli.inputs.len());
    }

    let opts = ConvertOptions {
        quality: cli.quality.unwrap_or(85).clamp(1, 100),
        from_ext: cli.from.clone(),
    };

    let mut files: Vec<PathBuf> = Vec::new();
    for input in &cli.inputs {
        if !input.is_file() {
            eprintln!("error: not a file: {}", input.display());
            std::process::exit(2);
        }
        let from = opts
            .from_ext
            .clone()
            .or_else(|| extension_of(input))
            .unwrap_or_default();
        if !targets_for(&from).contains(&to.as_str()) {
            eprintln!("warning: skipping {} (.{from} -> .{to} unsupported)", input.display());
            continue;
        }
        let out_name = input.with_extension(&to);
        let output = match &cli.out_dir {
            Some(dir) => dir.join(out_name.file_name().unwrap_or_default()),
            None => out_name,
        };
        if output.exists() && !cli.overwrite {
            eprintln!("warning: skipping {} (output exists; use --overwrite)", output.display());
            continue;
        }
        files.push(input.clone());
    }

    let results = convert_many(&files, &to, cli.out_dir.as_deref(), &opts, |done, total, path| {
        if !cli.quiet {
            eprintln!("[{done}/{total}] {}", path.display());
        }
    });

    let mut ok = 0usize;
    let mut failed = 0usize;
    let mut json_rows = Vec::new();

    for (input, result) in &results {
        match result {
            Ok(outcome) => {
                ok += 1;
                if cli.json {
                    json_rows.push(serde_json::json!({
                        "input": input.to_string_lossy(),
                        "output": outcome.output.to_string_lossy(),
                        "bytes": outcome.bytes,
                        "note": outcome.note,
                    }));
                } else {
                    println!("ok   {} -> {} ({} bytes)", input.display(), outcome.output.display(), outcome.bytes);
                    if let Some(note) = &outcome.note {
                        println!("     note: {note}");
                    }
                }
            }
            Err(err) => {
                failed += 1;
                if cli.json {
                    json_rows.push(serde_json::json!({
                        "input": input.to_string_lossy(),
                        "error": err.to_string(),
                    }));
                } else {
                    eprintln!("fail {} : {err}", input.display());
                }
            }
        }
    }

    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "converted": ok,
                "failed": failed,
                "results": json_rows,
            }))
            .unwrap()
        );
    } else if !cli.quiet {
        eprintln!("done: {ok} converted, {failed} failed");
    }

    if failed > 0 {
        std::process::exit(1);
    }
}

fn print_formats() {
    println!("{:<12} {:<18} {:<6} {:<6}", "EXT", "FORMAT", "READ", "WRITE");
    println!("{}", "-".repeat(46));
    for f in anyconvert_core::FORMATS {
        println!(
            "{:<12} {:<18} {:<6} {:<6}",
            f.ext,
            f.label,
            if f.input { "yes" } else { "no" },
            if f.output { "yes" } else { "no" },
        );
    }
}

fn print_conversions() {
    let all = all_conversions();
    println!("{} supported conversions\n", all.len());
    let mut current = "";
    for c in &all {
        if c.from != current {
            current = c.from;
            println!(".{current} ->");
        }
        println!("    .{}", c.to);
    }
}

fn print_targets(ext: &str) {
    let ext = anyconvert_core::model::canonical(ext);
    let targets = targets_for(ext);
    if targets.is_empty() {
        println!("No conversions from '.{ext}'.");
        std::process::exit(1);
    }
    println!(".{ext} can be converted to: {}", targets.join(", "));
}
