# url2md

Converts any web page to clean Markdown. Works as a CLI tool or a Rust library.

Handles JavaScript-heavy sites (X/Twitter, GitHub, SPAs) via headless Chrome, with HTTP fallback for simpler pages.

## CLI

```bash
cargo build --release

# Basic usage
./target/release/html2md https://example.com

# Save to file
./target/release/html2md https://example.com -o output.md

# Skip JS rendering (faster, plain HTTP only)
./target/release/html2md https://example.com --no-js

# Verbose output
./target/release/html2md https://example.com -v
```

## Library

Add to your `Cargo.toml`:

```toml
[dependencies]
html2md = { git = "https://github.com/m1kr0b1/url2md" }
```

Use it:

```rust
use html2md::Converter;

let converter = Converter::new("https://example.com")?;
let markdown = converter.convert(&html_string)?;
```

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--no-js` / `-n` | off | Skip headless browser, use plain HTTP |
| `--timeout` / `-t` | 60s | Request timeout in seconds |
| `--output` / `-o` | stdout | Save output to file |
| `--user-agent` / `-u` | Chrome 134 | Custom User-Agent string |
| `--js-wait-ms` | 3000ms | Extra wait after page load for JS to settle |
| `--verbose` / `-v` | off | Print timing and request details |
