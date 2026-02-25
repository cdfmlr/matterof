# matterof — example / demo directory

This directory contains sample Markdown files and a `Makefile` that let you
explore every feature of the `matterof` CLI hands-on.

## Files

| File | Purpose |
|------|---------|
| `blog-post.md` | A blog-post draft with nested and typed front-matter fields |
| `book-catalog.md` | A book catalog with an array of book objects |
| `no-frontmatter.md` | A plain Markdown file with no front-matter (used to demo `init`) |

## Quick Start

### 1. Build the binary

```bash
make matterof
```

This runs `cargo build` from the parent directory (`..`) and creates a
`./matterof` symlink pointing to `../target/debug/matterof` so you can use
it directly from inside this directory.

### 2. Play around

```bash
# See the full front-matter of a file
./matterof get --all blog-post.md

# Read a nested field
./matterof get --query "author.name" blog-post.md

# Filter books by price
./matterof get --query "books[?@.price > 45].title" book-catalog.md

# Add a tag
./matterof add --query "tags" --value "demo" blog-post.md

# Preview a change without modifying the file
./matterof set --query "published" --value "true" --dry-run blog-post.md
```

Refer to the [top-level README](../README.md) for the full CLI reference.

### 3. Run the e2e test suite

```bash
make test
```

The test target exercises **every** CLI command documented in the README:

- `get` — `--all`, `--query`/`--key`/`--jsonpath`, nested fields, array
  access (`[0]`, `[*]`, `[0:2]`, `[-1]`), filter expressions, recursive
  descent (`$..`), multiple files, `--format yaml|json|internal`
- `set` — simple, nested, array element, bulk filter, `--type int|float|bool`,
  multiple files
- `add` — append, `--index`, `--add-key`
- `remove` — field, nested field, array index, filter, `--all`
- `replace` — `--new-key` (rename), `--old-value`/`--new-value`, bulk filter
- `query` — paths only, `--count`, `--exists`, `--with-values`
- File-safety options — `--dry-run`, `--backup-suffix`, `--backup-dir`,
  `--output-dir`, `--stdout`
- `init`, `validate`, `format`, `clean`

All destructive operations run on **temporary copies** inside a `tmp/`
directory so the original demo files are never modified.

### 4. Clean up

```bash
make clean
```

Removes the `tmp/` directory and the `./matterof` symlink.
