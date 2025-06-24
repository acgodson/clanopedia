// src/extractor/mod.rs

pub mod file_extractor;
pub mod url_extractor;
pub mod types;

pub use types::*;
use crate::{AddDocumentRequest, ClanopediaResult, ClanopediaError};
use ic_cdk::api::time;
use ic_stable_structures::{
    memory_manager::{MemoryManager, MemoryId},
    DefaultMemoryImpl, StableBTreeMap,
};
use ic_stable_structures::storable::Storable;
use std::cell::RefCell;

// Memory ID for extraction progress storage
const EXTRACTION_PROGRESS_MEMORY_ID: MemoryId = MemoryId::new(10);

// Memory manager for stable storage
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );
}

/// Get memory for extraction progress storage
fn get_extraction_memory() -> ic_stable_structures::memory_manager::VirtualMemory<DefaultMemoryImpl> {
    MEMORY_MANAGER.with(|m| m.borrow().get(EXTRACTION_PROGRESS_MEMORY_ID))
}

// Key for the progress map: (collection_id, url)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ProgressKey {
    collection_id: String,
    url: String,
}

impl ProgressKey {
    fn new(collection_id: String, url: String) -> Self {
        Self { collection_id, url }
    }
}

impl Storable for ProgressKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(candid::encode_one((&self.collection_id, &self.url)).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        candid::decode_one(&bytes)
            .map(|(c, u): (String, String)| Self { collection_id: c, url: u })
            .unwrap_or_else(|_| Self { collection_id: String::new(), url: String::new() })
    }

    const BOUND: ic_stable_structures::storable::Bound = ic_stable_structures::storable::Bound::Bounded {
        max_size: 1024,
        is_fixed_size: false,
    };
}

// Global stable storage for extraction progress
thread_local! {
    static EXTRACTION_PROGRESS: RefCell<StableBTreeMap<ProgressKey, ExtractionProgress, ic_stable_structures::memory_manager::VirtualMemory<DefaultMemoryImpl>>> = 
        RefCell::new(StableBTreeMap::init(get_extraction_memory()));
}

pub struct Extractor;

impl Extractor {
    /// Extract content from uploaded file buffer
    pub fn extract_from_file(
        file_data: Vec<u8>,
        filename: String,
        collection_id: String,
    ) -> ClanopediaResult<Vec<AddDocumentRequest>> {
        file_extractor::extract_file_content(file_data, filename, collection_id)
    }

    /// Extract content from URL (YouTube, GitHub, etc.)
    pub async fn extract_from_url(
        url: String,
        collection_id: String,
        api_key: Option<String>,
    ) -> ClanopediaResult<Vec<AddDocumentRequest>> {
        url_extractor::extract_url_content(url, collection_id, api_key).await
    }

    /// Batch extract from multiple sources
    pub async fn batch_extract(
        sources: Vec<ExtractionSource>,
        collection_id: String,
    ) -> ClanopediaResult<Vec<AddDocumentRequest>> {
        let mut all_documents = Vec::new();

        for source in sources {
            let documents = match source {
                ExtractionSource::File { data, filename } => {
                    Self::extract_from_file(data, filename, collection_id.clone())?
                }
                ExtractionSource::Url { url, api_key } => {
                    Self::extract_from_url(url, collection_id.clone(), api_key).await?
                }
            };
            all_documents.extend(documents);
        }

        Ok(all_documents)
    }

    /// Get the current progress of an extraction
    pub fn get_progress(collection_id: &str, url: &str) -> Option<ExtractionProgress> {
        EXTRACTION_PROGRESS.with(|progress| {
            progress.borrow().get(&ProgressKey::new(collection_id.to_string(), url.to_string()))
        })
    }

    /// Update the progress of an extraction
    pub fn update_progress(progress: ExtractionProgress) {
        let key = ProgressKey::new(progress.collection_id.clone(), progress.url.clone());
        EXTRACTION_PROGRESS.with(|p| {
            p.borrow_mut().insert(key, progress);
        });
    }

    /// Remove progress tracking for a URL
    pub fn remove_progress(collection_id: &str, url: &str) {
        EXTRACTION_PROGRESS.with(|progress| {
            progress.borrow_mut().remove(&ProgressKey::new(collection_id.to_string(), url.to_string()));
        });
    }

    /// Create an ExtractionResponse with proper info
    pub fn create_response(
        documents: Vec<AddDocumentRequest>,
        progress: Option<ExtractionProgress>,
    ) -> ExtractionResponse {
        let extraction_info = if let Some(progress) = progress {
            ExtractionInfo::from_progress(&progress)
        } else {
            ExtractionInfo::for_file_extraction(documents.len() as u32)
        };

        ExtractionResponse {
            documents,
            extraction_info,
        }
    }

    /// Create a failed response
    pub fn create_failed_response(error_message: String) -> ExtractionResponse {
        ExtractionResponse {
            documents: Vec::new(),
            extraction_info: ExtractionInfo::for_failed_extraction(error_message),
        }
    }

    /// Get all extraction progress for a collection
    pub fn get_collection_extractions(collection_id: String) -> Vec<ExtractionProgress> {
        EXTRACTION_PROGRESS.with(|progress| {
            progress.borrow()
                .iter()
                .filter_map(|(key, progress)| {
                    if key.collection_id == collection_id {
                        Some(progress)
                    } else {
                        None
                    }
                })
                .collect()
        })
    }
}

pub fn sanitize_content(content: &str) -> String {
    // Remove excessive whitespace, normalize line endings
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn validate_content_size(content: &str) -> ClanopediaResult<()> {
    const MAX_SIZE: usize = 10 * 1024 * 1024; // 10MB limit for Blueband
    
    if content.len() > MAX_SIZE {
        return Err(ClanopediaError::InvalidInput(
            format!("Content too large: {} bytes (max: {} bytes)", content.len(), MAX_SIZE)
        ));
    }
    
    Ok(())
}

/// Helper function to get extraction statistics
#[ic_cdk::query]
pub fn get_extraction_stats() -> (u64, u64, u64) {
    EXTRACTION_PROGRESS.with(|progress| {
        let map = progress.borrow();
        let total = map.len();
        let (in_progress, paused) = map.iter().fold((0u64, 0u64), |(in_prog, paused), (_, prog)| {
            match prog.status {
                ExtractionStatus::InProgress => (in_prog + 1, paused),
                ExtractionStatus::Paused => (in_prog, paused + 1),
                _ => (in_prog, paused),
            }
        });
        (total, in_progress, paused)
    })
}

/// Helper function to clean up old completed extractions
#[ic_cdk::update]
pub fn cleanup_old_extractions() -> u32 {
    let cutoff_time = time() - (7 * 24 * 60 * 60 * 1_000_000_000); // 7 days ago
    let mut cleaned = 0u32;
    
    EXTRACTION_PROGRESS.with(|progress| {
        let mut map = progress.borrow_mut();
        let keys_to_remove: Vec<ProgressKey> = map.iter()
            .filter_map(|(key, prog)| {
                if matches!(prog.status, ExtractionStatus::Completed | ExtractionStatus::Failed(_)) 
                   && prog.last_updated < cutoff_time {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();
        
        for key in keys_to_remove {
            map.remove(&key);
            cleaned += 1;
        }
    });
    
    cleaned
}

/// Resume extraction from where it left off
#[ic_cdk::update]
pub async fn resume_extraction(
    collection_id: String,
    url: String,
    api_key: Option<String>,
) -> ClanopediaResult<Vec<AddDocumentRequest>> {
    let progress = EXTRACTION_PROGRESS.with(|p| {
        p.borrow().get(&ProgressKey::new(collection_id.clone(), url.clone()))
    }).ok_or_else(|| 
        ClanopediaError::InvalidInput("No extraction in progress for this URL".to_string())
    )?;

    if !matches!(progress.status, ExtractionStatus::Paused | ExtractionStatus::Failed(_)) {
        return Err(ClanopediaError::InvalidInput(
            "Extraction is not paused or failed".to_string()
        ));
    }

    // Resume the extraction
    url_extractor::extract_url_content(url, collection_id, api_key).await
}

/// Clean up completed or failed extraction progress
#[ic_cdk::update]
pub fn cleanup_extraction_progress(collection_id: String, url: String) -> ClanopediaResult<()> {
    EXTRACTION_PROGRESS.with(|progress| {
        progress.borrow_mut().remove(&ProgressKey::new(collection_id, url));
    });
    Ok(())
}