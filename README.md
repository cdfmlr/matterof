# matterof: markdown front-matter editor

> forked from [cdfmlr/smelt](https://github.com/cdfmlr/smelt). (they are different programs for different purpose: forked for reusing some initial code only)

matterof is a Rust library and commandline tool for reading and editing YAML front-matter in Markdown files using JSONPath queries.

## Core Library API

The core library provides a type-safe API for working with front-matter:

```rust
use matterof::{FrontMatterReader, FrontMatterWriter, FrontMatterValue, KeyPath};

// Load a document
let reader = FrontMatterReader::new();
let mut doc = reader.read_file("example.md")?;

// Read a value by key path
let title = doc.get(&KeyPath::parse("title")?);
let author_name = doc.get(&KeyPath::parse("author.name")?);

// Modify values
doc.set(&KeyPath::parse("title")?, FrontMatterValue::string("New Title"))?;
doc.add_to_array(&KeyPath::parse("tags")?, FrontMatterValue::string("rust"), None)?;

// Save changes
let writer = FrontMatterWriter::new();
writer.write_file(&doc, "example.md", None)?;
```

For JSONPath-based queries, use `matterof::JsonPathQuery` and `matterof::YamlJsonConverter`. The CLI tool exposes the full JSONPath interface.

## CLI Usage

All commands use [JSONPath](https://tools.ietf.org/rfc/rfc9535.txt) syntax for powerful querying and modification. Simple paths are automatically prefixed with `$.` for convenience.

### Get

```bash
# Get all front-matter
matterof get --all file.md

# Simple field access (auto-prepends "$.")
matterof get --query "title" file.md
matterof get --key "title" file.md              # alias for --query
matterof get --jsonpath "title" file.md         # explicit JSONPath syntax

# Nested object access
matterof get --query "author.name" file.md

# Array access
matterof get --query "tags[0]" file.md          # first tag
matterof get --query "tags[*]" file.md          # all tags
matterof get --query "tags[1:3]" file.md        # slice: tags 1-2

# Advanced filtering
matterof get --query "posts[?@.published]" file.md                    # published posts
matterof get --query "books[?@.price > 10].title" file.md             # expensive book titles
matterof get --query "authors[?search(@.name, 'John')]" file.md       # authors named John

# Recursive search
matterof get --query "$..author" file.md        # all "author" fields recursively

# Multiple files (output as YAML mapping)
matterof get --query "title" file1.md file2.md

# Output formats
matterof get --query "tags[*]" --format yaml file.md      # default YAML
matterof get --query "tags[*]" --format json file.md      # JSON array
matterof get --query "tags[*]" --format internal file.md  # Normalized Paths (RFC 9535 §2.7): path: value
```

### Set

```bash
# Simple assignment
matterof set --query "title" --value "New Title" file.md

# Nested object creation (creates parents as needed)
matterof set --query "author.name" --value "John Doe" file.md

# Array operations
matterof set --query "tags[0]" --value "rust" file.md

# Bulk operations (sets ALL matches)
matterof set --query "posts[*].published" --value true file.md
matterof set --query "books[?@.draft].status" --value "review" file.md

# Type specification
matterof set --query "count" --value "42" --type int file.md
matterof set --query "price" --value "19.99" --type float file.md
matterof set --query "enabled" --value "true" --type bool file.md

# Multiple files
matterof set --query "version" --value "2.0" file1.md file2.md
```

### Add

```bash
# Append to arrays
matterof add --query "tags" --value "new-tag" file.md

# Insert at specific position
matterof add --query "tags" --value "first-tag" --index 0 file.md

# Add a new key to an object
matterof add --query "author" --add-key "email" --value "john@example.com" file.md
```

### Remove

```bash
# Remove fields
matterof remove --query "draft" file.md
matterof remove --query "author.email" file.md

# Remove array elements
matterof remove --query "tags[0]" file.md           # remove first tag
matterof remove --query "tags[?@ == 'draft']" file.md    # remove "draft" tags

# Bulk removal
matterof remove --query "posts[?@.archived]" file.md     # remove archived posts

# Remove entire front-matter
matterof remove --all file.md
```

### Replace

```bash
# Rename keys (only works when JSONPath matches exactly one location)
matterof replace --query "old_key" --new-key "new_key" file.md
matterof replace --query "author.old_field" --new-key "new_field" file.md

# Replace values
matterof replace --query "status" --old-value "draft" --new-value "published" file.md

# Bulk replace with filtering
matterof replace --query "posts[?@.status == 'draft'].status" --new-value "review" file.md
```

### Query Analysis

```bash
# Show matching Normalized Paths only (RFC 9535 §2.7, useful for scripting)
matterof query --query "books[*].author" file.md
matterof query --key "books[*].author" file.md        # alias
matterof query --jsonpath "books[*].author" file.md   # explicit
# Output:
# $['books'][0]['author']
# $['books'][1]['author']

# Count matches
matterof query --count --query "posts[?@.published]" file.md
# Output: 5

# Check existence
matterof query --exists --query "author.email" file.md
# Exit code: 0 if exists, 1 if not

# Show query results with Normalized Paths (RFC 9535 §2.7)
matterof query --with-values --query "tags[*]" file.md
# Output:
# $['tags'][0]: rust
# $['tags'][1]: cli
```

### File Safety Options

```bash
# Preview changes (show diff without modifying)
matterof set --query "title" --value "New" --dry-run file.md

# Create backups
matterof set --query "title" --value "New" --backup-suffix ".bak" file.md
matterof set --query "title" --value "New" --backup-dir "./backups" file.md

# Output to different location
matterof set --query "title" --value "New" --output-dir "./modified" file.md
matterof set --query "title" --value "New" --stdout file.md    # single file only

# Atomic operations (default: true)
matterof set --query "title" --value "New" --no-atomic file.md
```

### Utility Commands

```bash
# Initialize empty front-matter
matterof init file.md

# Clean empty front-matter
matterof clean file.md

# Validate syntax
matterof validate file.md

# Format/prettify front-matter
matterof format file.md

# Help
matterof help
matterof help get
matterof get --help
```

## JSONPath Syntax Quick Reference

| Pattern | Description | Example |
|---------|-------------|---------|
| `title` | Root field (auto: `$.title`) | `"Hello World"` |
| `author.name` | Nested field | `"John Doe"` |
| `tags[0]` | Array index | `"rust"` |
| `tags[*]` | All array elements | `["rust", "cli"]` |
| `tags[1:3]` | Array slice | `["cli", "yaml"]` |
| `tags[-1]` | Last element | `"yaml"` |
| `books[?@.published]` | Filter by condition | Published books |
| `books[?@.price > 10]` | Numeric filter | Expensive books |
| `authors[?search(@.name, 'John')]` | Text search | Authors with "John" |
| `$..author` | Recursive search | All author fields |

For complete JSONPath syntax, see [RFC 9535](https://tools.ietf.org/rfc/rfc9535.txt).

## Installation

```bash
# From source
cargo install --git https://github.com/cdfmlr/matterof

# From crates.io (coming soon)
cargo install matterof
```

## Examples

### Blog Post Management

```bash
# Set all drafts to published
matterof set --query "posts[?@.status == 'draft'].status" --value "published" *.md

# Add publication date to all posts missing it
matterof set --query "posts[?!@.date].date" --value "2024-01-01" *.md

# Get all post titles
matterof get --query "posts[*].title" --format json blog/*.md
```

### Book Catalog

```bash
# Find expensive programming books
matterof get --query "books[?@.category == 'programming' && @.price > 50]" catalog.md

# Add ISBN to books that don't have one
matterof set --query "books[?!@.isbn].isbn" --value "TBD" catalog.md
```

## License

MIT OR Apache-2.0
