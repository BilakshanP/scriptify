use arborium::{
    AnsiHighlighter,
    theme::{Theme, builtin},
};
use clap::Parser;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use syn_inline_mod::InlinerBuilder;

/* ---------- CLI ---------- */

#[derive(Parser)]
#[command(name = "inline-mod")]
#[command(about = "Inline Rust modules with optional syntax highlighting")]
struct Cli {
    input: Option<PathBuf>,
    output: Option<PathBuf>,

    #[arg(short = 't', long = "theme", default_value = "Monokai")]
    theme: String,

    #[arg(long = "no-color")]
    no_color: bool,

    #[arg(long = "list-themes")]
    list_themes: bool,

    #[arg(short = 'm', long = "manifest-path")]
    zscript_manifest: Option<PathBuf>,

    #[arg(long = "auto-zscript")]
    auto_zscript: bool,
}

/* ---------- Themes ---------- */

fn build_theme_map() -> HashMap<String, Theme> {
    builtin::all()
        .into_iter()
        .map(|t| (t.name.clone(), t.clone()))
        .collect()
}

fn list_themes_and_exit() -> ! {
    println!("Available themes:");
    for t in builtin::all() {
        println!("  - {}", t.name);
    }
    std::process::exit(0)
}

/* ---------- Zscript ---------- */

fn find_manifest_from(start: &Path, stop: &Path) -> Option<PathBuf> {
    let mut cur = start.canonicalize().ok()?;
    let stop = stop.canonicalize().ok()?;

    loop {
        let cand = cur.join("Cargo.toml");
        if cand.exists() {
            return Some(cand);
        }
        if cur == stop || !cur.pop() {
            break;
        }
    }
    None
}

fn resolve_zscript_manifest(cli: &Cli, input: &Path) -> Option<PathBuf> {
    if let Some(p) = &cli.zscript_manifest {
        return Some(p.clone());
    }

    if cli.auto_zscript {
        let start = input.parent().unwrap_or(Path::new("."));
        let stop = std::env::current_dir().unwrap();
        return find_manifest_from(start, &stop);
    }

    None
}

fn build_zscript(manifest: &Path, body: &str) -> String {
    let manifest_src = std::fs::read_to_string(manifest).expect("Failed to read Cargo.toml");

    let mut buf = String::new();
    buf.push_str("#!/usr/bin/env -S cargo run -qZscript --release --manifest-path\n");
    buf.push_str("---cargo\n");
    buf.push_str(&manifest_src);
    if !manifest_src.ends_with('\n') {
        buf.push('\n');
    }
    buf.push_str("---\n\n");
    buf.push_str(body);
    buf
}

/* ---------- Core ---------- */

fn require_input(cli: &Cli) -> PathBuf {
    cli.input.clone().unwrap_or_else(|| {
        eprintln!("Error: <INPUT> is required");
        std::process::exit(1);
    })
}

fn inline_and_format(input: &Path) -> String {
    let res = InlinerBuilder::default()
        .parse_and_inline_modules(input)
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });

    prettyplease::unparse(res.output())
}

/* ---------- Coloring ---------- */

fn colorize(theme: &str, src: &str) -> String {
    let theme = build_theme_map()
        .get(theme)
        .unwrap_or_else(|| {
            eprintln!("Unknown theme: {theme}");
            std::process::exit(1);
        })
        .clone();

    let mut hl = AnsiHighlighter::new(theme);
    hl.highlight("rust", src)
        .unwrap_or_else(|_| src.to_string())
}

/* ---------- Output ---------- */

fn write_file(path: &Path, content: &str) {
    std::fs::write(path, content).unwrap();
}

fn stdout_plain(text: &str) {
    println!("{text}");
}

fn stdout_colored(text: &str, theme: &str) {
    println!("{}", colorize(theme, text));
}

/* ---------- Main ---------- */

fn main() {
    let cli = Cli::parse();

    if cli.list_themes {
        list_themes_and_exit();
    }

    let input = require_input(&cli);
    let body = inline_and_format(&input);
    let zscript_manifest = resolve_zscript_manifest(&cli, &input);
    let zscript = zscript_manifest.is_some();

    /* ---- output file path ---- */
    if let Some(out) = &cli.output {
        if let Some(manifest) = zscript_manifest {
            let full = build_zscript(&manifest, &body);
            write_file(out, &full);
        } else {
            write_file(out, &body);
        }
        return;
    }

    /* ---- stdout path ---- */

    if !zscript {
        // hard error: stdout + colorization is forbidden
        eprintln!("Error: stdout without zscript is not allowed (use --output)");
        std::process::exit(1);
    }

    // zscript + stdout
    let manifest = zscript_manifest.unwrap();
    let manifest_src = std::fs::read_to_string(&manifest).expect("Failed to read Cargo.toml");

    // manifest: NEVER colored
    println!("#!/usr/bin/env -S cargo run -qZscript --release --manifest-path");
    println!("---cargo");
    print!("{manifest_src}");
    if !manifest_src.ends_with('\n') {
        println!();
    }
    println!("---\n");

    // body: colorized unless explicitly disabled
    if cli.no_color {
        stdout_plain(&body);
    } else {
        stdout_colored(&body, &cli.theme);
    }
}
