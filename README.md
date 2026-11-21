# AnyConvert

A fast, lightweight, **fully offline** file-format converter. One Rust conversion engine powers two front-ends:

| Binary | Description |
| --- | --- |
| `anyconvert` | Command-line tool for single files and batches |
| `anyconvert-gui` | Native drag-and-drop GUI (egui/eframe) |

Everything runs locally. No uploads, no external executables, no network calls at runtime.

## What it converts

| Category | Read | Write |
| --- | --- | --- |
| Images | PNG, JPEG, GIF, WebP, BMP, TIFF, ICO, PNM/PPM, TGA, QOI, HDR, EXR, DDS, farbfeld | PNG, JPEG, WebP, GIF, BMP, TIFF, ICO, PNM/PPM, TGA, QOI, HDR, EXR |
| Structured data | JSON, YAML, TOML, CSV, XML, RON, JSON5, INI | JSON, YAML, TOML, CSV, XML, RON, JSON5, INI |
| Text / markup | TXT, Markdown, HTML | TXT, HTML, Markdown |
| Audio | MP1/2/3, WAV, FLAC, Ogg, Opus, AAC, M4A, AIFF | 16-bit PCM WAV |
| Documents | PDF | PDF (from text, structured data, or images) |

Notes:

- Structured formats round-trip through a `serde_json::Value` hub, so all pairs are mutually convertible.
- Audio decoding uses Symphonia; output is always interoperable 16-bit PCM WAV.
- Image output uses the pure-Rust `image` crate. Animated inputs are flattened to their first frame.
- Generated PDFs contain either extracted text or a full-page embedded JPEG.

Use `anyconvert --list-formats`, `anyconvert --targets png`, or `anyconvert --list-conversions` for the exact, machine-generated capability tables.

## Build

```sh
cargo build --release
```

Binaries are written to `target/release/`.

### Windows toolchain note

A machine-specific `.cargo/config.toml` is intentionally not committed. If your Windows GNU toolchain needs it, copy `.cargo/config.example.toml` to `.cargo/config.toml` and adjust the two paths.

The example points rustc at LLVM's `dlltool.exe` because Windows raw-dylib symbols require an import-library generator. It also enables `link-self-contained=yes` and links with Rust's bundled `rust-lld.exe`, avoiding GCC startup objects that can crash on some MinGW installations. If a normal MinGW/MSVC toolchain works for you, no config file is needed. Linux builds do not use it.

The CLI also disables clap's optional `color` feature: on this toolchain, `anstyle-wincon`'s raw-dylib import library could crash at startup. All other clap functionality is unchanged.

## CLI usage

```sh
# One file
anyconvert photo.png --to webp

# Batch, into a separate folder, with explicit quality
anyconvert *.png --to jpg --out-dir out --quality 90

# Override source detection (extensionless files)
anyconvert data --from json --to yaml

# Discover capabilities
anyconvert --list-formats
anyconvert --targets md
anyconvert --list-conversions

# Machine-readable result summary
anyconvert a.png b.jpg --to webp --json
```

Options:

- `-t, --to <EXT>` — target extension (required for conversion)
- `-o, --out-dir <DIR>` — output directory (default: beside each input)
- `--from <EXT>` — source extension override
- `-q, --quality <N>` — JPEG/WebP quality, 1–100 (default 85)
- `--overwrite` — allow replacing existing outputs
- `-Q, --quiet` — suppress progress output
- `--list-formats`, `--list-conversions`, `--targets <EXT>` — capability browser
- `--json` — emit a JSON summary

The process exits non-zero if any file fails to convert.

## GUI usage

Launch `anyconvert-gui`, then add files with **Add files…** / **Add folder…** or drag and drop them onto the window. Pick a target format (the dropdown is filtered to formats compatible with every selected file), optionally choose an output folder and quality, then click **Convert**. The **Supported formats** tab browses every `from → to` pair. The conversion itself is synchronous and local; no data leaves the machine.

## Project layout

```
core/  conversion engine, shared by both front-ends
cli/   anyconvert binary
gui/   anyconvert-gui binary
```

Run the backend test suite with:

```sh
cargo test -p anyconvert-core --test roundtrip
```
