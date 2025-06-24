// src/clanopedia_backend/src/lib.rs

use candid::Principal;
use getrandom::getrandom;
use ic_cdk::api::caller;
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use ic_cdk::api::time;
use ic_cdk::{query, update};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager},
    DefaultMemoryImpl, StableBTreeMap,
};
use std::cell::RefCell;

mod cycles;
mod external;
mod extractor;
mod governance;
mod storage;
mod types;
mod utils;

// Re-export specific types and functions
pub use types::{
    BluebandConfig, BluebandDocument, ClanopediaError, ClanopediaResult, Collection,
    CollectionConfig, CollectionId, DocumentId, DocumentRequest, GovernanceModel,
    GovernanceModelConfig, Proposal, ProposalId, ProposalStatus, ProposalType, SearchResult, Vote,
    PROPOSAL_DURATION_NANOS,
};

pub use external::blueband::{get_collection_metrics, CollectionMetrics};
pub use external::{
    add_document_to_blueband, create_blueband_collection, delete_collection, delete_document,
    embed_existing_document, fund_blueband_cycles, get_blueband_cycles_balance,
    get_document_content_from_blueband, get_document_metadata, get_token_balance,
    get_token_total_supply, transfer_genesis_admin, BluebandResult, BluebandService,
    DocumentMetadata, MemorySearchResult, SearchRequest, TokenResult, TokenService, VectorMatch,
};

pub use extractor::{
    AddDocumentsResult, DocumentAction, ExtractionInfo, ExtractionProgress, ExtractionResponse,
    ExtractionResult, ExtractionSource, ExtractionStatus, Extractor, FileExtractionConfig,
    FileType, UrlType, YouTubeVideoInfo,
};

pub use cycles::{estimate_embedding_cost, CyclesStatus};

use crate::external::blueband::AddDocumentRequest;

// use crate::extractor::{};

type Memory = ic_stable_structures::memory_manager::VirtualMemory<DefaultMemoryImpl>;

// Memory manager for stable storage
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );
}

// Global state for Blueband canister ID
thread_local! {
    static BLUEBAND_CANISTER_ID: RefCell<StableBTreeMap<(), Principal, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
        )
    );
}

pub fn set_blueband_canister_id(canister_id: Principal) {
    BLUEBAND_CANISTER_ID.with(|id| {
        id.borrow_mut().insert((), canister_id);
    });
}

pub fn get_blueband_canister_id() -> ClanopediaResult<Principal> {
    BLUEBAND_CANISTER_ID.with(|id| {
        id.borrow().get(&()).ok_or_else(|| {
            ClanopediaError::InvalidInput("Blueband canister not initialized".to_string())
        })
    })
}

// Helper function to check if a user is an admin of a collection
fn is_admin(collection_id: &str, user: Principal) -> bool {
    match storage::get_collection(&collection_id.to_string()) {
        Ok(collection) => collection.admins.contains(&user),
        Err(_) => false,
    }
}

// ============================
// COLLECTION MANAGEMENT
// ============================

#[update]
fn configure_blueband_canister(canister_id: Principal) -> ClanopediaResult<()> {
    // Only allow if not already set (for safety)
    if BLUEBAND_CANISTER_ID.with(|id| id.borrow().contains_key(&())) {
        return Err(ClanopediaError::InvalidOperation(
            "Blueband canister already configured".to_string(),
        ));
    }

    set_blueband_canister_id(canister_id);
    Ok(())
}

#[query]
fn get_collection(collection_id: String) -> ClanopediaResult<Collection> {
    storage::get_collection(&collection_id)
}

#[query]
fn list_collections() -> ClanopediaResult<Vec<Collection>> {
    Ok(storage::list_collections())
}

#[update]
async fn create_collection_endpoint(config: CollectionConfig) -> ClanopediaResult<CollectionId> {
    let caller = ic_cdk::caller();

    // Generate a random number using getrandom
    let mut random_bytes = [0u8; 4];
    getrandom(&mut random_bytes).map_err(|e| {
        ClanopediaError::InvalidInput(format!("Failed to generate random bytes: {}", e))
    })?;
    let random_number = u32::from_be_bytes(random_bytes);

    // Generate a unique collection ID that's shorter and meets Blueband's requirements
    // Use first 8 chars of caller, last 4 digits of timestamp, and 4 random hex chars
    let caller_short = caller.to_string().chars().take(8).collect::<String>();
    let timestamp = time().to_string();
    let timestamp_short = timestamp
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect::<String>();
    let random_hex = format!("{:04x}", random_number % 0xFFFF);
    let collection_id = format!("col_{}_{}_{}", caller_short, timestamp_short, random_hex);

    // Create collection in Blueband first
    let blueband_collection = create_blueband_collection(
        collection_id.clone(),
        config.name.clone(),
        config.description.clone(),
    )
    .await
    .map_err(|e| ClanopediaError::BluebandError(e.to_string()))?;

    // Convert string representations to Principal objects for validation
    let admins: Result<Vec<Principal>, _> = config
        .admins
        .iter()
        .map(Principal::from_text)
        .collect();

    let governance_token = config
        .governance_token
        .as_ref()
        .map(Principal::from_text)
        .transpose()
        .map_err(|e| {
            ClanopediaError::InvalidInput(format!("Invalid governance token principal: {}", e))
        })?;

    // Create collection in Clanopedia storage with Blueband ID
    let mut collection_config = config;

    // If any principal is invalid, use just the caller
    if admins.is_err() {
        collection_config.admins = vec![caller.to_string()];
    } else {
        let admins = admins.unwrap();
        if admins.is_empty() {
            collection_config.admins = vec![caller.to_string()];
        }
    }

    // Update the config with validated principals
    collection_config.governance_token = governance_token.map(|p| p.to_string());

    storage::create_collection(&collection_id, collection_config, caller)?;

    // Update the collection with Blueband ID
    let mut collection = storage::get_collection(&collection_id)?;
    collection.blueband_collection_id = blueband_collection.id;
    storage::update_collection(&collection_id, &collection)?;

    Ok(collection_id)
}

#[update]
async fn update_collection(
    collection_id: CollectionId,
    mut config: CollectionConfig,
) -> ClanopediaResult<()> {
    let caller = ic_cdk::caller();
    let collection = storage::get_collection(&collection_id)?;

    if !collection.admins.contains(&caller) {
        return Err(ClanopediaError::NotAuthorized);
    }

    // Convert string representations to Principal objects for validation
    let admins: Result<Vec<Principal>, _> = config
        .admins
        .iter()
        .map(|p| Principal::from_text(p))
        .collect();

    let governance_token = config
        .governance_token
        .as_ref()
        .map(|t| Principal::from_text(t))
        .transpose()
        .map_err(|e| {
            ClanopediaError::InvalidInput(format!("Invalid governance token principal: {}", e))
        })?;

    // If any principal is invalid, keep existing admins
    if admins.is_err() {
        config.admins = collection.admins.iter().map(|p| p.to_string()).collect();
    }

    let mut updated_collection = collection.clone();
    updated_collection.name = config.name;
    updated_collection.description = config.description;
    updated_collection.admins = admins.unwrap_or_else(|_| collection.admins.clone());
    updated_collection.threshold = config.threshold;
    updated_collection.governance_token = governance_token;
    updated_collection.governance_model = config.governance_model;
    updated_collection.quorum_threshold = config.quorum_threshold;
    updated_collection.is_permissionless = config.is_permissionless;
    updated_collection.updated_at = time();

    storage::update_collection(&collection_id, &updated_collection)?;
    Ok(())
}

#[update]
async fn delete_collection_endpoint(collection_id: CollectionId) -> ClanopediaResult<()> {
    let caller = ic_cdk::caller();
    governance::delete_collection(&collection_id, caller).await
}

// ============================
// DOCUMENT OPERATIONS
// ============================

#[update]
async fn get_document_endpoint(
    collection_id: CollectionId,
    document_id: DocumentId,
) -> ClanopediaResult<Option<String>> {
    let collection = storage::get_collection(&collection_id)?;
    get_document_content_from_blueband(&collection.blueband_collection_id, &document_id)
        .await
        .map_err(ClanopediaError::BluebandError)
}

// ============================
// GOVERNANCE OPERATIONS
// ============================

#[query]
fn get_proposals_endpoint(collection_id: String) -> ClanopediaResult<Vec<Proposal>> {
    governance::get_proposals(&collection_id)
}

#[update]
async fn create_proposal(
    collection_id: String,
    proposal_type: ProposalType,
    description: String,
) -> ClanopediaResult<ProposalId> {
    let caller = ic_cdk::caller();
    governance::create_proposal(&collection_id, proposal_type, caller, description).await
}

#[update]
async fn vote_on_proposal_endpoint(
    collection_id: String,
    proposal_id: String,
    vote: Vote,
) -> ClanopediaResult<()> {
    governance::vote_on_proposal(&collection_id, &proposal_id, vote).await
}

#[update]
async fn execute_proposal_endpoint(
    collection_id: String,
    proposal_id: String,
) -> ClanopediaResult<()> {
    governance::execute_proposal(&collection_id, &proposal_id).await
}

#[query]
fn get_proposal_status_endpoint(
    collection_id: String,
    proposal_id: String,
) -> ClanopediaResult<ProposalStatus> {
    governance::get_proposal_status(&collection_id, proposal_id)
}

#[query]
fn can_execute_directly_endpoint(collection_id: String) -> ClanopediaResult<bool> {
    governance::can_execute_directly(&collection_id)
}

// ============================
// ADMIN OPERATIONS
// ============================

#[update]
async fn create_admin_proposal(
    collection_id: String,
    new_admin: Principal,
) -> ClanopediaResult<ProposalId> {
    let caller = caller();
    let proposal_type = ProposalType::AddAdmin { admin: new_admin };
    governance::create_proposal(
        &collection_id,
        proposal_type,
        caller,
        "Add new admin".to_string(),
    )
    .await
}

#[update]
async fn create_remove_admin_proposal(
    collection_id: String,
    admin_to_remove: Principal,
) -> ClanopediaResult<ProposalId> {
    let caller = caller();
    let proposal_type = ProposalType::RemoveAdmin {
        admin: admin_to_remove,
    };
    governance::create_proposal(
        &collection_id,
        proposal_type,
        caller,
        "Remove admin".to_string(),
    )
    .await
}

// ============================
//  EXTRACTOR OPERATIONS
// ============================

#[update]
async fn extract_from_file(
    file_data: Vec<u8>,
    filename: String,
    collection_id: String,
) -> ClanopediaResult<ExtractionResponse> {
    let caller = ic_cdk::caller();

    // Verify the caller is an admin of the collection
    let collection = storage::get_collection(&collection_id)?;
    if !collection.admins.contains(&caller) {
        return Err(ClanopediaError::NotAuthorized);
    }

    ic_cdk::println!(
        "File extraction request from {}: {} ({} bytes) -> {}",
        caller,
        filename,
        file_data.len(),
        collection_id
    );

    // Extract content
    let documents = extractor::Extractor::extract_from_file(file_data, filename, collection_id)?;

    // File extraction is always complete (no pagination)
    let extraction_info = ExtractionInfo::for_file_extraction(documents.len() as u32);

    Ok(ExtractionResponse {
        documents,
        extraction_info,
    })
}

#[update]
async fn extract_from_url(
    url: String,
    collection_id: String,
    api_key: Option<String>,
) -> ClanopediaResult<ExtractionResponse> {
    let caller = ic_cdk::caller();

    // Add detailed logging for debugging
    ic_cdk::println!(
        "URL extraction request - Caller: {}, Collection: {}, URL: {}",
        caller,
        collection_id,
        url
    );

    let collection = storage::get_collection(&collection_id)?;

    // Log collection admins and caller for debugging
    ic_cdk::println!(
        "Collection admins: {:?}, Caller: {}",
        collection.admins,
        caller
    );

    if !collection.admins.contains(&caller) {
        ic_cdk::println!(
            "Authorization failed - Caller {} not in admins list: {:?}",
            caller,
            collection.admins
        );
        return Err(ClanopediaError::NotAuthorized);
    }

    ic_cdk::println!(
        "Authorization successful - proceeding with extraction for {}",
        caller
    );

    let documents =
        extractor::Extractor::extract_from_url(url.clone(), collection_id.clone(), api_key).await?;

    let progress = extractor::Extractor::get_progress(&collection_id, &url);

    let extraction_info = if let Some(progress) = progress {
        ExtractionInfo::from_progress(&progress)
    } else {
        ExtractionInfo::for_file_extraction(documents.len() as u32)
    };

    Ok(ExtractionResponse {
        documents,
        extraction_info,
    })
}

#[update]
async fn add_extracted_documents(
    collection_id: String,
    documents: Vec<AddDocumentRequest>,
) -> ClanopediaResult<AddDocumentsResult> {
    let caller = ic_cdk::caller();

    // Verify the caller is an admin of the collection
    let collection = storage::get_collection(&collection_id)?;
    if !collection.admins.contains(&caller) {
        return Err(ClanopediaError::NotAuthorized);
    }

    if documents.is_empty() {
        return Err(ClanopediaError::InvalidInput(
            "No documents to add".to_string(),
        ));
    }

    ic_cdk::println!(
        "Adding {} extracted documents to collection {}",
        documents.len(),
        collection_id
    );

    let total_docs = documents.len();
    let mut document_ids = Vec::new();
    let mut processed_count = 0;

    // Add documents to Blueband
    for doc_request in documents {
        let title = doc_request.title.clone();
        ic_cdk::println!("Adding document: {}", title);

        // Convert AddDocumentRequest to DocumentRequest
        let document_request = DocumentRequest {
            title: doc_request.title,
            content: doc_request.content,
            content_type: doc_request.content_type,
            source_url: doc_request.source_url,
            author: doc_request.author,
            tags: doc_request.tags,
        };

        let metadata =
            add_document_to_blueband(&collection.blueband_collection_id, document_request)
                .await
                .map_err(|e| {
                    ic_cdk::println!("Error adding document {}: {}", title, e);
                    ClanopediaError::BluebandError(e)
                })?;

        document_ids.push(metadata.id.clone());
        processed_count += 1;
        ic_cdk::println!(
            "Successfully added document: {} ({}/{})",
            metadata.id,
            processed_count,
            total_docs
        );
    }

    // Create proposal for embedding
    let doc_count = document_ids.len();
    let proposal_type = ProposalType::BatchEmbed {
        document_ids: document_ids.clone(),
    };

    let description = format!(
        "Embed {} extracted documents into the collection. Documents: [{}]",
        doc_count,
        document_ids
            .iter()
            .take(3) // Show first 3 IDs
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );

    let proposal_id = governance::create_proposal(
        &collection_id,
        proposal_type,
        caller,
        description
    ).await?;

    // Clone proposal_id for the message
    let proposal_id_clone = proposal_id.clone();

    // Get governance type for the message
    let governance_type = match collection.governance_model {
        GovernanceModel::Permissionless => "permissionless",
        GovernanceModel::Multisig => "multisig",
        GovernanceModel::TokenBased => "token-based",
        GovernanceModel::SnsIntegrated => "SNS-integrated",
    };

    // Create the result with the cloned proposal_id
    let result = AddDocumentsResult {
        document_ids,
        proposal_id: Some(proposal_id_clone.clone()),
        action: DocumentAction::ProposalCreated,
        message: format!(
            "Successfully added {} documents. Proposal {} created for {} governance approval",
            doc_count, proposal_id_clone, governance_type
        ),
    };

    Ok(result)
}

#[update]
fn cleanup_extraction_progress_endpoint(
    collection_id: String,
    url: String,
) -> ClanopediaResult<()> {
    let caller = ic_cdk::caller();

    // Verify the caller is an admin of the collection
    let collection = storage::get_collection(&collection_id)?;
    if !collection.admins.contains(&caller) {
        return Err(ClanopediaError::NotAuthorized);
    }

    // Only allow cleanup of completed/failed extractions
    if let Some(progress) = extractor::Extractor::get_progress(&collection_id, &url) {
        match progress.status {
            ExtractionStatus::Completed | ExtractionStatus::Failed(_) => {
                extractor::Extractor::remove_progress(&collection_id, &url);
                Ok(())
            }
            _ => Err(ClanopediaError::InvalidOperation(
                "Cannot cleanup active or paused extractions".to_string(),
            )),
        }
    } else {
        Err(ClanopediaError::InvalidInput(
            "No extraction found for this URL and collection".to_string(),
        ))
    }
}

// ============================
// EXTRACTION STATUS ENDPOINTS
// ============================

/// Get extraction progress for a specific URL/collection
#[query]
fn get_extraction_progress(collection_id: String, url: String) -> Option<ExtractionProgress> {
    extractor::Extractor::get_progress(&collection_id, &url)
}

/// Get all active extractions for a collection
#[query]
fn get_collection_extractions_endpoint(collection_id: String) -> Vec<ExtractionProgress> {
    extractor::Extractor::get_collection_extractions(collection_id)
}

/// Get extraction statistics
#[query]
fn get_extraction_stats_endpoint() -> (u64, u64, u64) {
    extractor::get_extraction_stats()
}

/// Clean up old completed extractions (system maintenance)
#[update]
fn cleanup_old_extractions_endpoint() -> u32 {
    extractor::cleanup_old_extractions()
}

// ============================
// EXTRACTION INFO ENDPOINTS
// ============================

#[query]
fn get_supported_file_types() -> Vec<String> {
    vec![
        "txt".to_string(),
        "md".to_string(),
        "markdown".to_string(),
        "pdf".to_string(),
        "docx".to_string(),
    ]
}

#[query]
fn get_supported_url_types() -> Vec<String> {
    vec![
        "YouTube playlists".to_string(),
        "GitHub markdown files".to_string(),
    ]
}

#[query]
fn get_extraction_limits() -> String {
    format!(
        "File size limit: {} MB\nContent size limit: {} MB\nYouTube playlist limit: 50 videos per batch\nGitHub file limit: 2 MB",
        10,
        10, 
    )
}

#[update]
async fn sync_sns_proposal_status_and_update_endpoint(
    collection_id: String,
    proposal_id: String,
) -> ClanopediaResult<()> {
    crate::governance::sync_sns_proposal_status_and_update(&collection_id, &proposal_id).await
}

#[query]
fn is_sns_integrated_endpoint(collection_id: String) -> ClanopediaResult<bool> {
    let collection = storage::get_collection(&collection_id)?;
    Ok(collection.governance_model == GovernanceModel::SnsIntegrated)
}

#[query]
fn get_sns_governance_canister_endpoint(collection_id: String) -> ClanopediaResult<Option<Principal>> {
    let collection = storage::get_collection(&collection_id)?;
    
    if collection.governance_model == GovernanceModel::SnsIntegrated {
        Ok(collection.sns_governance_canister)
    } else {
        Ok(None)
    }
}

#[update]
fn link_sns_proposal_id_endpoint(
    collection_id: String,
    proposal_id: String,
    sns_proposal_id: u64,
) -> ClanopediaResult<()> {
    let caller = ic_cdk::caller();
    crate::governance::link_sns_proposal_id(&collection_id, &proposal_id, sns_proposal_id, caller)
}

#[query]
fn is_admin_check(collection_id: CollectionId, user: Principal) -> bool {
    is_admin(&collection_id, user)
}

#[update]
async fn embed_single_document(
    collection_id: String,
    document: AddDocumentRequest,
) -> ClanopediaResult<DocumentMetadata> {
    let caller = ic_cdk::caller();
    let collection = storage::get_collection(&collection_id)?;
    if !collection.admins.contains(&caller) {
        return Err(ClanopediaError::NotAuthorized);
    }
    // Convert AddDocumentRequest to DocumentRequest
    let document_request = DocumentRequest {
        title: document.title,
        content: document.content,
        content_type: document.content_type,
        source_url: document.source_url,
        author: document.author,
        tags: document.tags,
    };
    // Add document to Blueband
    add_document_to_blueband(&collection.blueband_collection_id, document_request)
        .await
        .map_err(ClanopediaError::BluebandError)
}

#[update]
async fn get_collection_metrics_endpoint(
    collection_id: String,
) -> ClanopediaResult<CollectionMetrics> {
    let collection = storage::get_collection(&collection_id)?;
    external::blueband::get_collection_metrics(&collection.blueband_collection_id)
        .await
        .map_err(ClanopediaError::BluebandError)
}

// Export candid interface
ic_cdk::export_candid!();





