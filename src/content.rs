use anyhow::Result;
use gray_matter::{Matter, Pod};
use gray_matter::engine::YAML;
use pulldown_cmark::{html, Options, Parser};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn pod_to_yaml_value(pod: Pod) -> Value {
    match pod {
        Pod::String(s) => Value::String(s),
        Pod::Integer(n) => Value::Number(serde_yaml::Number::from(n)),
        Pod::Float(f) => Value::Number(serde_yaml::Number::from(f as i64)), // Approximate conversion
        Pod::Boolean(b) => Value::Bool(b),
        Pod::Array(arr) => Value::Sequence(arr.into_iter().map(pod_to_yaml_value).collect()),
        Pod::Hash(map) => {
            let mut yaml_map = serde_yaml::Mapping::new();
            for (k, v) in map {
                yaml_map.insert(Value::String(k), pod_to_yaml_value(v));
            }
            Value::Mapping(yaml_map)
        }
        Pod::Null => Value::Null,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub layout: Option<String>,
    pub lang: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentFile {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub front_matter: FrontMatter,
    pub content: String,
    pub html_content: String,
    pub collection: Option<String>,
    pub language: String,
}

impl ContentFile {
    pub fn from_path(path: &Path, source_root: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let matter = Matter::<YAML>::new();
        let result = matter.parse(&content);

        let front_matter: FrontMatter = if let Some(data) = result.data {
            match data {
                Pod::Hash(map) => {
                    let mut fm = FrontMatter {
                        title: None,
                        layout: None,
                        lang: None,
                        extra: HashMap::new(),
                    };
                    
                    for (key, value) in map {
                        match key.as_str() {
                            "title" => if let Pod::String(s) = value { fm.title = Some(s); }
                            "layout" => if let Pod::String(s) = value { fm.layout = Some(s); }
                            "lang" => if let Pod::String(s) = value { fm.lang = Some(s); }
                            _ => {
                                let yaml_value = pod_to_yaml_value(value);
                                fm.extra.insert(key, yaml_value);
                            }
                        }
                    }
                    fm
                }
                _ => FrontMatter {
                    title: None,
                    layout: None,
                    lang: None,
                    extra: HashMap::new(),
                }
            }
        } else {
            FrontMatter {
                title: None,
                layout: None,
                lang: None,
                extra: HashMap::new(),
            }
        };

        // Convert markdown to HTML
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_TASKLISTS);

        let parser = Parser::new_ext(&result.content, options);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        // Determine collection and language from path
        let relative_path = path.strip_prefix(source_root)?.to_path_buf();
        let (collection, language) = Self::extract_collection_and_language(&relative_path);

        Ok(ContentFile {
            path: path.to_path_buf(),
            relative_path,
            front_matter,
            content: result.content,
            html_content: html_output,
            collection,
            language,
        })
    }

    fn extract_collection_and_language(path: &Path) -> (Option<String>, String) {
        let path_str = path.to_string_lossy();
        
        if path_str.starts_with("_pages") {
            (Some("pages".to_string()), "en".to_string())
        } else {
            (None, "en".to_string())
        }
    }

    pub fn get_output_path(&self, _base_url: &str) -> String {
        let stem = self.path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("index");

        format!("/{}/", stem)
    }

    pub fn get_file_path(&self) -> PathBuf {
        let mut path = PathBuf::new();
        
        let stem = self.path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("index");
        
        if stem == "index" {
            path.push("index.html");
        } else {
            path.push(stem);
            path.push("index.html");
        }
        
        path
    }

    pub fn get_language_urls(&self) -> std::collections::HashMap<String, String> {
        let mut urls = std::collections::HashMap::new();
        let stem = self.path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("index");

        if stem == "index" {
            urls.insert("en".to_string(), "/".to_string());
        } else {
            urls.insert("en".to_string(), format!("/{}/", stem));
        }
        
        urls
    }
}