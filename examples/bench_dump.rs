// Run duckling against a dump of real inputs and report timing outliers.
// Usage: cargo run --release --example bench_dump -- <path>...
//
// Each <path> is either:
//   - a single text file (treated as one blob, or split on
//     "======== idx=N ... ========" headers if present);
//   - or a directory, in which case every regular file under it
//     (recursively) is processed the same way.

use std::time::Instant;

use chrono::{FixedOffset, TimeZone};
use duckling::{parse, Context, DimensionKind, Lang, Locale, Options};

fn all_dims() -> [DimensionKind; 14] {
    [
        DimensionKind::Numeral,
        DimensionKind::Ordinal,
        DimensionKind::Temperature,
        DimensionKind::Distance,
        DimensionKind::Volume,
        DimensionKind::Quantity,
        DimensionKind::AmountOfMoney,
        DimensionKind::Email,
        DimensionKind::PhoneNumber,
        DimensionKind::Url,
        DimensionKind::CreditCardNumber,
        DimensionKind::TimeGrain,
        DimensionKind::Duration,
        DimensionKind::Time,
    ]
}

fn split_sources(raw: &str) -> Vec<(String, String)> {
    if !raw.contains("======== idx=") {
        return vec![("whole_file".into(), raw.to_string())];
    }
    let mut out = Vec::new();
    let mut current_label = String::from("preamble");
    let mut current_body = String::new();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("======== idx=") {
            if !current_body.trim().is_empty() {
                out.push((current_label.clone(), current_body.clone()));
            }
            let label = rest.trim_end_matches(" ========").trim();
            current_label = format!("idx={label}");
            current_body.clear();
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    if !current_body.trim().is_empty() {
        out.push((current_label, current_body));
    }
    out
}

fn collect_files(path: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    if path.is_dir() {
        let entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("skipping {}: {e}", path.display());
                return;
            }
        };
        for entry in entries.flatten() {
            collect_files(&entry.path(), out);
        }
    } else if path.is_file() {
        out.push(path.to_path_buf());
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: bench_dump <path>...");
        std::process::exit(2);
    }

    let mut files = Vec::new();
    for arg in &args {
        collect_files(std::path::Path::new(arg), &mut files);
    }
    if files.is_empty() {
        eprintln!("no files found in: {args:?}");
        std::process::exit(1);
    }

    let locale = Locale::new(Lang::EN, None);
    let ctx = Context::new(
        FixedOffset::west_opt(7 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 4, 14, 20, 0, 0)
            .unwrap(),
        locale,
    );
    let opts = Options::default();
    let dims = all_dims();

    let mut slow: Vec<(String, String, std::time::Duration, usize)> = Vec::new();
    let mut total = std::time::Duration::ZERO;
    let mut count = 0usize;

    for path in &files {
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("skipping {}: {e}", path.display());
                continue;
            }
        };
        let blocks = split_sources(&raw);
        for (label, body) in blocks {
            let start = Instant::now();
            let entities = parse(&body, &locale, &dims, &ctx, &opts);
            let elapsed = start.elapsed();
            total += elapsed;
            count += 1;
            let tag = format!("{}:{}", path.display(), label);
            println!(
                "{:>8.2?}  {:>4} entities  {:>5} chars  {}",
                elapsed,
                entities.len(),
                body.len(),
                tag,
            );
            if elapsed >= std::time::Duration::from_millis(250) {
                slow.push((tag, body, elapsed, entities.len()));
            }
        }
    }

    println!("\n===== SUMMARY =====");
    println!("blocks: {count}, total: {total:.2?}");
    if slow.is_empty() {
        println!("no blocks >=250ms");
    } else {
        slow.sort_by(|a, b| b.2.cmp(&a.2));
        println!("slow blocks (>=250ms), slowest first:");
        for (tag, body, elapsed, n_entities) in &slow {
            let preview: String = body.chars().take(120).collect();
            println!(
                "  {:>8.2?}  {} entities  {} chars  {}\n    preview: {:?}",
                elapsed,
                n_entities,
                body.len(),
                tag,
                preview.replace('\n', " "),
            );
        }
    }
}
