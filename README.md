# matterof: markdown front-matter editor

> forked from [cdfmlr/smelt](https://github.com/cdfmlr/smelt). (they are different programs for different purpose: forked for reusing some initial code only)

matterof is a commandline tool in help of reading/editing the YAML front-matters at the head of Markdown files.

## Usage

### Get

```
# get all front-matter key-values, output as YAML
matterof get --all <path/to/file.md>

# get specific key-values
matterof get --key=key <file>

# nested keys with dot notation
matterof get --key=parent.child.key <file>
# notice when a key contains dots, it should be quoted
matterof get --key='"parent.with.dots"."child.key"' <file>
# or use parenthesis notation
matterof get --key='parent["with.dots"]["child.key"]' <file>
# or use a list of --key-part flags
matterof get --key-part=parent.with.dots --key-part=child.key <file>
# --key-part comma,separated is supported too, but not recommended due to ambiguity

# get multiple key-values
matterof get --key=key1,key2,... <file>
# or repeat the flag
matterof get --key=key1 --key=key2 ... <file>

# regular expression to match keys
matterof get --key-regex='^key_prefix_.*' <file>

# to use regex for nested keys, use --key-part-regex flags is highly recommended for better clarity
matterof get --key-part='parent' --key-part-regex='^child_.*' <file>
# or (NOT RECOMMENDED AT ALL) you can still use dot notation or parenthesis notation flags as above with careful escaping
matterof get --key-regex='^parent\.child\..*' <file> # the escaped dot is considered as the key level separator
matterof get --key-regex='^parent\["child"\]\..*' <file>

# regular expression to match values
matterof get --key=key1 --value-regex='^value_prefix_.*' <file>
# when both key-regex and value-regex are provided, only key-values matching both are returned

# multiple files, output as YAML mapping from file names to key-values
matterof get --key=key <file1> <file2> ...
## example output:
## file1.md:
##   key: value
## file2.md:
##   key: value
```

### Set

```
# the value defaults to string
matterof set --key=key --value=value <file>
# nested keys with dot notation, parenthesis notation, or --key-part flags are supported as in 'get' command, creating parent keys if not exist (like mkdir -p)

# specify value type
matterof set --key=key --type=string|int|float|bool --value=value <file>

# add multiple values as a list
matterof set --key=key --value=value1,value2,... <file>
# or repeat the flag
matterof set --key=key --value=value1 --value=value2 ... <file>

# multiple files: set the same key-value pair in all files
matterof set --key=key --value=value <file1> <file2> ...

# multiple keys: set the same value for multiple keys
matterof set --key=key1,key2,... --value=value <file>
# fuzzy key matching with regex
matterof set --key-regex='^key_prefix_.*' --value=value <file>

# append a value to a list/mapping
matterof add --key=key --value=value <file>

# insert a value to a list at specific index (0-based)
matterof add --key=key --index=N --value=value <file>
```

### Remove

```
# remove specific key
matterof rm --key=key <file>
# nested keys with dot notation, parenthesis notation, or --key-part flags are supported as in 'get' command

# remove a value from a list/mapping
matterof rm --key=key --value=value <file>

# multiple files: remove the same key in all files
matterof rm --key=key <file1> <file2> ...

# regex to match keys
matterof rm --key-regex='^key_prefix_.*' <file>
# regex to match values
matterof rm --key=key --value-regex='^value_prefix_.*' <file>
# when both key-regex and value-regex are provided, only key-values matching both are removed

# remove the entire front-matter
matterof rm --all <file>
```

### Replace

```
# rename a key (in the same level)
matterof replace --key=old_key --new-key=new_key <file>
# to rename nested keys, use dot notation, parenthesis notation, or --key-part flags as in 'get' command, the last part is considered as the key name to be changed
matterof replace --key-part=parent --key-part=old_key --new-key=new_key <file>

# moving a key to a different parent with --new-key-part, it creates the new parents if not exist (like mkdir -p)
matterof replace --key-part=old_parent --key-part=old_key --new-key-part=new_parent --new-key=new_key <file>
# or (NOT RECOMMENDED) using dot notation / parenthesis notation with careful double checking:
matterof replace --key=old_parent.old_key --new-key=new_parent.new_key <file>

# replace value for specific key: alias of 'set'
matterof replace --key=key [--type=?] --value=new_value <file>

# replace a value in a list/mapping
matterof replace --key=key --old-value=old_value --new-value=new_value <file>
# comman-separated / repeat-flag multiple old/new values are not supported here

# regex to match keys/values
matterof replace --key-regex='^key_prefix_.*' --old-value-regex='^old_value_prefix_.*' --new-value=new_value <file>
```

### Dry-run, Backup and Output Options

By default, the commands that modify files (set, rm, replace) will directly change the files in-place. You can use the following options for safer operations:

```
# preview changes without modifying files: output a unified diff (diff -u) to stdout
matterof [get|set|rm|replace] --dry-run ...

# create a backup copy of each modified file with suffix:
matterof [set|rm|replace] --backup-suffix='.bak' ...

# create a backup copy of each modified file to a specific directory, preserving the original file names and relative paths
matterof [set|rm|replace] --backup-dir='/path/to/backup/dir' ...

# output the modified content to stdout instead of writing back to the file (only available when modifying a single file)
matterof [set|rm|replace] --stdout ...

# output the modified files to a specific directory, preserving the original file names and relative paths
matterof [set|rm|replace] --output-dir='/path/to/dir' ...
```

### Chore

```
# initialize front-matter if not exists
matterof init <file>

# remove front-matter if empty
matterof clean <file>

# validate front-matter syntax
matterof validate <file>

# format front-matter (sort keys, consistent indentation, etc.)
matterof fmt <file>

# show help
matterof help  # or matterof --help
matterof help <command>  # or matterof <command> --help

# show version
matterof version  # or matterof --version
```

## License

MIT OR Apache-2.0
