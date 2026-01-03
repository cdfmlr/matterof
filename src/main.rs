use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;
use regex::Regex;

use matterof::{
    args::{Commands, CommonOpts},
    io::{resolve_files, read_to_string, write_atomic, parse, format},
    core::{Document, Selector, parse_key_path},
};

#[derive(Parser)]
#[command(name = "matterof", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Get(args) => {
            let selector = Selector {
                all: args.all,
                keys: args.key.iter().map(|k| parse_key_path(k)).collect(),
                key_parts: args.key_part,
                key_regex: args.key_regex.as_deref().map(Regex::new).transpose()?,
                key_part_regex: args.key_part_regex.iter().map(|s| Regex::new(s)).collect::<Result<Vec<_>, _>>()?,
                value_regex: args.value_regex.as_deref().map(Regex::new).transpose()?,
                ..Default::default()
            };

            let mut results = HashMap::new();
            let files = resolve_files(&args.files);

            for file in &files {
                let content = read_to_string(file)?;
                let (fm, body) = parse(&content)?;
                let doc = Document::new(fm, body);
                let selected = doc.select(&selector);
                
                if !selected.is_null() && !(selected.is_mapping() && selected.as_mapping().unwrap().is_empty()) {
                    results.insert(file.to_string_lossy().to_string(), selected);
                }
            }

            if results.is_empty() { return Ok(()); }
            if files.len() == 1 {
                println!("{}", serde_yaml::to_string(results.values().next().unwrap())?.trim_start_matches("---\
"));
            } else {
                println!("{}", serde_yaml::to_string(&results)?.trim_start_matches("---\
"));
            }
        }
        Commands::Set(args) => {
            let value = parse_cli_values(&args.value, &args.type_)?;
            let key_regex = args.key_regex.as_deref().map(Regex::new).transpose()?;

            for file in resolve_files(&args.files) {
                let content = read_to_string(&file)?;
                let (fm, body) = parse(&content)?;
                let mut doc = Document::new(fm, body);

                if let Some(re) = &key_regex {
                    let selector = Selector { key_regex: Some(re.clone()), ..Default::default() };
                    let flattened: Vec<(Vec<String>, serde_yaml::Value)> = doc.data.as_ref().map(|d| matterof::core::path::flatten_yaml(d)).unwrap_or_default();
                    for (path, val) in flattened {
                         if selector.matches(&path, &val) {
                             doc.set(&path, value.clone());
                         }
                    }
                } else {
                    for k in &args.key { doc.set(&parse_key_path(k), value.clone()); }
                    if !args.key_part.is_empty() { doc.set(&args.key_part, value.clone()); }
                }
                save_doc(&file, &doc, &args.opts)?;
            }
        }
        Commands::Add(args) => {
            let val = parse_cli_value(&args.value, &None)?;
            let path = if let Some(k) = &args.key { parse_key_path(k) } else { args.key_part.clone() };

            for file in resolve_files(&args.files) {
                let content = read_to_string(&file)?;
                let (fm, body) = parse(&content)?;
                let mut doc = Document::new(fm, body);
                doc.add(&path, val.clone(), args.index);
                save_doc(&file, &doc, &args.opts)?;
            }
        }
        Commands::Rm(args) => {
            let selector = Selector {
                keys: args.key.as_deref().map(|k| vec![parse_key_path(k)]).unwrap_or_default(),
                key_parts: args.key_part,
                key_regex: args.key_regex.as_deref().map(Regex::new).transpose()?,
                value_match: args.value,
                value_regex: args.value_regex.as_deref().map(Regex::new).transpose()?,
                all: args.all,
                ..Default::default()
            };

            for file in resolve_files(&args.files) {
                let content = read_to_string(&file)?;
                let (fm, body) = parse(&content)?;
                let mut doc = Document::new(fm, body);
                doc.remove(&selector);
                save_doc(&file, &doc, &args.opts)?;
            }
        }
        Commands::Replace(args) => {
            let selector = Selector {
                keys: args.key.as_deref().map(|k| vec![parse_key_path(k)]).unwrap_or_default(),
                key_parts: args.key_part,
                key_regex: args.key_regex.as_deref().map(Regex::new).transpose()?,
                value_match: args.old_value,
                value_regex: args.old_value_regex.as_deref().map(Regex::new).transpose()?,
                ..Default::default()
            };
            let val = if let Some(v) = &args.value { Some(parse_cli_value(v, &args.type_)?) } else { None };
            let new_val = if let Some(v) = &args.new_value { Some(parse_cli_value(v, &args.type_)?) } else { None };
            let target_val = new_val.or(val);

            let nk_path = if let Some(nk) = &args.new_key {
                 Some(parse_key_path(nk))
            } else if !args.new_key_part.is_empty() {
                 Some(args.new_key_part.clone())
            } else {
                 None
            };

            for file in resolve_files(&args.files) {
                let content = read_to_string(&file)?;
                let (fm, body) = parse(&content)?;
                let mut doc = Document::new(fm, body);
                doc.replace(&selector, target_val.clone(), nk_path.clone());
                save_doc(&file, &doc, &args.opts)?;
            }
        }
        Commands::Init(args) => {
            for file in resolve_files(&args.files) {
                let content = read_to_string(&file)?;
                let (fm, body) = parse(&content)?;
                if fm.is_none() {
                    let doc = Document::new(Some(serde_yaml::Value::Mapping(serde_yaml::Mapping::new())), body);
                    save_doc(&file, &doc, &CommonOpts::default())?;
                }
            }
        }
        Commands::Clean(args) => {
            for file in resolve_files(&args.files) {
                let content = read_to_string(&file)?;
                let (fm, body) = parse(&content)?;
                if let Some(f) = &fm {
                    if f.is_null() || (f.is_mapping() && f.as_mapping().unwrap().is_empty()) {
                        let doc = Document::new(None, body);
                        save_doc(&file, &doc, &CommonOpts::default())?;
                    }
                }
            }
        }
        Commands::Validate(args) => {
            for file in resolve_files(&args.files) {
                match read_to_string(&file).and_then(|c| parse(&c)) {
                    Ok(_) => println!("{}: OK", file.display()),
                    Err(e) => println!("{}: Invalid ({})", file.display(), e),
                }
            }
        }
        Commands::Fmt(args) => {
            for file in resolve_files(&args.files) {
                let content = read_to_string(&file)?;
                let (fm, body) = parse(&content)?;
                if let Some(f) = fm {
                    let doc = Document::new(Some(f), body);
                    save_doc(&file, &doc, &CommonOpts::default())?;
                }
            }
        }
    }
    Ok(())
}

fn save_doc(path: &std::path::Path, doc: &Document, opts: &CommonOpts) -> Result<()> {
    let new_content = format(doc.data.as_ref(), &doc.body)?;
    
    if opts.dry_run {
         // Re-implementing dry-run diff logic briefly or move to IO
         println!("--- Dry run: {} ---", path.display());
         println!("{}", new_content);
         return Ok(());
    }

    if opts.stdout {
        println!("{}", new_content);
        return Ok(());
    }

    // Simplified save for brevity in main, logic should ideally be in io::fs with options
    write_atomic(path, &new_content)
}

fn parse_cli_values(raw: &[String], type_: &Option<String>) -> Result<serde_yaml::Value> {
    let mut vals = Vec::new();
    for r in raw {
        if raw.len() == 1 {
            for p in r.split(',') { vals.push(parse_cli_value(p, type_)?); }
        } else {
            vals.push(parse_cli_value(r, type_)?);
        }
    }
    Ok(if vals.len() == 1 { vals.remove(0) } else { serde_yaml::Value::Sequence(vals) })
}

fn parse_cli_value(v: &str, type_: &Option<String>) -> Result<serde_yaml::Value> {
    match type_.as_deref() {
        Some("int") => Ok(serde_yaml::Value::Number(v.trim().parse::<i64>()?.into())),
        Some("float") => Ok(serde_yaml::Value::Number(serde_yaml::Number::from(v.trim().parse::<f64>()?))),
        Some("bool") => Ok(serde_yaml::Value::Bool(v.trim().parse()?)),
        _ => Ok(serde_yaml::Value::String(v.to_string())),
    }
}