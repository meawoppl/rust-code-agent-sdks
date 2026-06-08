//! Parse a directory tree of Claude Code session-transcript JSONL files and
//! report every line that fails to deserialize as [`ClaudeOutput`].
//!
//! This is a coverage tool: it sweeps real-world transcripts and surfaces the
//! lines the crate's top-level parser can't yet handle, grouped by the line's
//! `type` and a signature of the serde error.
//!
//! # Usage
//!
//! ```bash
//! # Defaults to ~/.claude/projects
//! cargo run --example parse_transcripts
//!
//! # Or point at a specific directory / file tree
//! cargo run --example parse_transcripts -- /path/to/transcripts
//! ```
//!
//! Note: session transcripts are a superset of the `--print` stream format
//! this crate models; many lines (e.g. `queue-operation`, `attachment`,
//! `ai-title`) have no `ClaudeOutput` variant and are expected to fail. The
//! value is in the breakdown — which modeled types still slip through, and
//! what the unmodeled ones are.

use claude_codes::ClaudeOutput;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Recursively collect every `*.jsonl` file under `dir`.
fn find_jsonl(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_jsonl(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "jsonl") {
            out.push(path);
        }
    }
}

/// Reduce a serde error message to a stable signature for grouping (drops the
/// trailing `at line N column M`, which is per-line noise).
fn error_signature(err: &str) -> String {
    match err.find(" at line ") {
        Some(idx) => err[..idx].to_string(),
        None => err.to_string(),
    }
}

/// Pull the `type` field out of a raw line for grouping, without committing to
/// any typed shape.
fn line_type(line: &str) -> String {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(|v| {
            v.get("type")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "<no-type>".to_string())
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/.claude/projects")
    });

    let mut files = Vec::new();
    find_jsonl(Path::new(&dir), &mut files);
    files.sort();

    if files.is_empty() {
        eprintln!("No .jsonl files found under {dir}");
        std::process::exit(2);
    }

    let mut total_lines = 0usize;
    let mut total_fail = 0usize;
    let mut json_errors = 0usize;
    // type -> (seen, failed)
    let mut by_type: BTreeMap<String, [usize; 2]> = BTreeMap::new();
    // "type: error-signature" -> (count, first sample location)
    let mut failures: BTreeMap<String, (usize, String)> = BTreeMap::new();

    for file in &files {
        let Ok(content) = fs::read_to_string(file) else {
            eprintln!("warning: could not read {}", file.display());
            continue;
        };
        for (idx, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            total_lines += 1;

            // Lines that aren't even valid JSON are tracked separately.
            if serde_json::from_str::<serde_json::Value>(line).is_err() {
                json_errors += 1;
                total_fail += 1;
                let slot = by_type.entry("<invalid-json>".to_string()).or_default();
                slot[0] += 1;
                slot[1] += 1;
                continue;
            }

            let typ = line_type(line);
            let slot = by_type.entry(typ.clone()).or_default();
            slot[0] += 1;

            if let Err(err) = serde_json::from_str::<ClaudeOutput>(line) {
                total_fail += 1;
                slot[1] += 1;
                let sig = format!("{typ}: {}", error_signature(&err.to_string()));
                let entry = failures
                    .entry(sig)
                    .or_insert_with(|| (0, format!("{}:{}", file.display(), idx + 1)));
                entry.0 += 1;
            }
        }
    }

    let ok = total_lines - total_fail;
    println!("Scanned {} file(s) under {dir}", files.len());
    println!(
        "Lines: {total_lines} total | {ok} parsed as ClaudeOutput | {total_fail} failed ({})",
        pct(total_fail, total_lines)
    );
    if json_errors > 0 {
        println!("  ({json_errors} of the failures were not valid JSON at all)");
    }

    println!("\nBy line `type` (failed / seen):");
    for (typ, [seen, failed]) in &by_type {
        let mark = if *failed == 0 { "✓" } else { "✗" };
        println!(
            "  {mark} {typ:<24} {failed:>6} / {seen:<6} ({})",
            pct(*failed, *seen)
        );
    }

    if failures.is_empty() {
        println!("\nNo parse failures. 🎉");
        return;
    }

    // Sort failure signatures by descending count.
    let mut ranked: Vec<_> = failures.iter().collect();
    ranked.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    println!(
        "\nDistinct failure signatures ({}), most frequent first:",
        ranked.len()
    );
    for (sig, (count, sample)) in ranked {
        println!("\n  [{count}×] {sig}");
        println!("        e.g. {sample}");
    }

    // Non-zero exit so this can gate CI if ever wired up.
    std::process::exit(1);
}

/// Format `n/d` as a percentage string, guarding divide-by-zero.
fn pct(n: usize, d: usize) -> String {
    if d == 0 {
        "0%".to_string()
    } else {
        format!("{:.1}%", (n as f64 / d as f64) * 100.0)
    }
}
