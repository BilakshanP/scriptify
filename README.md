# scriptify

Inline Rust modules into a single file, with optional **cargo zscript** emission and **strict output rules**.

This tool refuses to guess. If output would be ambiguous or unsafe, it errors.

---

## Features

- Recursively inlines `mod foo;`
- Formats output with `prettyplease`
- Emits self-contained **cargo zscripts**
- Optional ANSI syntax highlighting (strictly controlled)

---

## Installation

### From GitHub

```bash
cargo install --git https://github.com/bilakshanp/scriptify
```

### From crates.io

```bash
# unplanned
```

## CLI Reference

```txt
scriptify <INPUT> [OUTPUT]

Options:
  -t, --theme <NAME>    Syntax highlight theme
      --no-color        Disable colorization
      --list-themes     List available themes
  -m, --manifest-path   PATH Cargo.toml for zscript
      --auto-zscript    Auto-detect Cargo.toml
```

## Quick Start

### Write inlined Rust to a file

```bash
scriptify src/main.rs out.rs
```

### Generate a runnable zscript

```bash
scriptify src/main.rs script.rs --auto-zscript
chmod +x script.rs
```

### Zscript to stdout (colorized body)

```bash
scriptify src/main.rs --auto-zscript
```

## Important Rules (by design)

- Stdout without zscript is an error
- Output files are never colorized
- Manifest is never colorized
- Only zscript body may be colorized, and only on stdout
- This prevents broken pipes, corrupted files, and accidental ANSI junk.

## Zscript Modes

### Explicit manifest

```bash
scriptify input.rs --manifest-path Cargo.toml
```

### Auto-detect manifest

```bash
scriptify input.rs --auto-zscript
```

This searches upward from the input file.

## Color & Themes

List themes:

```bash
scriptify --list-themes
```

Use a theme (stdout + zscript only):

```bash
scriptify input.rs --auto-zscript --theme Monokai
```

Disable color:

```bash
scriptify input.rs --auto-zscript --no-color
```

## Example Usage

Share a single-file script

```bash
scriptify src/main.rs script.rs --auto-zscript
```

Run immediately

```bash
scriptify src/main.rs --auto-zscript | bash
```

Prepare code for review

```bash
scriptify src/lib.rs out.rs
```
