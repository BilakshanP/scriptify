use arborium::{AnsiHighlighter, theme::builtin};
use clap::Parser;
use std::path::{Path, PathBuf};
use syn_inline_mod::InlinerBuilder;

const DEFAULT_SHEBANG: &str = "#!/usr/bin/env -S RUSTC_BOOTSTRAP=1 RUSTFLAGS=-Coverflow-checks cargo run -qZscript --release --manifest-path";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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

    let result = run(&cli);

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<()> {
    let input = cli.input.as_ref().ok_or("<INPUT> is required")?;

    let code = inline_modules(input)?;
    let manifest = resolve_manifest(cli, input)?;
    let output_content = prepare_output(&code, cli.theme.as_deref(), manifest.as_deref())?;

    if let Some(out_path) = &cli.output {
        std::fs::write(out_path, output_content)?;
    } else {
        print!("{output_content}");
    }

    Ok(())
}

fn list_themes() {
    println!("Available themes:");
    for theme in builtin::all() {
        println!("  {}", theme.name);
    }
}

fn inline_modules(input: &Path) -> Result<String> {
    let result = InlinerBuilder::default().parse_and_inline_modules(input)?;

    Ok(prettyplease::unparse(result.output()))
}

fn resolve_manifest(cli: &Cli, input: &Path) -> Result<Option<PathBuf>> {
    if let Some(manifest) = &cli.manifest {
        return Ok(Some(manifest.clone()));
    }

    if cli.zscript {
        let input_abs = input.canonicalize()?;
        let search_from = input_abs
            .parent()
            .ok_or("Input file has no parent directory")?;
        let stop_at = cli
            .stop_at_cwd
            .then(|| std::env::current_dir().ok())
            .flatten();

        return Ok(find_cargo_toml(search_from, stop_at.as_deref()));
    }

    Ok(None)
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

fn prepare_output(code: &str, theme: Option<&str>, manifest: Option<&Path>) -> Result<String> {
    let highlighted_code = apply_syntax_highlighting(code, theme)?;
    format_output(&highlighted_code, manifest)
}

fn apply_syntax_highlighting(code: &str, theme: Option<&str>) -> Result<String> {
    match theme {
        Some(t) => highlight_code(code, t),
        None => Ok(code.to_string()),
    }
}

fn format_output(code: &str, manifest: Option<&Path>) -> Result<String> {
    match manifest {
        Some(m) => build_cargo_script(m, code),
        None => Ok(code.to_string()),
    }
}

fn read_manifest(manifest: &Path) -> Result<String> {
    Ok(std::fs::read_to_string(manifest)?)
}

fn highlight_code(code: &str, theme_name: &str) -> Result<String> {
    let themes: std::collections::HashMap<_, _> = builtin::all()
        .into_iter()
        .map(|t| (t.name.to_lowercase(), t))
        .collect();

    let theme = themes.get(&theme_name.to_lowercase()).ok_or_else(|| {
        format!("unknown theme '{theme_name}'. Use --list-themes to see available themes")
    })?;

    let mut highlighter = AnsiHighlighter::new(theme.clone());
    Ok(highlighter
        .highlight("rust", code)
        .unwrap_or_else(|_| code.to_string()))
}

fn get_shebang() -> String {
    std::env::var("SCRIPTIFY_SHEBANG").unwrap_or_else(|_| DEFAULT_SHEBANG.to_string())
}

fn build_cargo_script(manifest: &Path, code: &str) -> Result<String> {
    let manifest_content = read_manifest(manifest)?;
    let shebang = get_shebang();
    let mut script = String::new();

    script.push_str(&shebang);
    script.push('\n');
    script.push_str("---cargo\n");
    script.push_str(&manifest_content);

    if !manifest_content.ends_with('\n') {
        script.push('\n');
    }

    script.push_str("---\n\n");
    script.push_str(code);

    Ok(script)
}
