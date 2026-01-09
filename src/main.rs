use arborium::{AnsiHighlighter, theme::builtin};
use clap::Parser;
use std::path::{Path, PathBuf};
use syn_inline_mod::InlinerBuilder;

/// Inline Rust modules with optional syntax highlighting and cargo-script support
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Input Rust source file
    input: Option<PathBuf>,

    /// Output file (defaults to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Enable syntax highlighting with specified theme
    #[arg(short, long)]
    theme: Option<String>,

    /// List all available themes
    #[arg(long)]
    list_themes: bool,

    /// Path to Cargo.toml for cargo-script generation
    #[arg(short = 'm', long)]
    manifest: Option<PathBuf>,

    /// Auto-discover Cargo.toml from input file location
    #[arg(short = 'z', long)]
    zscript: bool,

    /// Stop searching for Cargo.toml at current working directory
    #[arg(long, requires = "zscript")]
    stop_at_cwd: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.list_themes {
        list_themes();
        return;
    }

    let input = cli.input.as_ref().unwrap_or_else(|| {
        eprintln!("error: <INPUT> is required");
        std::process::exit(1);
    });

    let code = inline_modules(input);
    let manifest = resolve_manifest(&cli, input);

    if let Some(out_path) = &cli.output {
        write_output(out_path, &code, manifest.as_deref());
    } else {
        print_to_stdout(&cli, &code, manifest.as_deref());
    }
}

fn list_themes() {
    println!("Available themes:");
    for theme in builtin::all() {
        println!("  {}", theme.name);
    }
}

fn inline_modules(input: &Path) -> String {
    let result = InlinerBuilder::default()
        .parse_and_inline_modules(input)
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            std::process::exit(1);
        });

    prettyplease::unparse(result.output())
}

fn resolve_manifest(cli: &Cli, input: &Path) -> Option<PathBuf> {
    if let Some(manifest) = &cli.manifest {
        return Some(manifest.clone());
    }

    if cli.zscript {
        let input_abs = input.canonicalize().ok()?;
        let search_from = input_abs.parent()?;
        let stop_at = cli
            .stop_at_cwd
            .then(|| std::env::current_dir().ok())
            .flatten();

        return find_cargo_toml(search_from, stop_at.as_deref());
    }

    None
}

fn find_cargo_toml(mut current: &Path, stop_at: Option<&Path>) -> Option<PathBuf> {
    loop {
        let manifest = current.join("Cargo.toml");
        if manifest.exists() {
            return Some(manifest);
        }

        if stop_at.is_some_and(|stop| stop == current) {
            return None;
        }

        current = current.parent()?;
    }
}

fn write_output(path: &Path, code: &str, manifest: Option<&Path>) {
    let content = format_output(code, manifest);
    std::fs::write(path, content).unwrap_or_else(|e| {
        eprintln!("error: failed to write to {}: {e}", path.display());
        std::process::exit(1);
    });
}

fn print_to_stdout(cli: &Cli, code: &str, manifest: Option<&Path>) {
    let code = apply_syntax_highlighting(code, cli.theme.as_deref());
    let output = format_output(&code, manifest);
    print!("{output}");
}

fn format_output(code: &str, manifest: Option<&Path>) -> String {
    match manifest {
        Some(m) => build_cargo_script(m, code),
        None => code.to_string(),
    }
}

fn apply_syntax_highlighting(code: &str, theme: Option<&str>) -> String {
    match theme {
        Some(t) => highlight_code(code, t),
        None => code.to_string(),
    }
}

fn read_manifest(manifest: &Path) -> String {
    std::fs::read_to_string(manifest).unwrap_or_else(|e| {
        eprintln!("error: failed to read manifest: {e}");
        std::process::exit(1);
    })
}

fn highlight_code(code: &str, theme_name: &str) -> String {
    let themes: std::collections::HashMap<_, _> = builtin::all()
        .into_iter()
        .map(|t| (t.name.to_lowercase(), t))
        .collect();

    let theme = themes.get(&theme_name.to_lowercase()).unwrap_or_else(|| {
        eprintln!("error: unknown theme '{theme_name}'");
        eprintln!("hint: use --list-themes to see available themes");
        std::process::exit(1);
    });

    let mut highlighter = AnsiHighlighter::new(theme.clone());
    highlighter
        .highlight("rust", code)
        .unwrap_or_else(|_| code.to_string())
}

fn build_cargo_script(manifest: &Path, code: &str) -> String {
    let manifest_content = read_manifest(manifest);
    let mut script = String::new();

    script.push_str("#!/usr/bin/env -S RUSTC_BOOTSTRAP=1 RUSTFLAGS=-Coverflow-checks cargo run -qZscript --release --manifest-path\n");
    script.push_str("---cargo\n");
    script.push_str(&manifest_content);

    if !manifest_content.ends_with('\n') {
        script.push('\n');
    }

    script.push_str("---\n\n");
    script.push_str(code);
    script
}
