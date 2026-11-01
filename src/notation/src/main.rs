//! CLI entrypoint for Razomy Notation (`rn`) v13.0.

use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process;

use razomy_notation::{generate, import_from, parse_rn, Format, LspServer};

#[derive(Parser)]
#[command(
    name = "rn",
    version = "13.0.0",
    about = "Razomy Notation (.rn) v13.0 — Ultra-Lean Unified Node Parser, HIR, Formatter, Multi-Format Codegen/Importer & LSP"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and validate a `.rn` file, emitting diagnostics and HIR summary
    Check {
        /// Input `.rn` file path
        file: PathBuf,
    },
    /// Format a `.rn` file into canonical 1-space indentation
    Fmt {
        /// Input `.rn` file path
        file: PathBuf,
        /// Write formatted output back to file in-place
        #[arg(short, long)]
        write: bool,
    },
    /// Convert between `.rn` and other formats (`json`, `json5`, `yaml`, `proto`, `mermaid`, `sql`, `html`, `md`, `csv`)
    Convert {
        /// Input file path
        input: PathBuf,
        /// Source format (`rn`, `json`, `json5`, `yaml`, `proto`, `mermaid`, `sql`, `html`, `md`, `csv`)
        #[arg(short, long)]
        from: Option<String>,
        /// Target format (`rn`, `json`, `json5`, `yaml`, `proto`, `mermaid`, `sql`, `html`, `md`, `csv`)
        #[arg(short, long)]
        to: String,
        /// Optional output file path (prints to stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Start the Razomy Notation Language Server Protocol (LSP) server over stdio
    Lsp,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { file } => {
            let src = read_file_or_exit(&file);
            let doc = parse_rn(&src);
            if doc.diagnostics.is_empty() {
                println!(
                    "OK: {} parsed cleanly ({} symbols, root mode: {:?})",
                    file.display(),
                    doc.symbols.len(),
                    doc.root_mode
                );
            } else {
                for d in &doc.diagnostics {
                    eprintln!(
                        "{}:{}:{}: [{:?} {}] {}",
                        file.display(),
                        d.line + 1,
                        d.col + 1,
                        d.severity,
                        d.code,
                        d.message
                    );
                }
                process::exit(1);
            }
        }
        Commands::Fmt { file, write } => {
            let src = read_file_or_exit(&file);
            let doc = parse_rn(&src);
            let formatted = generate(&doc, Format::Rn).unwrap_or_else(|e| {
                eprintln!("Formatting error: {}", e);
                process::exit(1);
            });
            if write {
                fs::write(&file, formatted).unwrap_or_else(|e| {
                    eprintln!("Failed to write {}: {}", file.display(), e);
                    process::exit(1);
                });
            } else {
                print!("{}", formatted);
            }
        }
        Commands::Convert {
            input,
            from,
            to,
            output,
        } => {
            let src = read_file_or_exit(&input);
            let from_fmt = from
                .as_deref()
                .and_then(Format::from_str_name)
                .or_else(|| {
                    input
                        .extension()
                        .and_then(|e| e.to_str())
                        .and_then(Format::from_str_name)
                })
                .unwrap_or(Format::Rn);

            let to_fmt = Format::from_str_name(&to).unwrap_or_else(|| {
                eprintln!(
                    "Unknown target format '{}'. Supported: rn, json, json5, yaml, proto, mermaid, sql, html, md, csv",
                    to
                );
                process::exit(1);
            });

            let doc = import_from(&src, from_fmt).unwrap_or_else(|e| {
                eprintln!("Import error ({:?}): {}", from_fmt, e);
                process::exit(1);
            });

            let out_str = generate(&doc, to_fmt).unwrap_or_else(|e| {
                eprintln!("Codegen error ({:?}): {}", to_fmt, e);
                process::exit(1);
            });

            if let Some(out_path) = output {
                fs::write(&out_path, out_str).unwrap_or_else(|e| {
                    eprintln!("Failed to write {}: {}", out_path.display(), e);
                    process::exit(1);
                });
            } else {
                print!("{}", out_str);
            }
        }
        Commands::Lsp => {
            let mut server = LspServer::new();
            if let Err(e) = server.run_stdio() {
                eprintln!("LSP server error: {}", e);
                process::exit(1);
            }
        }
    }
}

fn read_file_or_exit(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {}", path.display(), e);
        process::exit(1);
    })
}