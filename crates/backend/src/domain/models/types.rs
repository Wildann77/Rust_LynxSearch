use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Doc,
    Code,
    Config,
}

impl DocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Doc => "doc",
            Self::Code => "code",
            Self::Config => "config",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        let clean = ext.trim_start_matches('.').to_ascii_lowercase();
        match clean.as_str() {
            "md" | "markdown" | "txt" => Some(Self::Doc),
            "json" | "toml" | "yaml" | "yml" | "ini" => Some(Self::Config),
            "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "kt" | "c" | "cpp"
            | "h" | "cs" | "rb" | "php" | "swift" | "sh" | "sql" | "html" | "css" | "scss"
            | "vue" | "svelte" | "lua" => Some(Self::Code),
            _ => None,
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }
}

impl fmt::Display for DocumentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DocumentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "doc" => Ok(Self::Doc),
            "code" => Ok(Self::Code),
            "config" => Ok(Self::Config),
            other => Err(format!("Unknown document type: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Language {
    Markdown,
    Text,
    Json,
    Toml,
    Yaml,
    Ini,
    Rust,
    Typescript,
    Javascript,
    Python,
    Go,
    Java,
    Kotlin,
    C,
    Cpp,
    Csharp,
    Ruby,
    Php,
    Swift,
    Shell,
    Sql,
    Html,
    Css,
    Scss,
    Vue,
    Svelte,
    Lua,
    Other(String),
}

impl Language {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Markdown => "markdown",
            Self::Text => "text",
            Self::Json => "json",
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Ini => "ini",
            Self::Rust => "rust",
            Self::Typescript => "typescript",
            Self::Javascript => "javascript",
            Self::Python => "python",
            Self::Go => "go",
            Self::Java => "java",
            Self::Kotlin => "kotlin",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Csharp => "csharp",
            Self::Ruby => "ruby",
            Self::Php => "php",
            Self::Swift => "swift",
            Self::Shell => "shell",
            Self::Sql => "sql",
            Self::Html => "html",
            Self::Css => "css",
            Self::Scss => "scss",
            Self::Vue => "vue",
            Self::Svelte => "svelte",
            Self::Lua => "lua",
            Self::Other(s) => s.as_str(),
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        let clean = ext.trim_start_matches('.').to_ascii_lowercase();
        match clean.as_str() {
            "md" | "markdown" => Some(Self::Markdown),
            "txt" => Some(Self::Text),
            "json" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            "yaml" | "yml" => Some(Self::Yaml),
            "ini" => Some(Self::Ini),
            "rs" => Some(Self::Rust),
            "ts" | "tsx" => Some(Self::Typescript),
            "js" | "jsx" => Some(Self::Javascript),
            "py" => Some(Self::Python),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "kt" => Some(Self::Kotlin),
            "c" | "h" => Some(Self::C),
            "cpp" => Some(Self::Cpp),
            "cs" => Some(Self::Csharp),
            "rb" => Some(Self::Ruby),
            "php" => Some(Self::Php),
            "swift" => Some(Self::Swift),
            "sh" => Some(Self::Shell),
            "sql" => Some(Self::Sql),
            "html" => Some(Self::Html),
            "css" => Some(Self::Css),
            "scss" => Some(Self::Scss),
            "vue" => Some(Self::Vue),
            "svelte" => Some(Self::Svelte),
            "lua" => Some(Self::Lua),
            _ => None,
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }

    pub fn is_known(&self) -> bool {
        !matches!(self, Self::Other(_))
    }
}

impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from_str(&s).unwrap_or(Self::Other(s)))
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Language {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.trim().to_ascii_lowercase();
        let lang = match clean.as_str() {
            "markdown" => Self::Markdown,
            "text" => Self::Text,
            "json" => Self::Json,
            "toml" => Self::Toml,
            "yaml" => Self::Yaml,
            "ini" => Self::Ini,
            "rust" => Self::Rust,
            "typescript" => Self::Typescript,
            "javascript" => Self::Javascript,
            "python" => Self::Python,
            "go" => Self::Go,
            "java" => Self::Java,
            "kotlin" => Self::Kotlin,
            "c" => Self::C,
            "cpp" => Self::Cpp,
            "csharp" => Self::Csharp,
            "ruby" => Self::Ruby,
            "php" => Self::Php,
            "swift" => Self::Swift,
            "shell" => Self::Shell,
            "sql" => Self::Sql,
            "html" => Self::Html,
            "css" => Self::Css,
            "scss" => Self::Scss,
            "vue" => Self::Vue,
            "svelte" => Self::Svelte,
            "lua" => Self::Lua,
            _ => Self::Other(clean),
        };
        Ok(lang)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterKey {
    Language,
    Tag,
    Project,
    Extension,
    Type,
}

impl FilterKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Language => "language",
            Self::Tag => "tag",
            Self::Project => "project",
            Self::Extension => "extension",
            Self::Type => "type",
        }
    }
}

impl fmt::Display for FilterKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for FilterKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "language" => Ok(Self::Language),
            "tag" | "tags" => Ok(Self::Tag),
            "project" => Ok(Self::Project),
            "extension" | "ext" => Ok(Self::Extension),
            "type" => Ok(Self::Type),
            other => Err(format!("Unknown filter key: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_type_mapping() {
        assert_eq!(DocumentType::from_extension("md"), Some(DocumentType::Doc));
        assert_eq!(DocumentType::from_extension("txt"), Some(DocumentType::Doc));
        assert_eq!(DocumentType::from_extension("rs"), Some(DocumentType::Code));
        assert_eq!(DocumentType::from_extension("py"), Some(DocumentType::Code));
        assert_eq!(
            DocumentType::from_extension("json"),
            Some(DocumentType::Config)
        );
        assert_eq!(
            DocumentType::from_extension("toml"),
            Some(DocumentType::Config)
        );
        assert_eq!(DocumentType::from_extension("unknown"), None);
    }

    #[test]
    fn test_language_mapping() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("ts"), Some(Language::Typescript));
        assert_eq!(Language::from_extension("tsx"), Some(Language::Typescript));
        assert_eq!(Language::from_extension("md"), Some(Language::Markdown));
        assert_eq!(Language::from_extension("yaml"), Some(Language::Yaml));
        assert_eq!(Language::from_extension("yml"), Some(Language::Yaml));
    }

    #[test]
    fn test_filter_key_parsing() {
        assert_eq!(
            FilterKey::from_str("language").unwrap(),
            FilterKey::Language
        );
        assert_eq!(FilterKey::from_str("tag").unwrap(), FilterKey::Tag);
        assert_eq!(FilterKey::from_str("tags").unwrap(), FilterKey::Tag);
        assert_eq!(FilterKey::from_str("project").unwrap(), FilterKey::Project);
        assert_eq!(FilterKey::from_str("ext").unwrap(), FilterKey::Extension);
        assert_eq!(FilterKey::from_str("type").unwrap(), FilterKey::Type);
        assert!(FilterKey::from_str("invalid").is_err());
    }

    #[test]
    fn test_serde_json_compatibility() {
        let dt = DocumentType::Code;
        assert_eq!(serde_json::to_string(&dt).unwrap(), "\"code\"");

        let lang = Language::Rust;
        assert_eq!(serde_json::to_string(&lang).unwrap(), "\"rust\"");

        let fk = FilterKey::Language;
        assert_eq!(serde_json::to_string(&fk).unwrap(), "\"language\"");
    }
}
