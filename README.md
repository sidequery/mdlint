# mdlint

A fast markdown linter written in Rust. Validates backlinks, wikilinks, heading anchors, images, and document structure across collections of markdown files.

Built for Obsidian vaults, documentation sites, wikis, and any project where broken links between markdown files are a problem.

## Features

- **Link validation**: standard links, wikilinks (`[[page]]`), heading anchors (`#section`), images, cross-file references
- **Wikilink resolution**: shortest-path matching (like Obsidian), with ambiguity detection and frontmatter alias support
- **Heading anchor validation**: configurable slug generation (GFM transforms `## Hello World` to `#hello-world`; Obsidian mode matches exact heading text case-insensitively)
- **Structural rules**: heading hierarchy, required frontmatter fields, first heading level, orphan page detection
- **Rich diagnostics**: rustc-style error output with source context, or JSON/short formats for CI
- **Fast**: ~58 MB/s throughput, lints 5,000 files in under 350ms

## Install

```
cargo install --path .
```

## Usage

```
# Lint current directory
mdlint

# Lint a specific directory
mdlint ./docs

# JSON output for CI
mdlint --format json

# Errors only, no warnings
mdlint --quiet

# Specific files
mdlint --files doc1.md doc2.md
```

## Example output

```
broken-links

  x link target not found: ./does-not-exist.md
    ,-[index.md:12:3]
 11 | - [Guide](./guide.md)
 12 | - [Missing Page](./does-not-exist.md)
    :   ``````````````````````````````````
 13 | - [Guide Heading](./guide.md#getting-started)
    `----
  help: file `./does-not-exist.md` does not exist relative to `docs/`

heading-increment

  ! heading level skipped: h2 -> h4
    ,-[guide.md:13:1]
 12 |
 13 | #### Deep Heading
    : `````````````````
 14 |
    `----
  help: expected h3 or lower, found h4
```

## Configuration

Create an `mdlint.toml` in your project root (or any parent directory):

```toml
[workspace]
include = ["**/*.md"]
exclude = ["node_modules", "target", ".git"]

[links]
slug_mode = "gfm"                    # "gfm" or "obsidian"
wikilink_resolution = "shortest-path" # "shortest-path" or "relative"
check_external = false
warn_case_mismatch = true

[rules.broken-links]
level = "error"

[rules.heading-increment]
level = "warning"

[rules.require-frontmatter]
level = "error"
fields = ["title"]

[rules.first-heading]
level = "warning"

[rules.orphan-pages]
level = "warning"
exclude = ["index.md", "README.md"]
```

Set any rule to `"off"` to disable it.

## Rules

| Rule | Default | Description |
|------|---------|-------------|
| `broken-links` | error | Validates all internal links, wikilinks, heading anchors, and image references resolve to existing targets |
| `heading-increment` | warning | Heading levels must increment by one (no jumping from h1 to h4) |
| `require-frontmatter` | off | Requires specified YAML frontmatter fields to be present |
| `first-heading` | warning | First heading in a file must be a specific level (default: h1) |
| `orphan-pages` | warning | Flags markdown files that no other file links to. Configurable `exclude` list for entry points (defaults to `index.md`, `README.md`) |

### Link types validated

- Standard links: `[text](path.md)`, `[text](path.md#heading)`
- Wikilinks: `[[page]]`, `[[page|alias]]`, `[[page#heading]]`
- Same-file anchors: `[text](#heading)`
- Images: `![alt](image.png)`
- Frontmatter aliases are resolved for wikilinks

### What it catches

- Broken file references (file doesn't exist)
- Broken heading anchors (heading doesn't exist in target file)
- Ambiguous wikilinks (multiple files match `[[page]]`)
- Case mismatches that would break on case-sensitive filesystems
- Missing images and assets

## Output formats

**Pretty** (default): rich terminal output with source snippets, colors, and help text.

**JSON** (`--format json`): machine-readable array of diagnostics, written to stdout.

```json
[
  {
    "rule": "broken-links",
    "severity": "error",
    "message": "link target not found: ./missing.md",
    "file": "docs/index.md",
    "line": 12,
    "col": 3,
    "help": "file `./missing.md` does not exist relative to `docs/`"
  }
]
```

**Short** (`--format short`): one line per diagnostic, grep-friendly.

```
docs/index.md:12:3: error[broken-links] link target not found: ./missing.md
```

## Performance

Benchmarked on Apple M-series, single crate, using rayon for parallel file parsing:

```
                   small vault      10 files     0.02 MB    0.001s      19 MB/s
                  medium vault     500 files     1.97 MB    0.034s      58 MB/s
                   large vault    5000 files    19.70 MB    0.342s      58 MB/s
                   large files     250 files     7.92 MB    0.477s      17 MB/s
```

Run benchmarks yourself:

```
cargo bench --bench throughput
BENCH_FILES=1000 BENCH_FILE_KB=8 cargo bench --bench throughput
```

## How it works

1. Walks the directory tree (respects `.gitignore`)
2. Parses all markdown files in parallel using [comrak](https://github.com/kivikakk/comrak) (CommonMark + GFM + wikilinks)
3. Builds in-memory indexes: file paths, heading slugs, wikilink basenames, frontmatter aliases
4. Runs file-level rules in parallel (heading structure, frontmatter)
5. Runs workspace-level rules (link resolution against indexes)
6. Reports diagnostics sorted by file and line

All link resolution is done against pre-built indexes with zero filesystem calls at resolve time.

## License

MIT
