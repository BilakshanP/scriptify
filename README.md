# scriptify

A CLI tool to inline all Rust module declarations into a single file, with optional syntax highlighting and cargo-script support. Inspired by cargo-expand.

## Installation

### From Source

```bash
git clone https://github.com/bilakshanp/scriptify.git
cd scriptify
cargo install --path .
```

Or install directly from the repository:

```bash
cargo install --git https://github.com/bilakshanp/scriptify.git
```

### From crates.io

```txt
# unplanned
```

## Usage

### Basic Usage

Inline modules and print to stdout:

```bash
scriptify src/lib.rs
```

Save to a file:

```bash
scriptify src/lib.rs -o output.rs
```

### Syntax Highlighting

Enable syntax highlighting with a theme:

```bash
scriptify src/lib.rs -t monokai
scriptify src/lib.rs --theme dracula
```

List all available themes:

```bash
scriptify --list-themes
```

### Cargo Script Generation

Generate script with an empty manifest:

```bash
scriptify file.rs -e
```

Automatically convert a whole project into a script:

```bash
scriptify . -z
```

Note: `CWD` should contain `Cargo.toml` file.

Generate a cargo-script (RFC 3424) with auto-discovered `Cargo.toml`:

```bash
scriptify src/lib.rs -z -o script.rs
```

Specify a custom manifest path:

```bash
scriptify src/lib.rs -m path/to/Cargo.toml -o script.rs
```

The generated script will have this structure:

```rust
#!/usr/bin/env -S RUSTC_BOOTSTRAP=1 RUSTFLAGS=-Coverflow-checks cargo run -qZscript --release --manifest-path
---cargo
[dependencies]
serde = "1.0"
---

// Your inlined code here
```

Make it executable and run:

```bash
chmod +x script.rs
./script.rs
```

### Advanced Options

Stop manifest search at current working directory:

```bash
scriptify src/lib.rs -z --stop-at-cwd
```

## Examples

### Example 1: Simple Module Inlining

Given this structure:

```txt
src/
├── lib.rs
├── foo.rs
└── bar.rs
```

Where `lib.rs` contains:

```rust
mod foo;
mod bar;

pub fn main() {
    foo::hello();
    bar::world();
}
```

Running:

```bash
scriptify src/lib.rs -o single.rs
```

Produces a single file with all modules inlined.

Output:

```rust
mod foo {
    pub fn hello();
}
mod bar {
    pub fn world();
}
pub fn main() {
    foo::hello();
    bar::world();
}
```

### Example 2: Create a Portable Script

```bash
# Create a self-contained script with dependencies
scriptify src/main.rs -z > script.rs
chmod +x script.rs

# Share it with others - no cargo project needed!
./script.rs
```

### Example 3: Code Review with Syntax Highlighting

```bash
# Generate a beautifully highlighted version for review
scriptify src/lib.rs -t github-light | less -R
```

### Example $: Customized SHEBANG

```bash
# Modify the shebang paramaters
SCRIPTIFY_SHEBANG="#!/usr/bin/env -S cargo +nightly -zScript" scriptify file.rs
```

## Available Themes

Run `scriptify --list-themes` for the complete list.

## Command-Line Options

```txt
Usage: scriptify [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input Rust source file

Options:
  -o, --output <OUTPUT>      Output file (defaults to stdout)
  -t, --theme <THEME>        Enable syntax highlighting with specified theme
      --list-themes          List all available themes
  -m, --manifest <MANIFEST>  Path to Cargo.toml for cargo-script generation
  -z, --zscript              Auto-discover Cargo.toml from input file location
      --stop-at-cwd          Stop searching for Cargo.toml at current working directory
  -h, --help                 Print help
  -V, --version              Print version
```
