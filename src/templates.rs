use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use tera::{Context, Tera};

use crate::content::ContentFile;

pub struct TemplateEngine {
    tera: Tera,
    data: HashMap<String, Value>,
}

impl TemplateEngine {
    pub fn new(source_dir: &Path) -> Result<Self> {
        let mut tera = Tera::new("templates/**/*.html").unwrap_or_else(|e| {
            println!("Failed to load from templates/: {}", e);
            Tera::default()
        });
        
        let data = Self::load_data_files(source_dir)?;
        
        tera.register_filter("escape", |value: &Value, _: &HashMap<String, Value>| {
            match value {
                Value::String(s) => Ok(Value::String(html_escape::encode_text(s).to_string())),
                _ => Ok(value.clone()),
            }
        });

        tera.register_filter("relative_url", |value: &Value, args: &HashMap<String, Value>| {
            if let Value::String(url) = value {
                let base_url = args.get("base_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                if url.starts_with('/') {
                    Ok(Value::String(format!("{}{}", base_url, url)))
                } else {
                    Ok(value.clone())
                }
            } else {
                Ok(value.clone())
            }
        });

        tera.register_filter("absolute_url", |value: &Value, args: &HashMap<String, Value>| {
            if let Value::String(url) = value {
                let site_url = args.get("site_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                Ok(Value::String(format!("{}{}", site_url, url)))
            } else {
                Ok(value.clone())
            }
        });

        Ok(TemplateEngine { tera, data })
    }

    fn load_data_files(source_dir: &Path) -> Result<HashMap<String, Value>> {
        let mut data = HashMap::new();
        let data_dir = source_dir.join("_data");

        if data_dir.exists() {
            for entry in std::fs::read_dir(&data_dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if let Some(extension) = path.extension() {
                    if extension == "yml" || extension == "yaml" {
                        let content = std::fs::read_to_string(&path)?;
                        let yaml_data: Value = serde_yaml::from_str(&content)?;
                        
                        if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                            data.insert(filename.to_string(), yaml_data);
                        }
                    } else if extension == "json" {
                        let content = std::fs::read_to_string(&path)?;
                        let json_data: Value = serde_json::from_str(&content)?;
                        
                        if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                            data.insert(filename.to_string(), json_data);
                        }
                    }
                }
            }
        }

        Ok(data)
    }

    pub fn render_content(&self, content: &ContentFile, site_config: &crate::config::SiteConfig) -> Result<String> {
        let mut context = Context::new();
        
        context.insert("page", &content.front_matter);
        context.insert("content", &content.html_content);
        context.insert("site", site_config);
        context.insert("data", &self.data);
        
        let lang = content.language.clone();
        context.insert("lang", &lang);
        
        if let Some(translations) = self.data.get("translations") {
            if let Some(t) = translations.get(&lang) {
                context.insert("t", t);
            }
        }

        let language_urls = content.get_language_urls();
        context.insert("language_urls", &language_urls);

        let layout = content.front_matter.layout
            .as_ref()
            .unwrap_or(&"default".to_string())
            .clone();

        let template_name = format!("{}.html", layout);
        
        self.tera.render(&template_name, &context)
            .map_err(|e| anyhow::anyhow!("Template rendering error: {}", e))
    }

    pub fn render_page(&self, template_name: &str, context: &Context) -> Result<String> {
        self.tera.render(template_name, context)
            .map_err(|e| anyhow::anyhow!("Template rendering error: {}", e))
    }
}