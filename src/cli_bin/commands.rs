//! CLI command handlers that bridge CLI arguments to library operations
//!
//! This module contains the implementation of all CLI commands, providing
//! a clean separation between CLI argument parsing and core library operations.

use crate::cli_bin::args::*;
use log::{debug, info, warn};
use matterof::core::{
    Document, FrontMatterValue, JsonMutator, JsonPathQuery, JsonPathQueryResult, KeyPath, Query,
    YamlJsonConverter,
};
use matterof::error::{MatterOfError, Result};
use matterof::io::{
    BackupOptions, FileResolver, FrontMatterReader, FrontMatterWriter, OutputOptions, ReaderConfig,
    ResolverConfig, WriteOptions as LibWriteOptions, WriterConfig,
};

use std::collections::BTreeMap;

/// Execute the get command
pub fn get_command(args: GetArgs) -> Result<()> {
    debug!("Executing get command with args: {:?}", args);

    let files = resolve_files(&args.files)?;
    if files.is_empty() {
        warn!("No files found to process");
        return Ok(());
    }

    let reader = create_reader(&args.files)?;
    let mut results = BTreeMap::new();

    for file in &files {
        debug!("Processing file: {}", file.display());

        let document = reader.read_file(file)?;

        if args.all {
            // Get all front matter
            if let Some(front_matter) = document.front_matter() {
                let yaml_value = YamlJsonConverter::document_front_matter_to_yaml(front_matter);
                if files.len() == 1 {
                    output_yaml_value(&yaml_value, &args.format, args.pretty)?;
                    return Ok(());
                } else {
                    results.insert(file.to_string_lossy().to_string(), yaml_value);
                }
            }
        } else if let Some(query_str) = &args.query {
            // Use JSONPath query
            let jsonpath_query = if args.no_auto_root {
                JsonPathQuery::new_with_options(query_str, false)?
            } else {
                JsonPathQuery::new(query_str)?
            };

            if let Some(front_matter) = document.front_matter() {
                let yaml_value = YamlJsonConverter::document_front_matter_to_yaml(front_matter);
                let json_value = YamlJsonConverter::yaml_to_json(&yaml_value)?;
                let located_results = jsonpath_query.query_located(&json_value);
                let matches: Vec<_> = located_results
                    .into_iter()
                    .map(|(path, value)| (path, value.clone()))
                    .collect();

                let query_result = JsonPathQueryResult::new(jsonpath_query.clone(), matches);

                if !query_result.is_empty() {
                    if files.len() == 1 {
                        output_jsonpath_result(&query_result, &args.format, args.pretty)?;
                        return Ok(());
                    } else {
                        results.insert(file.to_string_lossy().to_string(), query_result.to_yaml()?);
                    }
                }
            }
        } else {
            return Err(MatterOfError::validation(
                "Either --all or --query must be specified".to_string(),
            ));
        }
    }

    // Output results for multiple files
    if !results.is_empty() {
        output_multiple_yaml_results(&results, &args.format, args.pretty)?;
    } else {
        info!("No matching values found");
    }

    Ok(())
}

/// Execute the set command
pub fn set_command(args: SetArgs) -> Result<()> {
    debug!("Executing set command");

    let files = resolve_files(&args.files)?;
    if files.is_empty() {
        warn!("No files found to process");
        return Ok(());
    }

    let reader = create_reader(&args.files)?;
    let writer = create_writer(&args.write_options)?;
    let write_options = create_write_options(&args.write_options)?;

    // Parse value
    let value = parse_cli_value(&args.value, args.type_.map(Into::into).as_ref())?;

    // Create JSONPath query
    let jsonpath_query = if args.no_auto_root {
        JsonPathQuery::new_with_options(&args.query, false)?
    } else {
        JsonPathQuery::new(&args.query)?
    };

    let mut processed_count = 0;

    for file in files {
        debug!("Processing file: {}", file.display());

        let mut document = if file.exists() {
            reader.read_file(&file)?
        } else {
            Document::empty()
        };

        let modified = set_jsonpath_value(&mut document, &jsonpath_query, &value)?;

        if modified {
            let result = writer.write_file(&document, &file, Some(write_options.clone()))?;
            if result.modified {
                processed_count += 1;
                info!("Updated: {}", file.display());

                if let Some(diff) = result.diff {
                    if args.write_options.dry_run {
                        println!("{}", diff);
                    }
                }
            }
        }
    }

    info!("Processed {} files", processed_count);
    Ok(())
}

/// Execute the query command
pub fn query_command(args: QueryArgs) -> Result<()> {
    debug!("Executing query command with args: {:?}", args);

    let files = resolve_files(&args.files)?;
    if files.is_empty() {
        warn!("No files found to process");
        return Ok(());
    }

    let reader = create_reader(&args.files)?;

    // Create JSONPath query
    let jsonpath_query = if args.no_auto_root {
        JsonPathQuery::new_with_options(&args.query, false)?
    } else {
        JsonPathQuery::new(&args.query)?
    };

    let mut total_matches = 0;
    let mut any_matches = false;

    for file in &files {
        debug!("Processing file: {}", file.display());

        let document = reader.read_file(file)?;

        // Convert front matter to JSON for JSONPath processing
        let front_matter = document.front_matter();
        if front_matter.is_none() {
            continue;
        }

        let yaml_value = YamlJsonConverter::document_front_matter_to_yaml(front_matter.unwrap());
        let json_value = YamlJsonConverter::yaml_to_json(&yaml_value)?;
        let located_results = jsonpath_query.query_located(&json_value);
        let matches: Vec<_> = located_results
            .into_iter()
            .map(|(path, value)| (path, value.clone()))
            .collect();

        let query_result = JsonPathQueryResult::new(jsonpath_query.clone(), matches);

        if !query_result.is_empty() {
            any_matches = true;
            total_matches += query_result.len();

            if args.count {
                // Just count, don't output results yet
                continue;
            } else if args.exists {
                // Just check existence, exit early on first match
                std::process::exit(0);
            } else if args.with_values {
                // Show normalized paths with values
                if files.len() > 1 {
                    println!("{}:", file.display());
                }
                for line in query_result.to_internal_format() {
                    if files.len() > 1 {
                        println!("  {}", line);
                    } else {
                        println!("{}", line);
                    }
                }
            } else {
                // Show just normalized paths
                if files.len() > 1 {
                    println!("{}:", file.display());
                }
                for path in query_result.paths() {
                    if files.len() > 1 {
                        println!("  {}", path);
                    } else {
                        println!("{}", path);
                    }
                }
            }
        }
    }

    if args.count {
        println!("{}", total_matches);
    } else if args.exists {
        // If we reach here, no matches were found
        std::process::exit(1);
    } else if !any_matches {
        debug!("No matching values found");
    }

    Ok(())
}

/// Execute the add command
pub fn add_command(args: AddArgs) -> Result<()> {
    debug!("Executing add command");

    let files = resolve_files(&args.files)?;
    if files.is_empty() {
        warn!("No files found to process");
        return Ok(());
    }

    let reader = create_reader(&args.files)?;
    let writer = create_writer(&args.write_options)?;
    let write_options = create_write_options(&args.write_options)?;

    // Create JSONPath query
    let jsonpath_query = if args.no_auto_root {
        JsonPathQuery::new_with_options(&args.query, false)?
    } else {
        JsonPathQuery::new(&args.query)?
    };

    // Parse value
    let value =
        FrontMatterValue::parse_from_string(&args.value, args.type_.map(Into::into).as_ref())?;

    let mut processed_count = 0;

    for file in files {
        debug!("Processing file: {}", file.display());

        let mut document = if file.exists() {
            reader.read_file(&file)?
        } else {
            Document::empty()
        };

        let modified = if let Some(add_key) = &args.add_key {
            // Add to object: create a new path by appending the add_key
            add_jsonpath_value(
                &mut document,
                &jsonpath_query,
                &value,
                Some(add_key),
                args.index,
            )?
        } else {
            // Add to array: use append semantics
            add_jsonpath_value(&mut document, &jsonpath_query, &value, None, args.index)?
        };

        if modified {
            let result = writer.write_file(&document, &file, Some(write_options.clone()))?;
            if result.modified {
                processed_count += 1;
                info!("Updated: {}", file.display());

                if let Some(diff) = result.diff {
                    if args.write_options.dry_run {
                        println!("{}", diff);
                    }
                }
            }
        }
    }

    info!("Processed {} files", processed_count);
    Ok(())
}

/// Execute the remove command
pub fn remove_command(args: RemoveArgs) -> Result<()> {
    debug!("Executing remove command");

    let files = resolve_files(&args.files)?;
    if files.is_empty() {
        warn!("No files found to process");
        return Ok(());
    }

    let reader = create_reader(&args.files)?;
    let writer = create_writer(&args.write_options)?;
    let write_options = create_write_options(&args.write_options)?;

    let mut processed_count = 0;

    for file in files {
        debug!("Processing file: {}", file.display());

        let mut document = reader.read_file(&file)?;
        let mut modified = false;

        if args.all {
            // Remove all front matter
            document = Document::new(None, document.body().to_string());
            modified = true;
        } else {
            // Use JSONPath query for removal
            if let Some(query_str) = &args.query {
                let jsonpath_query = if args.no_auto_root {
                    JsonPathQuery::new_with_options(query_str, false)?
                } else {
                    JsonPathQuery::new(query_str)?
                };

                modified = remove_jsonpath_value(&mut document, &jsonpath_query)?;
            }

            if args.cleanup_empty {
                document.clean_empty_front_matter();
            }
        }

        if modified {
            let result = writer.write_file(&document, &file, Some(write_options.clone()))?;
            if result.modified {
                processed_count += 1;
                info!("Updated: {}", file.display());

                if let Some(diff) = result.diff {
                    if args.write_options.dry_run {
                        println!("{}", diff);
                    }
                }
            }
        }
    }

    info!("Processed {} files", processed_count);
    Ok(())
}

/// Execute the replace command
pub fn replace_command(args: ReplaceArgs) -> Result<()> {
    debug!("Executing replace command");

    let files = resolve_files(&args.files)?;
    if files.is_empty() {
        warn!("No files found to process");
        return Ok(());
    }

    let reader = create_reader(&args.files)?;
    let writer = create_writer(&args.write_options)?;
    let write_options = create_write_options(&args.write_options)?;

    let mut processed_count = 0;

    for file in files {
        debug!("Processing file: {}", file.display());

        let mut document = reader.read_file(&file)?;
        let mut modified = false;

        // Use JSONPath query for replacement
        let _jsonpath_query = if args.no_auto_root {
            JsonPathQuery::new_with_options(&args.query, false)?
        } else {
            JsonPathQuery::new(&args.query)?
        };

        // For now, use a simplified approach
        // TODO: Implement proper JSONPath-based replacement
        let simple_key = args.query.trim_start_matches("$.").trim_start_matches("$");
        let old_key_path = KeyPath::parse(simple_key)?;

        if let Some(existing_value) = document.get(&old_key_path) {
            // Determine new key path
            let new_key_path = if let Some(new_key) = &args.new_key {
                KeyPath::parse(new_key)?
            } else {
                old_key_path.clone()
            };

            // Get new value
            let new_value = if let Some(new_val_str) = &args.new_value {
                FrontMatterValue::parse_from_string(
                    new_val_str,
                    args.type_.map(Into::into).as_ref(),
                )?
            } else {
                existing_value
            };

            // Remove old key if different from new key
            if old_key_path != new_key_path {
                document.remove(&old_key_path)?;
            }

            // Set new value at new key
            document.set(&new_key_path, new_value)?;
            modified = true;

            debug!("Replaced {} -> {}", old_key_path, new_key_path);
        }

        if modified {
            let result = writer.write_file(&document, &file, Some(write_options.clone()))?;
            if result.modified {
                processed_count += 1;
                info!("Updated: {}", file.display());

                if let Some(diff) = result.diff {
                    if args.write_options.dry_run {
                        println!("{}", diff);
                    }
                }
            }
        }
    }

    info!("Processed {} files", processed_count);
    Ok(())
}

/// Execute the init command
pub fn init_command(args: InitArgs) -> Result<()> {
    debug!("Executing init command");

    let files = resolve_files(&args.files)?;
    if files.is_empty() {
        warn!("No files found to process");
        return Ok(());
    }

    let reader = create_reader(&args.files)?;
    let writer = create_writer(&args.write_options)?;
    let write_options = create_write_options(&args.write_options)?;

    // Parse default values
    let defaults = parse_default_values(&args.defaults)?;

    let mut processed_count = 0;

    for file in files {
        debug!("Processing file: {}", file.display());

        let mut document = if file.exists() {
            reader.read_file(&file)?
        } else {
            Document::empty()
        };

        let needs_init = !document.has_front_matter();
        if args.only_missing && document.has_front_matter() {
            continue;
        }

        if needs_init || !defaults.is_empty() {
            document.ensure_front_matter();

            // Add default values
            for (key_path, value) in &defaults {
                if document.get(key_path).is_none() {
                    document.set(key_path, value.clone())?;
                }
            }

            let result = writer.write_file(&document, &file, Some(write_options.clone()))?;
            if result.modified {
                processed_count += 1;
                info!("Initialized: {}", file.display());
            }
        }
    }

    info!("Processed {} files", processed_count);
    Ok(())
}

/// Execute the clean command
pub fn clean_command(args: CleanArgs) -> Result<()> {
    debug!("Executing clean command");

    let files = resolve_files(&args.files)?;
    if files.is_empty() {
        warn!("No files found to process");
        return Ok(());
    }

    let reader = create_reader(&args.files)?;
    let writer = create_writer(&args.write_options)?;
    let write_options = create_write_options(&args.write_options)?;

    let mut processed_count = 0;

    for file in files {
        debug!("Processing file: {}", file.display());

        let mut document = reader.read_file(&file)?;
        let mut modified = false;

        if document.has_front_matter() {
            if args.remove_null {
                // Remove null values
                let query = Query::new()
                    .and_custom(|_key, value| value.is_null())
                    .combine_with(matterof::core::CombineMode::Any);

                let null_matches = document.query(&query);
                for (key_path, _) in null_matches.matches() {
                    document.remove(key_path)?;
                    modified = true;
                }
            }

            // Clean empty front matter
            document.clean_empty_front_matter();

            if !document.has_front_matter() {
                modified = true;
            }
        }

        if modified {
            let result = writer.write_file(&document, &file, Some(write_options.clone()))?;
            if result.modified {
                processed_count += 1;
                info!("Cleaned: {}", file.display());
            }
        }
    }

    info!("Processed {} files", processed_count);
    Ok(())
}

/// Execute the validate command
pub fn validate_command(args: ValidateArgs) -> Result<()> {
    debug!("Executing validate command");

    let files = resolve_files(&args.files)?;
    if files.is_empty() {
        warn!("No files found to process");
        return Ok(());
    }

    let reader = create_reader(&args.files)?;
    let mut validation_results = Vec::new();
    let mut error_count = 0;

    for file in files {
        debug!("Validating file: {}", file.display());

        let result = reader.read_file(&file);
        match result {
            Ok(document) => {
                if let Err(validation_error) = document.validate() {
                    if args.fail_fast {
                        return Err(MatterOfError::validation(format!(
                            "Validation failed for {}: {}",
                            file.display(),
                            validation_error
                        )));
                    }
                    validation_results.push((file.clone(), Err(validation_error.clone())));
                    error_count += 1;
                } else {
                    validation_results.push((file.clone(), Ok(())));
                }
            }
            Err(error) => {
                if args.fail_fast {
                    return Err(MatterOfError::validation(format!(
                        "Failed to read {}: {}",
                        file.display(),
                        error
                    )));
                }
                validation_results.push((file.clone(), Err(error)));
                error_count += 1;
            }
        }
    }

    // Output results
    output_validation_results(&validation_results, &args.format)?;

    if error_count > 0 {
        return Err(MatterOfError::validation(format!(
            "{} files failed validation",
            error_count
        )));
    }

    info!("All {} files passed validation", validation_results.len());
    Ok(())
}

/// Execute the format command
pub fn format_command(args: FormatArgs) -> Result<()> {
    debug!("Executing format command");

    let files = resolve_files(&args.files)?;
    if files.is_empty() {
        warn!("No files found to process");
        return Ok(());
    }

    let reader = create_reader(&args.files)?;
    let writer = create_writer(&args.write_options)?;
    let write_options = create_write_options(&args.write_options)?;

    let mut processed_count = 0;

    for file in files {
        debug!("Processing file: {}", file.display());

        let mut document = reader.read_file(&file)?;
        let mut modified = false;

        if document.has_front_matter() {
            if args.remove_null {
                // Remove null values
                let query = Query::new()
                    .and_custom(|_key, value| value.is_null())
                    .combine_with(matterof::core::CombineMode::Any);

                let null_matches = document.query(&query);
                for (key_path, _) in null_matches.matches() {
                    document.remove(key_path)?;
                    // modified is set to true for formatting operations
                }
            }

            // Note: Key sorting and indentation would be handled by the writer's YAML formatter
            // This is a simplified implementation
            modified = true; // Always consider formatting as a modification
        }

        if modified {
            let result = writer.write_file(&document, &file, Some(write_options.clone()))?;
            if result.modified {
                processed_count += 1;
                info!("Formatted: {}", file.display());
            }
        }
    }

    info!("Processed {} files", processed_count);
    Ok(())
}

// Helper functions

fn resolve_files(file_options: &CommonFileOptions) -> Result<Vec<std::path::PathBuf>> {
    let config = ResolverConfig {
        follow_links: file_options.follow_links,
        max_depth: file_options.max_depth,
        include_hidden: file_options.include_hidden,
        include_extensions: if file_options.extensions.is_empty() {
            vec!["md".to_string(), "markdown".to_string()]
        } else {
            file_options.extensions.clone()
        },
        exclude_patterns: file_options.exclude_patterns.clone(),
        ..Default::default()
    };

    let resolver = FileResolver::with_config(config);
    let resolved = resolver.resolve_paths(&file_options.files)?;

    Ok(resolved
        .into_iter()
        .map(|f| f.path().to_path_buf())
        .collect())
}

fn create_reader(_file_options: &CommonFileOptions) -> Result<FrontMatterReader> {
    let config = ReaderConfig {
        preserve_original: false, // We don't need original content for most operations
        validate_on_read: true,
        max_file_size: Some(10 * 1024 * 1024), // 10MB limit
    };

    Ok(FrontMatterReader::with_config(config))
}

fn create_writer(write_options: &WriteOptions) -> Result<FrontMatterWriter> {
    let config = WriterConfig {
        backup_enabled: write_options.backup_suffix.is_some() || write_options.backup_dir.is_some(),
        backup_suffix: write_options.backup_suffix.clone(),
        backup_dir: write_options.backup_dir.clone(),
        atomic_writes: !write_options.no_atomic,
        preserve_permissions: true,
        line_endings: write_options
            .line_endings
            .map(Into::into)
            .unwrap_or(matterof::io::LineEndings::Preserve),
    };

    Ok(FrontMatterWriter::with_config(config))
}

fn create_write_options(write_options: &WriteOptions) -> Result<LibWriteOptions> {
    let backup = if write_options.backup_suffix.is_some() || write_options.backup_dir.is_some() {
        Some(BackupOptions {
            enabled: true,
            suffix: write_options.backup_suffix.clone(),
            directory: write_options.backup_dir.clone(),
        })
    } else {
        None
    };

    let output = if write_options.stdout {
        Some(OutputOptions::Stdout)
    } else if let Some(ref output_dir) = write_options.output_dir {
        Some(OutputOptions::Directory(output_dir.clone()))
    } else {
        Some(OutputOptions::InPlace)
    };

    Ok(LibWriteOptions {
        backup,
        output,
        dry_run: write_options.dry_run,
    })
}

/// Set a value in a document using JSONPath
fn set_jsonpath_value(
    document: &mut Document,
    jsonpath_query: &JsonPathQuery,
    new_value: &FrontMatterValue,
) -> Result<bool> {
    // Ensure document has front matter
    document.ensure_front_matter();

    let front_matter = document.front_matter().unwrap();
    let yaml_value = YamlJsonConverter::document_front_matter_to_yaml(front_matter);
    let json_value = YamlJsonConverter::yaml_to_json(&yaml_value)?;

    // Find all matching locations first
    let located_results = jsonpath_query.query_located(&json_value);

    if located_results.is_empty() {
        debug!("No matches found for JSONPath query");
        return Ok(false);
    }

    // Collect the path strings to avoid borrowing issues
    let path_strings: Vec<String> = located_results
        .into_iter()
        .map(|(path, _)| path.to_string())
        .collect();

    // Now work with a fresh mutable copy of the JSON
    let mut json_value = YamlJsonConverter::yaml_to_json(&yaml_value)?;
    let new_json_value = YamlJsonConverter::front_matter_to_json(new_value)?;

    // Set value at all matching locations using the robust JsonMutator
    for path_string in path_strings {
        JsonMutator::set_at_path(&mut json_value, &path_string, new_json_value.clone())?;
    }

    // Convert back to YAML and update document
    let updated_yaml = YamlJsonConverter::json_to_yaml(&json_value)?;
    let updated_front_matter = YamlJsonConverter::yaml_to_document_front_matter(&updated_yaml)?;
    *document = Document::new(Some(updated_front_matter), document.body().to_string());

    Ok(true)
}

/// Set a JSON value at a specific normalized path string
fn remove_jsonpath_value(document: &mut Document, jsonpath_query: &JsonPathQuery) -> Result<bool> {
    // Ensure document has front matter
    document.ensure_front_matter();

    let front_matter = document.front_matter().unwrap();
    let yaml_value = YamlJsonConverter::document_front_matter_to_yaml(front_matter);
    let json_value = YamlJsonConverter::yaml_to_json(&yaml_value)?;

    // Find all matching locations first
    let located_results = jsonpath_query.query_located(&json_value);

    if located_results.is_empty() {
        debug!("No matches found for JSONPath query");
        return Ok(false);
    }

    // Collect the path strings to avoid borrowing issues
    // Sort them in reverse order to remove from deepest paths first
    let mut path_strings: Vec<String> = located_results
        .into_iter()
        .map(|(path, _)| path.to_string())
        .collect();
    path_strings.sort_by(|a, b| b.len().cmp(&a.len()));

    // Now work with a fresh mutable copy of the JSON
    let mut json_value = YamlJsonConverter::yaml_to_json(&yaml_value)?;
    let mut any_removed = false;

    // Remove values at all matching locations using the robust JsonMutator
    for path_string in path_strings {
        if JsonMutator::remove_at_path(&mut json_value, &path_string)? {
            any_removed = true;
        }
    }

    if any_removed {
        // Convert back to YAML and update document
        let updated_yaml = YamlJsonConverter::json_to_yaml(&json_value)?;
        let updated_front_matter = YamlJsonConverter::yaml_to_document_front_matter(&updated_yaml)?;
        *document = Document::new(Some(updated_front_matter), document.body().to_string());
    }

    Ok(any_removed)
}

/// Add a value using JSONPath semantics
fn add_jsonpath_value(
    document: &mut Document,
    jsonpath_query: &JsonPathQuery,
    new_value: &FrontMatterValue,
    add_key: Option<&str>,
    index: Option<usize>,
) -> Result<bool> {
    // Ensure document has front matter
    document.ensure_front_matter();

    let front_matter = document.front_matter().unwrap();
    let yaml_value = YamlJsonConverter::document_front_matter_to_yaml(front_matter);
    let mut json_value = YamlJsonConverter::yaml_to_json(&yaml_value)?;
    let new_json_value = YamlJsonConverter::front_matter_to_json(new_value)?;

    if let Some(key) = add_key {
        // Adding a new property to an object
        // Find all matching locations and add the new key to each
        let located_results = jsonpath_query.query_located(&json_value);

        if located_results.is_empty() {
            // If no matches, try to create the path and add the property
            let base_path = jsonpath_query.path().to_string();

            // Create the base path if it doesn't exist
            JsonMutator::set_at_path(&mut json_value, &base_path, serde_json::json!({}))?;

            // Add the new property
            let new_path = format!("{}['{}']", base_path, key);
            JsonMutator::set_at_path(&mut json_value, &new_path, new_json_value)?;
        } else {
            // Collect path strings to avoid borrowing issues
            let path_strings: Vec<String> = located_results
                .into_iter()
                .map(|(path, _)| path.to_string())
                .collect();

            // Add the new property to all matching objects
            for path_string in path_strings {
                let new_path = format!("{}['{}']", path_string, key);
                JsonMutator::set_at_path(&mut json_value, &new_path, new_json_value.clone())?;
            }
        }
    } else {
        // Adding to an array
        let located_results = jsonpath_query.query_located(&json_value);

        if located_results.is_empty() {
            // If no matches, create an array at the path and add the value
            let base_path = jsonpath_query.path().to_string();

            // Create the base path as an empty array
            JsonMutator::set_at_path(&mut json_value, &base_path, serde_json::json!([]))?;

            // Add the value using append semantics
            let append_path = format!("{}[-]", base_path);
            JsonMutator::set_at_path(&mut json_value, &append_path, new_json_value)?;
        } else {
            // Collect path strings to avoid borrowing issues
            let path_strings: Vec<String> = located_results
                .into_iter()
                .map(|(path, _)| path.to_string())
                .collect();

            // Add to all matching arrays
            for path_string in path_strings {
                if let Some(idx) = index {
                    // Insert at specific index
                    let indexed_path = format!("{}[{}]", path_string, idx);
                    JsonMutator::set_at_path(
                        &mut json_value,
                        &indexed_path,
                        new_json_value.clone(),
                    )?;
                } else {
                    // Append to array
                    let append_path = format!("{}[-]", path_string);
                    JsonMutator::set_at_path(
                        &mut json_value,
                        &append_path,
                        new_json_value.clone(),
                    )?;
                }
            }
        }
    }

    // Convert back to YAML and update document
    let updated_yaml = YamlJsonConverter::json_to_yaml(&json_value)?;
    let updated_front_matter = YamlJsonConverter::yaml_to_document_front_matter(&updated_yaml)?;
    *document = Document::new(Some(updated_front_matter), document.body().to_string());

    Ok(true)
}

fn parse_cli_value(
    value: &str,
    value_type: Option<&matterof::core::ValueType>,
) -> Result<FrontMatterValue> {
    FrontMatterValue::parse_from_string(value, value_type)
}

fn parse_default_values(defaults: &[String]) -> Result<Vec<(KeyPath, FrontMatterValue)>> {
    let mut result = Vec::new();

    for default in defaults {
        let parts: Vec<&str> = default.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(MatterOfError::validation(format!(
                "Invalid default format '{}', expected 'key=value'",
                default
            )));
        }

        let key_path = KeyPath::parse(parts[0])?;
        let value = FrontMatterValue::parse_from_string(parts[1], None)?;
        result.push((key_path, value));
    }

    Ok(result)
}

fn output_jsonpath_result(
    result: &JsonPathQueryResult,
    format: &OutputFormat,
    pretty: bool,
) -> Result<()> {
    match format {
        OutputFormat::Yaml => {
            let yaml_value = result.to_yaml()?;
            let output = serde_yaml::to_string(&yaml_value)?;
            print!("{}", output);
        }
        OutputFormat::Json => {
            let json_value = result.to_json()?;
            if pretty {
                let output = serde_json::to_string_pretty(&json_value)
                    .map_err(|e| MatterOfError::validation(e.to_string()))?;
                println!("{}", output);
            } else {
                let output = serde_json::to_string(&json_value)
                    .map_err(|e| MatterOfError::validation(e.to_string()))?;
                println!("{}", output);
            }
        }
        OutputFormat::Internal => {
            for line in result.to_internal_format() {
                println!("{}", line);
            }
        }
    }

    Ok(())
}

fn output_yaml_value(
    yaml_value: &serde_yaml::Value,
    format: &OutputFormat,
    pretty: bool,
) -> Result<()> {
    match format {
        OutputFormat::Yaml => {
            let output = serde_yaml::to_string(yaml_value)?;
            print!("{}", output);
        }
        OutputFormat::Json => {
            let json_value = YamlJsonConverter::yaml_to_json(yaml_value)?;
            if pretty {
                let output = serde_json::to_string_pretty(&json_value)
                    .map_err(|e| MatterOfError::validation(e.to_string()))?;
                println!("{}", output);
            } else {
                let output = serde_json::to_string(&json_value)
                    .map_err(|e| MatterOfError::validation(e.to_string()))?;
                println!("{}", output);
            }
        }
        OutputFormat::Internal => {
            // For --all queries, show the root path
            println!("$: {}", serde_yaml::to_string(yaml_value)?.trim());
        }
    }

    Ok(())
}

fn output_multiple_yaml_results(
    results: &BTreeMap<String, serde_yaml::Value>,
    format: &OutputFormat,
    pretty: bool,
) -> Result<()> {
    match format {
        OutputFormat::Yaml => {
            let mut output_map = serde_yaml::Mapping::new();
            for (filename, result) in results {
                output_map.insert(serde_yaml::Value::String(filename.clone()), result.clone());
            }
            let output = serde_yaml::to_string(&serde_yaml::Value::Mapping(output_map))?;
            print!("{}", output);
        }
        _ => {
            // For other formats, output each file separately
            for (filename, result) in results {
                println!("# {}", filename);
                output_yaml_value(result, format, pretty)?;
                println!();
            }
        }
    }

    Ok(())
}

fn output_validation_results(
    results: &[(std::path::PathBuf, Result<()>)],
    format: &ValidationFormat,
) -> Result<()> {
    match format {
        ValidationFormat::Human => {
            for (path, result) in results {
                match result {
                    Ok(()) => println!("{}: ✓ OK", path.display()),
                    Err(error) => println!("{}: ✗ ERROR - {}", path.display(), error),
                }
            }
        }
        ValidationFormat::Json => {
            let json_results: Vec<serde_json::Value> = results
                .iter()
                .map(|(path, result)| {
                    serde_json::json!({
                        "file": path.to_string_lossy(),
                        "valid": result.is_ok(),
                        "error": if let Err(e) = result { Some(e.to_string()) } else { None }
                    })
                })
                .collect();

            let output = if results.len() == 1 {
                serde_json::to_string_pretty(&json_results[0])
                    .map_err(|e| MatterOfError::validation(e.to_string()))?
            } else {
                serde_json::to_string_pretty(&json_results)
                    .map_err(|e| MatterOfError::validation(e.to_string()))?
            };
            println!("{}", output);
        }
        ValidationFormat::Simple => {
            for (path, result) in results {
                if result.is_ok() {
                    println!("{}", path.display());
                }
            }
        }
    }

    Ok(())
}
