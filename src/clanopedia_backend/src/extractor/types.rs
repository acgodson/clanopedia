// src/extractor/types.rs

use crate::{external::blueband::ContentType, DocumentId};
use crate::{AddDocumentRequest, ProposalId};
use candid::CandidType;
use ic_stable_structures::storable::Storable;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum ExtractionSource {
    File {
        data: Vec<u8>,
        filename: String,
    },
    Url {
        url: String,
        api_key: Option<String>,
    },
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FileType {
    Pdf,
    DocX,
    PlainText,
    Markdown,
    Unknown,
}

impl FileType {
    pub fn from_filename(filename: &str) -> Self {
        let extension = filename
            .rfind('.')
            .map(|i| &filename[i + 1..])
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "pdf" => FileType::Pdf,
            "docx" | "doc" => FileType::DocX,
            "txt" => FileType::PlainText,
            "md" | "markdown" => FileType::Markdown,
            _ => FileType::Unknown,
        }
    }

    pub fn to_content_type(&self) -> ContentType {
        match self {
            FileType::Pdf => ContentType::PlainText,
            FileType::DocX => ContentType::PlainText,
            FileType::PlainText => ContentType::PlainText,
            FileType::Markdown => ContentType::Markdown,
            FileType::Unknown => ContentType::PlainText,
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum UrlType {
    YouTube,
    GitHub,
    Unknown,
}

impl UrlType {
    pub fn from_url(url: &str) -> Self {
        if url.contains("youtube.com") || url.contains("youtu.be") {
            UrlType::YouTube
        } else if url.contains("github.com") {
            UrlType::GitHub
        } else {
            UrlType::Unknown
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct ExtractionResult {
    pub title: String,
    pub content: String,
    pub content_type: ContentType,
    pub source_url: Option<String>,
    pub metadata: Option<ExtractionMetadata>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct ExtractionMetadata {
    pub file_size: Option<u64>,
    pub page_count: Option<u32>,
    pub author: Option<String>,
    pub created_at: Option<u64>,
    pub tags: Option<Vec<String>>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct YouTubeVideoInfo {
    pub title: String,
    pub description: Option<String>,
    pub video_id: String,
    pub published_at: u64,
    pub creator: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct FileExtractionConfig {
    pub max_file_size: u64,
    pub supported_types: Vec<FileType>,
    pub extract_metadata: bool,
}

impl Default for FileExtractionConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            supported_types: vec![
                FileType::Pdf,
                FileType::DocX,
                FileType::PlainText,
                FileType::Markdown,
            ],
            extract_metadata: true,
        }
    }
}

/// Structure to track extraction progress for a URL in a collection
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct ExtractionProgress {
    pub url: String,
    pub collection_id: String,
    pub playlist_id: String,
    pub next_page_token: Option<String>,
    pub total_videos: Option<u32>,
    pub processed_videos: u32,
    pub last_updated: u64,
    pub status: ExtractionStatus,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ExtractionStatus {
    InProgress,
    Completed,
    Failed(String),
    Paused,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct AddDocumentsResult {
    pub document_ids: Vec<DocumentId>,
    pub proposal_id: Option<ProposalId>,
    pub action: DocumentAction,
    pub message: String,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum DocumentAction {
    EmbeddedDirectly, // Documents were embedded immediately
    ProposalCreated,  // Governance proposal was created
}

impl Storable for ExtractionProgress {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(candid::encode_one(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap_or_else(|_| ExtractionProgress {
            url: String::new(),
            collection_id: String::new(),
            playlist_id: String::new(),
            next_page_token: None,
            total_videos: None,
            processed_videos: 0,
            last_updated: 0,
            status: ExtractionStatus::Failed("Failed to deserialize".to_string()),
        })
    }

    const BOUND: ic_stable_structures::storable::Bound =
        ic_stable_structures::storable::Bound::Bounded {
            max_size: 1024 * 1024, // 1MB max size
            is_fixed_size: false,
        };
}

/// Enhanced response structure that includes extraction info
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct ExtractionResponse {
    pub documents: Vec<AddDocumentRequest>,
    pub extraction_info: ExtractionInfo,
}

/// Information about the extraction process
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct ExtractionInfo {
    pub status: ExtractionStatus,
    pub processed_count: u32,
    pub total_count: Option<u32>,
    pub has_more: bool,
    pub can_resume: bool,
    pub summary_message: String,
}

impl ExtractionInfo {
    pub fn new(
        status: ExtractionStatus,
        processed_count: u32,
        total_count: Option<u32>,
        has_more: bool,
    ) -> Self {
        let can_resume = matches!(
            status,
            ExtractionStatus::Paused | ExtractionStatus::Failed(_)
        );

        let summary_message = match (&status, total_count, has_more) {
            (ExtractionStatus::Completed, Some(total), false) => {
                format!(
                    "Extraction completed successfully. {} documents processed.",
                    total
                )
            }
            (ExtractionStatus::Paused, Some(total), true) => {
                format!(
                    "Batch extraction successful: {}/{} documents processed. Resume to continue with remaining {} documents.",
                    processed_count,
                    total,
                    total - processed_count
                )
            }
            (ExtractionStatus::Paused, None, true) => {
                format!(
                    "Batch extraction successful: {} documents processed. More content available - resume to continue.",
                    processed_count
                )
            }
            (ExtractionStatus::InProgress, _, _) => "Extraction in progress...".to_string(),
            (ExtractionStatus::Failed(err), _, _) => {
                format!("Extraction failed: {}", err)
            }
            _ => format!("{} documents processed.", processed_count),
        };

        Self {
            status,
            processed_count,
            total_count,
            has_more,
            can_resume,
            summary_message,
        }
    }

    /// Create extraction info from progress
    pub fn from_progress(progress: &ExtractionProgress) -> Self {
        let has_more = matches!(progress.status, ExtractionStatus::Paused);
        Self::new(
            progress.status.clone(),
            progress.processed_videos,
            progress.total_videos,
            has_more,
        )
    }

    /// Create extraction info for file extraction (no pagination)
    pub fn for_file_extraction(documents_count: u32) -> Self {
        Self::new(
            ExtractionStatus::Completed,
            documents_count,
            Some(documents_count),
            false,
        )
    }

    /// Create extraction info for failed extraction
    pub fn for_failed_extraction(error_message: String) -> Self {
        Self::new(ExtractionStatus::Failed(error_message), 0, None, false)
    }
}

impl Storable for AddDocumentRequest {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(candid::encode_one(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap_or_else(|_| AddDocumentRequest {
            collection_id: String::new(),
            title: String::new(),
            content: String::new(),
            content_type: None,
            source_url: None,
            author: None,
            tags: None,
        })
    }

    const BOUND: ic_stable_structures::storable::Bound = ic_stable_structures::storable::Bound::Bounded {
        max_size: 1024 * 1024, // 1MB max size
        is_fixed_size: false,
    };
}
