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
    /// Input Rust source file or directory (use "." for current directory)
    input: Option<PathBuf>,

    /// Output file (defaults to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Enable syntax highlighting with specified theme
    /// NOTE: cannot be used together with --output because highlighting writes ANSI escapes which would corrupt output files
    #[arg(short, long, conflicts_with = "output")]
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

    /// Generate cargo-script with empty manifest
    #[arg(short = 'e', long, conflicts_with_all = ["manifest", "zscript"])]
    empty_manifest: bool,
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
    let input_path = cli.input.as_ref().ok_or("<INPUT> is required")?;
    let input = resolve_input_path(input_path)?;

    let code = inline_modules(&input)?;
    let manifest = resolve_manifest(cli, &input)?;
    let output_content = prepare_output(&code, cli.theme.as_deref(), manifest)?;

    if let Some(out_path) = &cli.output {
        std::fs::write(out_path, output_content)?;
    } else {
        print!("{output_content}");
    }

    Ok(())
}

fn resolve_input_path(input: &Path) -> Result<PathBuf> {
    if !input.is_dir() {
        return Ok(input.to_path_buf());
    }

    // If input is a directory, find Cargo.toml and determine entry point
    let manifest_path = input.join("Cargo.toml");
    if !manifest_path.exists() {
        return Err(format!("No Cargo.toml found in directory: {}", input.display()).into());
    }

    let manifest_content = std::fs::read_to_string(&manifest_path)?;
    let entry_point = parse_entry_point(&manifest_content, input)?;

    Ok(entry_point)
}

fn parse_entry_point(manifest_content: &str, base_dir: &Path) -> Result<PathBuf> {
    let manifest: toml::Value = toml::from_str(manifest_content)?;

    // Check for [[bin]] entries first
    if let Some(bins) = manifest.get("bin").and_then(|b| b.as_array())
        && let Some(first_bin) = bins.first()
        && let Some(path) = first_bin.get("path").and_then(|p| p.as_str())
    {
        return Ok(base_dir.join(path));
    }

    // Check for single [bin] entry
    if let Some(bin) = manifest.get("bin").and_then(|b| b.as_table())
        && let Some(path) = bin.get("path").and_then(|p| p.as_str())
    {
        return Ok(base_dir.join(path));
    }

    // Check for lib
    if let Some(lib) = manifest.get("lib") {
        if let Some(path) = lib.get("path").and_then(|p| p.as_str()) {
            return Ok(base_dir.join(path));
        }
        // Default lib path
        let lib_path = base_dir.join("src/lib.rs");
        if lib_path.exists() {
            return Ok(lib_path);
        }
    }

    // Default: check for src/main.rs (binary crate default)
    let main_path = base_dir.join("src/main.rs");
    if main_path.exists() {
        return Ok(main_path);
    }

    // Fallback: check for src/lib.rs
    let lib_path = base_dir.join("src/lib.rs");
    if lib_path.exists() {
        return Ok(lib_path);
    }

    Err(
        "Could not determine entry point from Cargo.toml. No src/main.rs or src/lib.rs found."
            .into(),
    )
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

enum ManifestOption {
    Path(PathBuf),
    Empty,
    None,
}

fn resolve_manifest(cli: &Cli, input: &Path) -> Result<ManifestOption> {
    if cli.empty_manifest {
        return Ok(ManifestOption::Empty);
    }

    if let Some(manifest) = &cli.manifest {
        return Ok(ManifestOption::Path(manifest.clone()));
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

        if let Some(manifest) = find_cargo_toml(search_from, stop_at.as_deref()) {
            return Ok(ManifestOption::Path(manifest));
        }
    }

    Ok(ManifestOption::None)
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

fn prepare_output(code: &str, theme: Option<&str>, manifest: ManifestOption) -> Result<String> {
    let highlighted_code = apply_syntax_highlighting(code, theme)?;
    format_output(&highlighted_code, manifest)
}

fn apply_syntax_highlighting(code: &str, theme: Option<&str>) -> Result<String> {
    match theme {
        Some(t) => highlight_code(code, t),
        None => Ok(code.to_string()),
    }
}

fn format_output(code: &str, manifest: ManifestOption) -> Result<String> {
    match manifest {
        ManifestOption::Path(ref path) => build_cargo_script_with_manifest(path, code),
        ManifestOption::Empty => Ok(build_cargo_script_empty(code)),
        ManifestOption::None => Ok(code.to_string()),
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

fn build_cargo_script_empty(code: &str) -> String {
    let shebang = get_shebang();
    let mut script = String::new();

    script.push_str(&shebang);
    script.push('\n');
    script.push_str("---cargo\n");
    script.push_str("[dependencies]\n");
    script.push_str("---\n\n");
    script.push_str(code);

    script
}

fn build_cargo_script_with_manifest(manifest: &Path, code: &str) -> Result<String> {
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
