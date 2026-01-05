//! CLI command handlers that bridge CLI arguments to library operations
//!
//! This module contains the implementation of all CLI commands, providing
//! a clean separation between CLI argument parsing and core library operations.

use crate::cli_bin::args::*;
use log::{debug, info, warn};
use matterof::core::{Document, FrontMatterValue, KeyPath, Query};
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

    // Build the query
    let query = build_query_from_get_args(&args)?;

    for file in &files {
        debug!("Processing file: {}", file.display());

        let document = reader.read_file(file)?;
        let query_result = document.query(&query);

        if !query_result.is_empty() {
            if files.len() == 1 {
                // Single file - output just the values
                output_query_result(&query_result, &args.format, args.pretty)?;
                return Ok(());
            } else {
                // Multiple files - collect results
                results.insert(file.to_string_lossy().to_string(), query_result);
            }
        }
    }

    // Output results for multiple files
    if !results.is_empty() {
        output_multiple_file_results(&results, &args.format, args.pretty)?;
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

    // Parse values
    let values = parse_cli_values(&args.value, args.type_.map(Into::into).as_ref())?;

    // Build key paths
    let key_paths = build_key_paths(&args.key, &args.key_part, args.key_regex.as_deref())?;

    let mut processed_count = 0;

    for file in files {
        debug!("Processing file: {}", file.display());

        let mut document = if file.exists() {
            reader.read_file(&file)?
        } else {
            Document::empty()
        };

        let mut modified = false;

        for key_path in &key_paths {
            debug!("Setting key: {} = {:?}", key_path, values);
            document.set(key_path, values.clone())?;
            modified = true;
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

    // Build key path
    let key_path = if let Some(key) = &args.key {
        KeyPath::parse(key)?
    } else {
        KeyPath::from_segments(args.key_part.clone())
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

        document.add_to_array(&key_path, value.clone(), args.index)?;

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
            // Build query for removal
            let query = build_removal_query(&args)?;

            // Find matches to remove
            let matches = document.query(&query);
            for (key_path, _) in matches.matches() {
                debug!("Removing key: {}", key_path);
                document.remove(key_path)?;
                modified = true;
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

        // Build query for what to replace
        let query = build_replacement_query(&args)?;

        // Find matches
        let matches = document.query(&query);

        for (old_key_path, _) in matches.matches() {
            // Determine new key path
            let new_key_path = if let Some(new_key) = &args.new_key {
                KeyPath::parse(new_key)?
            } else if !args.new_key_part.is_empty() {
                KeyPath::from_segments(args.new_key_part.clone())
            } else {
                old_key_path.clone()
            };

            // Get current value or use new value
            let new_value = if let Some(new_val_str) = &args.new_value {
                FrontMatterValue::parse_from_string(
                    new_val_str,
                    args.type_.map(Into::into).as_ref(),
                )?
            } else {
                document
                    .get(old_key_path)
                    .unwrap_or(FrontMatterValue::null())
            };

            // Remove old key if different from new key
            if old_key_path != &new_key_path {
                document.remove(old_key_path)?;
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

fn build_query_from_get_args(args: &GetArgs) -> Result<Query> {
    if args.all {
        return Ok(Query::all());
    }

    let mut query = Query::new();
    let mut has_conditions = false;

    // Add key conditions
    if !args.key.is_empty() {
        let key_paths: Result<Vec<KeyPath>> = args.key.iter().map(|k| KeyPath::parse(k)).collect();
        for key_path in key_paths? {
            if args.exact {
                query = query.and_exact_key(key_path);
            } else {
                query = query.and_key(key_path);
            }
            has_conditions = true;
        }
    }

    // Add key part conditions
    if !args.key_part.is_empty() {
        let key_path = KeyPath::from_segments(args.key_part.clone());
        if args.exact {
            query = query.and_exact_key(key_path);
        } else {
            query = query.and_key(key_path);
        }
        has_conditions = true;
    }

    // Add regex conditions
    if let Some(ref key_regex) = args.key_regex {
        query = query.and_key_regex(key_regex)?;
        has_conditions = true;
    }

    if let Some(ref value_regex) = args.value_regex {
        query = query.and_value_regex(value_regex)?;
        has_conditions = true;
    }

    // Add value match condition
    if let Some(ref value_match) = args.value_match {
        let value = FrontMatterValue::string(value_match);
        query = query.and_value(value);
        has_conditions = true;
    }

    // Add depth condition
    if let Some(depth) = args.depth {
        query = query.and_depth(depth);
        has_conditions = true;
    }

    // Add exists condition
    if args.exists_only {
        query = query.and_exists();
        has_conditions = true;
    }

    if !has_conditions {
        query = Query::all();
    }

    Ok(query)
}

fn build_key_paths(
    keys: &[String],
    key_parts: &[String],
    key_regex: Option<&str>,
) -> Result<Vec<KeyPath>> {
    let mut key_paths = Vec::new();

    // Add explicit keys
    for key in keys {
        key_paths.push(KeyPath::parse(key)?);
    }

    // Add key parts
    if !key_parts.is_empty() {
        key_paths.push(KeyPath::from_segments(key_parts.to_vec()));
    }

    // Note: key_regex would need special handling as it's not a static key path
    if key_regex.is_some() {
        return Err(MatterOfError::not_supported(
            "Regex key paths in set operations".to_string(),
        ));
    }

    if key_paths.is_empty() {
        return Err(MatterOfError::validation("No keys specified".to_string()));
    }

    Ok(key_paths)
}

fn build_removal_query(args: &RemoveArgs) -> Result<Query> {
    let mut query = Query::new();
    let mut has_conditions = false;

    // Add key conditions
    for key in &args.key {
        let key_path = KeyPath::parse(key)?;
        query = query.and_key(key_path);
        has_conditions = true;
    }

    if !args.key_part.is_empty() {
        let key_path = KeyPath::from_segments(args.key_part.clone());
        query = query.and_key(key_path);
        has_conditions = true;
    }

    if let Some(ref key_regex) = args.key_regex {
        query = query.and_key_regex(key_regex)?;
        has_conditions = true;
    }

    if let Some(ref value) = args.value {
        let value_obj = FrontMatterValue::string(value);
        query = query.and_value(value_obj);
        has_conditions = true;
    }

    if let Some(ref value_regex) = args.value_regex {
        query = query.and_value_regex(value_regex)?;
        has_conditions = true;
    }

    if !has_conditions {
        return Err(MatterOfError::validation(
            "No removal criteria specified".to_string(),
        ));
    }

    Ok(query)
}

fn build_replacement_query(args: &ReplaceArgs) -> Result<Query> {
    let mut query = Query::new();
    let mut has_conditions = false;

    // Add key conditions
    for key in &args.key {
        let key_path = KeyPath::parse(key)?;
        query = query.and_key(key_path);
        has_conditions = true;
    }

    if !args.key_part.is_empty() {
        let key_path = KeyPath::from_segments(args.key_part.clone());
        query = query.and_key(key_path);
        has_conditions = true;
    }

    if let Some(ref key_regex) = args.key_regex {
        query = query.and_key_regex(key_regex)?;
        has_conditions = true;
    }

    if let Some(ref old_value) = args.old_value {
        let value_obj = FrontMatterValue::string(old_value);
        query = query.and_value(value_obj);
        has_conditions = true;
    }

    if let Some(ref old_value_regex) = args.old_value_regex {
        query = query.and_value_regex(old_value_regex)?;
        has_conditions = true;
    }

    if !has_conditions {
        return Err(MatterOfError::validation(
            "No replacement criteria specified".to_string(),
        ));
    }

    Ok(query)
}

fn parse_cli_values(
    values: &[String],
    value_type: Option<&matterof::core::ValueType>,
) -> Result<FrontMatterValue> {
    if values.len() == 1 {
        // Single value
        FrontMatterValue::parse_from_string(&values[0], value_type)
    } else {
        // Multiple values - create array
        let parsed_values: Result<Vec<FrontMatterValue>> = values
            .iter()
            .map(|v| FrontMatterValue::parse_from_string(v, value_type))
            .collect();
        Ok(FrontMatterValue::array(parsed_values?))
    }
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

fn output_query_result(
    result: &matterof::core::QueryResult,
    format: &OutputFormat,
    pretty: bool,
) -> Result<()> {
    let yaml_value = result.to_yaml_value();

    match format {
        OutputFormat::Yaml => {
            let output = serde_yaml::to_string(&yaml_value)?;
            print!("{}", output);
        }
        OutputFormat::Json => {
            if pretty {
                let output = serde_json::to_string_pretty(&yaml_value)
                    .map_err(|e| MatterOfError::validation(e.to_string()))?;
                println!("{}", output);
            } else {
                let output = serde_json::to_string(&yaml_value)
                    .map_err(|e| MatterOfError::validation(e.to_string()))?;
                println!("{}", output);
            }
        }
        OutputFormat::Text => {
            for (_, value) in result.matches() {
                println!("{}", value.to_string_representation());
            }
        }
        OutputFormat::Csv => {
            // Simple CSV output for flat key-value pairs
            println!("key,value");
            for (key_path, value) in result.matches() {
                println!("{},{}", key_path, value.to_string_representation());
            }
        }
    }

    Ok(())
}

fn output_multiple_file_results(
    results: &BTreeMap<String, matterof::core::QueryResult>,
    format: &OutputFormat,
    pretty: bool,
) -> Result<()> {
    match format {
        OutputFormat::Yaml => {
            let mut output_map = serde_yaml::Mapping::new();
            for (filename, result) in results {
                output_map.insert(
                    serde_yaml::Value::String(filename.clone()),
                    result.to_yaml_value(),
                );
            }
            let output = serde_yaml::to_string(&serde_yaml::Value::Mapping(output_map))?;
            print!("{}", output);
        }
        _ => {
            // For other formats, output each file separately
            for (filename, result) in results {
                println!("# {}", filename);
                output_query_result(result, format, pretty)?;
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
