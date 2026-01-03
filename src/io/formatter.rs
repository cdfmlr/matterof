use gray_matter::Matter;
use gray_matter::engine::YAML;
use anyhow::{Result, Context};

pub fn parse(content: &str) -> Result<(Option<serde_yaml::Value>, String)> {
    if !content.trim_start().starts_with("---") {
        return Ok((None, content.to_string()));
    }

    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(content);
    
    let data = if let Some(d) = parsed.data {
        let val: serde_yaml::Value = d.deserialize()
            .context("failed to deserialize front matter")?;
        Some(val)
    } else {
        Some(serde_yaml::Value::Null)
    };

    Ok((data, parsed.content))
}

pub fn format(fm: Option<&serde_yaml::Value>, body: &str) -> Result<String> {
    if let Some(fm_val) = fm {
        let fm_str = serde_yaml::to_string(fm_val)?;
        let trimmed_fm = fm_str.trim_start_matches("---").trim();
        Ok(format!("---\n{}\n---\n{}", trimmed_fm, body))
    } else {
        Ok(body.to_string())
    }
}
