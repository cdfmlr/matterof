MATTEROF      := ./matterof
BINARY_PATH   := ../target/debug/matterof
CARGO_MANIFEST := ../Cargo.toml
TMP           := tmp

.PHONY: matterof test clean

# ---------------------------------------------------------------------------
# Build matterof from the parent directory and create a local symlink.
# ---------------------------------------------------------------------------
matterof:
	cargo build --manifest-path $(CARGO_MANIFEST)
	ln -sf $(BINARY_PATH) $(MATTEROF)

# ---------------------------------------------------------------------------
# End-to-end test: exercises every CLI feature documented in the README.
# All destructive operations run on temporary copies inside $(TMP)/ so the
# original demo files remain untouched between runs.
# ---------------------------------------------------------------------------
test: matterof
	@echo ""
	@echo "================================================================="
	@echo " matterof e2e test"
	@echo "================================================================="

	# -- Setup: fresh copies of demo files ----------------------------------
	@mkdir -p $(TMP)
	cp -f blog-post.md     $(TMP)/blog-post.md
	cp -f book-catalog.md  $(TMP)/book-catalog.md
	cp -f no-frontmatter.md $(TMP)/no-frontmatter.md
	cp -f blog-post.md     $(TMP)/blog-post2.md
	cp -f book-catalog.md  $(TMP)/book-catalog2.md

	# -----------------------------------------------------------------------
	@echo ""
	@echo "--> init / validate / format / clean"
	# -----------------------------------------------------------------------

	# init: add empty front matter to a plain markdown file
	$(MATTEROF) init $(TMP)/no-frontmatter.md

	# validate: file now has front matter
	$(MATTEROF) validate $(TMP)/no-frontmatter.md

	# validate: original demo files
	$(MATTEROF) validate $(TMP)/blog-post.md
	$(MATTEROF) validate $(TMP)/book-catalog.md

	# format: normalise front-matter whitespace/ordering
	$(MATTEROF) format $(TMP)/blog-post.md
	$(MATTEROF) format $(TMP)/book-catalog.md

	# -----------------------------------------------------------------------
	@echo ""
	@echo "--> get"
	# -----------------------------------------------------------------------

	# get --all: dump entire front matter
	$(MATTEROF) get --all $(TMP)/blog-post.md

	# get --query: simple field
	$(MATTEROF) get --query "title"        $(TMP)/blog-post.md
	# --key is an alias for --query
	$(MATTEROF) get --key  "author.name"   $(TMP)/blog-post.md
	# --jsonpath: explicit JSONPath syntax (use $$ in Makefile for literal $)
	$(MATTEROF) get --jsonpath "$$.author.email" $(TMP)/blog-post.md

	# nested object
	$(MATTEROF) get --query "author.name"  $(TMP)/blog-post.md

	# array: first element, all elements, slice
	$(MATTEROF) get --query "tags[0]"      $(TMP)/blog-post.md
	$(MATTEROF) get --query "tags[*]"      $(TMP)/blog-post.md
	$(MATTEROF) get --query "tags[0:2]"    $(TMP)/blog-post.md

	# array: last element
	$(MATTEROF) get --query "tags[-1]"     $(TMP)/blog-post.md

	# advanced filter: published books
	$(MATTEROF) get --query "books[?@.published]" $(TMP)/book-catalog.md

	# advanced filter: price comparison
	$(MATTEROF) get --query "books[?@.price > 45].title" $(TMP)/book-catalog.md

	# advanced filter: text search
	$(MATTEROF) get --query "books[?search(@.author, 'Steve')]" $(TMP)/book-catalog.md

	# recursive descent
	$(MATTEROF) get --query "$$..author"   $(TMP)/blog-post.md

	# multiple files (output as YAML mapping)
	$(MATTEROF) get --query "title" $(TMP)/blog-post.md $(TMP)/book-catalog.md

	# output formats: yaml (default), json, internal
	$(MATTEROF) get --query "tags[*]" --format yaml     $(TMP)/blog-post.md
	$(MATTEROF) get --query "tags[*]" --format json     $(TMP)/blog-post.md
	$(MATTEROF) get --query "tags[*]" --format internal $(TMP)/blog-post.md

	# -----------------------------------------------------------------------
	@echo ""
	@echo "--> set"
	# -----------------------------------------------------------------------
	cp -f blog-post.md    $(TMP)/blog-post.md
	cp -f book-catalog.md $(TMP)/book-catalog.md

	# simple assignment
	$(MATTEROF) set --query "title" --value "Updated: Getting Started with Rust" $(TMP)/blog-post.md
	$(MATTEROF) get --query "title" $(TMP)/blog-post.md

	# nested object creation
	$(MATTEROF) set --query "author.twitter" --value "@janedoe" $(TMP)/blog-post.md
	$(MATTEROF) get --query "author.twitter" $(TMP)/blog-post.md

	# array element
	$(MATTEROF) set --query "tags[0]" --value "systems" $(TMP)/blog-post.md
	$(MATTEROF) get --query "tags[0]" $(TMP)/blog-post.md

	# bulk operation
	$(MATTEROF) set --query "books[*].category" --value "computer-science" $(TMP)/book-catalog.md
	$(MATTEROF) get --query "books[0].category" $(TMP)/book-catalog.md

	# type specification: int, float, bool
	$(MATTEROF) set --query "views"   --value "42"    --type int   $(TMP)/blog-post.md
	$(MATTEROF) set --query "version" --value "1.5"   --type float $(TMP)/blog-post.md
	$(MATTEROF) set --query "published" --value "true" --type bool $(TMP)/blog-post.md
	$(MATTEROF) get --query "views"   $(TMP)/blog-post.md
	$(MATTEROF) get --query "version" $(TMP)/blog-post.md
	$(MATTEROF) get --query "published" $(TMP)/blog-post.md

	# multiple files
	$(MATTEROF) set --query "reviewed" --value "true" $(TMP)/blog-post.md $(TMP)/blog-post2.md
	$(MATTEROF) get --query "reviewed" $(TMP)/blog-post.md
	$(MATTEROF) get --query "reviewed" $(TMP)/blog-post2.md

	# -----------------------------------------------------------------------
	@echo ""
	@echo "--> add"
	# -----------------------------------------------------------------------
	cp -f blog-post.md $(TMP)/blog-post.md

	# append to array
	$(MATTEROF) add --query "tags" --value "new-tag" $(TMP)/blog-post.md
	$(MATTEROF) get --query "tags" $(TMP)/blog-post.md

	# insert at specific position
	$(MATTEROF) add --query "tags" --value "first-tag" --index 0 $(TMP)/blog-post.md
	$(MATTEROF) get --query "tags[0]" $(TMP)/blog-post.md

	# add a new key to an object
	$(MATTEROF) add --query "author" --add-key "website" --value "https://example.com" $(TMP)/blog-post.md
	$(MATTEROF) get --query "author.website" $(TMP)/blog-post.md

	# -----------------------------------------------------------------------
	@echo ""
	@echo "--> remove"
	# -----------------------------------------------------------------------
	cp -f blog-post.md    $(TMP)/blog-post.md
	cp -f book-catalog.md $(TMP)/book-catalog.md

	# remove a top-level field
	$(MATTEROF) remove --query "draft" $(TMP)/blog-post.md

	# remove a nested field
	$(MATTEROF) remove --query "author.email" $(TMP)/blog-post.md

	# remove an array element by index
	$(MATTEROF) remove --query "tags[0]" $(TMP)/blog-post.md
	$(MATTEROF) get --query "tags" $(TMP)/blog-post.md

	# remove array elements matching a filter
	$(MATTEROF) remove --query "books[?@.draft]" $(TMP)/book-catalog.md
	$(MATTEROF) get --query "books[*].title" $(TMP)/book-catalog.md

	# remove entire front matter
	cp -f no-frontmatter.md $(TMP)/no-frontmatter.md
	$(MATTEROF) init $(TMP)/no-frontmatter.md
	$(MATTEROF) remove --all $(TMP)/no-frontmatter.md

	# -----------------------------------------------------------------------
	@echo ""
	@echo "--> replace"
	# -----------------------------------------------------------------------
	cp -f blog-post.md    $(TMP)/blog-post.md
	cp -f book-catalog.md $(TMP)/book-catalog.md

	# rename a key
	$(MATTEROF) replace --query "version" --new-key "revision" $(TMP)/blog-post.md
	$(MATTEROF) get --query "revision" $(TMP)/blog-post.md

	# rename a nested key
	$(MATTEROF) replace --query "author.email" --new-key "contact" $(TMP)/blog-post.md
	$(MATTEROF) get --query "author.contact" $(TMP)/blog-post.md

	# replace a specific value
	$(MATTEROF) replace --query "published" --old-value "false" --new-value "true" $(TMP)/blog-post.md
	$(MATTEROF) get --query "published" $(TMP)/blog-post.md

	# bulk replace with filter
	$(MATTEROF) replace --query "books[?@.published == true].category" --new-value "tech" $(TMP)/book-catalog.md
	$(MATTEROF) get --query "books[*].category" $(TMP)/book-catalog.md

	# -----------------------------------------------------------------------
	@echo ""
	@echo "--> query (path analysis)"
	# -----------------------------------------------------------------------
	cp -f book-catalog.md $(TMP)/book-catalog.md

	# list normalized paths matching a query
	$(MATTEROF) query --query "books[*].author" $(TMP)/book-catalog.md
	# --key and --jsonpath are aliases
	$(MATTEROF) query --key      "books[*].title"  $(TMP)/book-catalog.md
	$(MATTEROF) query --jsonpath "books[*].price"  $(TMP)/book-catalog.md

	# count matches
	$(MATTEROF) query --count --query "books[?@.published]" $(TMP)/book-catalog.md

	# check existence (should exit 0)
	$(MATTEROF) query --exists --query "catalog"          $(TMP)/book-catalog.md
	$(MATTEROF) query --exists --query "books[0].author"  $(TMP)/book-catalog.md

	# check non-existence (should exit 1, use negation)
	! $(MATTEROF) query --exists --query "nonexistent_key" $(TMP)/book-catalog.md

	# show matching paths alongside their values
	$(MATTEROF) query --with-values --query "tags[*]" $(TMP)/book-catalog.md

	# -----------------------------------------------------------------------
	@echo ""
	@echo "--> file safety options"
	# -----------------------------------------------------------------------
	cp -f blog-post.md $(TMP)/blog-post.md

	# dry-run: preview changes without modifying the file
	$(MATTEROF) set --query "title" --value "Dry Run Title" --dry-run $(TMP)/blog-post.md
	# original title should be unchanged
	$(MATTEROF) get --query "title" $(TMP)/blog-post.md | grep -q "Getting Started"

	# backup with a suffix
	$(MATTEROF) set --query "title" --value "Backed Up Title" --backup-suffix ".bak" $(TMP)/blog-post.md
	test -f $(TMP)/blog-post.md.bak

	# backup to a directory
	mkdir -p $(TMP)/backups
	cp -f blog-post.md $(TMP)/blog-post.md
	$(MATTEROF) set --query "title" --value "Backup Dir Title" --backup-dir $(TMP)/backups $(TMP)/blog-post.md
	test -n "$$(ls $(TMP)/backups/)"

	# output to a different directory
	mkdir -p $(TMP)/modified
	cp -f blog-post.md $(TMP)/blog-post.md
	$(MATTEROF) set --query "title" --value "Output Dir Title" --output-dir $(TMP)/modified $(TMP)/blog-post.md
	test -f $(TMP)/modified/blog-post.md

	# stdout: write result to stdout instead of file
	cp -f blog-post.md $(TMP)/blog-post.md
	$(MATTEROF) set --query "title" --value "Stdout Title" --stdout $(TMP)/blog-post.md | grep -q "Stdout Title"

	# -----------------------------------------------------------------------
	@echo ""
	@echo "--> clean (remove empty front matter)"
	# -----------------------------------------------------------------------
	# init creates empty front matter; clean should remove it
	cp -f no-frontmatter.md $(TMP)/no-frontmatter.md
	$(MATTEROF) init  $(TMP)/no-frontmatter.md
	$(MATTEROF) clean $(TMP)/no-frontmatter.md

	# -----------------------------------------------------------------------
	@echo ""
	@echo "================================================================="
	@echo " All tests passed!"
	@echo "================================================================="

# ---------------------------------------------------------------------------
# Remove the local symlink and temporary test artefacts.
# ---------------------------------------------------------------------------
clean:
	rm -rf $(TMP)
	rm -f  $(MATTEROF)
