use std::borrow::Cow;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

use crate::domain::models::{DocumentId, DocumentType, Language, SearchHighlight};

/// Custom validator untuk memastikan panjang query setelah normalisasi (trim) maksimal 500 karakter.
pub fn validate_search_query(q: &str) -> Result<(), ValidationError> {
    let trimmed = q.trim();
    if trimmed.chars().count() > 500 {
        let mut err = ValidationError::new("length");
        err.message = Some(Cow::Borrowed("Query maksimal 500 karakter"));
        return Err(err);
    }
    Ok(())
}

/// DTO query parameters untuk endpoint `GET /api/search`.
/// Memvalidasi nomor halaman (1-1000), ukuran halaman (1-100), dan batas query normalisasi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct SearchRequestDto {
    #[validate(custom(function = "validate_search_query"))]
    pub q: Option<String>,

    #[validate(range(min = 1, max = 1000, message = "Nomor halaman antara 1 hingga 1000"))]
    pub page: Option<u32>,

    #[validate(range(min = 1, max = 100, message = "Ukuran halaman antara 1 hingga 100"))]
    pub size: Option<u32>,

    #[serde(rename = "type")]
    pub doc_type: Option<String>,

    pub language: Option<String>,

    pub tag: Option<String>,

    pub project: Option<String>,

    pub sort: Option<String>,
}

impl SearchRequestDto {
    /// Mengambil string query yang telah dinormalisasi (trimmed).
    /// Mengembalikan `None` jika query bernilai `None`, `""`, atau hanya whitespace.
    pub fn normalized_query(&self) -> Option<&str> {
        self.q.as_deref().map(str::trim).filter(|s| !s.is_empty())
    }

    /// Nomor halaman aktif (default 1 jika None).
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1)
    }

    /// Ukuran item per halaman (default 20 jika None).
    pub fn size(&self) -> u32 {
        self.size.unwrap_or(20)
    }
}

/// DTO query parameters untuk endpoint `GET /api/suggest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct SuggestRequestDto {
    #[validate(length(
        min = 1,
        max = 500,
        message = "Query suggest minimal 1 karakter dan maksimal 500 karakter"
    ))]
    pub q: String,

    #[validate(range(min = 1, max = 50, message = "Limit suggest antara 1 hingga 50"))]
    pub limit: Option<u32>,
}

impl SuggestRequestDto {
    pub fn limit(&self) -> u32 {
        self.limit.unwrap_or(5)
    }
}

/// Satu item hasil pencarian dokumen yang disajikan ke client UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResultItemDto {
    pub id: DocumentId,
    pub title: String,
    pub relative_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(rename = "type")]
    pub doc_type: DocumentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<SearchHighlight>,
    pub score: f32,
    #[serde(rename = "file_size", alias = "file_size_bytes")]
    pub file_size: u64,
    #[serde(
        rename = "updated_at",
        alias = "modified_at",
        skip_serializing_if = "Option::is_none"
    )]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Bucket agregasi facet kategori pencarian.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetBucketDto {
    pub key: String,
    pub doc_count: u64,
}

/// Kelompok facet pencarian (types, languages, tags, projects).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchFacetsDto {
    #[serde(default)]
    pub types: Vec<FacetBucketDto>,
    #[serde(default)]
    pub languages: Vec<FacetBucketDto>,
    #[serde(default)]
    pub tags: Vec<FacetBucketDto>,
    #[serde(default)]
    pub projects: Vec<FacetBucketDto>,
}

/// Payload respons lengkap untuk endpoint `GET /api/search`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResponseDto {
    pub query: String,
    pub page: u32,
    pub size: u32,
    pub total: u64,
    pub took_ms: u64,
    pub items: Vec<SearchResultItemDto>,
    pub results: Vec<SearchResultItemDto>,
    pub facets: SearchFacetsDto,
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_request_dto_valid_defaults() {
        let dto = SearchRequestDto {
            q: Some("  rust async  ".to_string()),
            page: None,
            size: None,
            doc_type: None,
            language: None,
            tag: None,
            project: None,
            sort: None,
        };

        assert!(dto.validate().is_ok());
        assert_eq!(dto.normalized_query(), Some("rust async"));
        assert_eq!(dto.page(), 1);
        assert_eq!(dto.size(), 20);
    }

    #[test]
    fn test_search_request_dto_empty_queries() {
        let empty_dto = SearchRequestDto {
            q: Some("   \t \n  ".to_string()),
            page: Some(1),
            size: Some(10),
            doc_type: None,
            language: None,
            tag: None,
            project: None,
            sort: None,
        };

        assert!(empty_dto.validate().is_ok());
        assert_eq!(empty_dto.normalized_query(), None);

        let none_dto = SearchRequestDto {
            q: None,
            page: Some(1),
            size: Some(10),
            doc_type: None,
            language: None,
            tag: None,
            project: None,
            sort: None,
        };

        assert!(none_dto.validate().is_ok());
        assert_eq!(none_dto.normalized_query(), None);
    }

    #[test]
    fn test_search_request_dto_page_and_size_bounds() {
        // Page min bound
        let bad_page_min = SearchRequestDto {
            q: None,
            page: Some(0),
            size: Some(20),
            doc_type: None,
            language: None,
            tag: None,
            project: None,
            sort: None,
        };
        assert!(bad_page_min.validate().is_err());

        // Page max bound
        let bad_page_max = SearchRequestDto {
            q: None,
            page: Some(1001),
            size: Some(20),
            doc_type: None,
            language: None,
            tag: None,
            project: None,
            sort: None,
        };
        assert!(bad_page_max.validate().is_err());

        // Size min bound
        let bad_size_min = SearchRequestDto {
            q: None,
            page: Some(1),
            size: Some(0),
            doc_type: None,
            language: None,
            tag: None,
            project: None,
            sort: None,
        };
        assert!(bad_size_min.validate().is_err());

        // Size max bound
        let bad_size_max = SearchRequestDto {
            q: None,
            page: Some(1),
            size: Some(101),
            doc_type: None,
            language: None,
            tag: None,
            project: None,
            sort: None,
        };
        assert!(bad_size_max.validate().is_err());

        // Valid upper bounds
        let valid_bounds = SearchRequestDto {
            q: None,
            page: Some(1000),
            size: Some(100),
            doc_type: None,
            language: None,
            tag: None,
            project: None,
            sort: None,
        };
        assert!(valid_bounds.validate().is_ok());
    }

    #[test]
    fn test_search_request_dto_query_normalization_length() {
        // 500 characters + 10 surrounding whitespace -> valid after normalization
        let base_str = "a".repeat(500);
        let padded = format!("   {base_str}   ");

        let valid_padded = SearchRequestDto {
            q: Some(padded),
            page: Some(1),
            size: Some(20),
            doc_type: None,
            language: None,
            tag: None,
            project: None,
            sort: None,
        };
        assert!(valid_padded.validate().is_ok());

        // 501 characters -> invalid
        let too_long = "b".repeat(501);
        let invalid_dto = SearchRequestDto {
            q: Some(too_long),
            page: Some(1),
            size: Some(20),
            doc_type: None,
            language: None,
            tag: None,
            project: None,
            sort: None,
        };
        assert!(invalid_dto.validate().is_err());
    }
}
