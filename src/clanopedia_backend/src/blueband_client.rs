// src/clanopedia_backend/src/blueband_client.rs - Updated client functions to match lib.rs calls

use crate::blueband_interface::{
    BluebandService,
    CreateCollectionRequest,
    DocumentMetadata,
    AddDocumentRequest,
    ContentType,
    SearchRequest,
    Collection as BluebandCollection,
    BluebandResult,
};
use crate::types::*;
use candid::Principal;

// Get Blueband canister ID from global state
fn get_blueband_canister() -> ClanopediaResult<Principal> {
    super::get_blueband_canister_id()
}

// ============================
// COLLECTION OPERATIONS
// ============================

pub async fn create_blueband_collection(
    collection_id: String,
    name: String,
    description: String,
) -> BluebandResult<BluebandCollection> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;
    
    let service = BluebandService::new(blueband_canister);
    
    let request = CreateCollectionRequest {
        id: collection_id,
        name,
        description: Some(description),
        settings: None, // Use default settings
    };
    
    service.create_collection(request).await
}

// ============================
// DOCUMENT OPERATIONS  
// ============================

pub async fn add_document_to_blueband(
    collection_id: &str,
    document: DocumentRequest,
) -> BluebandResult<DocumentMetadata> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;
    
    let service = BluebandService::new(blueband_canister);
    
    // Convert Clanopedia DocumentRequest to Blueband AddDocumentRequest
    let request = AddDocumentRequest {
        title: document.title,
        content: document.content,
        content_type: Some(ContentType::PlainText), // Default to plain text
        collection_id: collection_id.to_string(),
        source_url: None,
    };
    
    service.add_document(request).await
}

pub async fn embed_existing_document(
    collection_id: &str,
    document_id: &str,
) -> BluebandResult<u32> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;
    
    let service = BluebandService::new(blueband_canister);
    service.embed_existing_document(
        collection_id.to_string(),
        document_id.to_string(),
    ).await
}

pub async fn get_document_content_from_blueband(
    collection_id: &str,
    document_id: &str,
) -> BluebandResult<Option<String>> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;
    
    let service = BluebandService::new(blueband_canister);
    Ok(service.get_document_content(
        collection_id.to_string(),
        document_id.to_string(),
    ).await)
}

pub async fn get_document_metadata(
    collection_id: String,
    document_id: String,
) -> BluebandResult<Option<DocumentMetadata>> {
    let blueband_canister = get_blueband_canister().map_err(|e| format!("{:?}", e))?;
    let service = BluebandService::new(blueband_canister);
    Ok(service.get_document(collection_id, document_id).await)
}

pub async fn delete_document(
    collection_id: &str,
    document_id: &str,
) -> BluebandResult<()> {
    let blueband_canister = get_blueband_canister().map_err(|e| format!("{:?}", e))?;
    let service = BluebandService::new(blueband_canister);
    service.delete_document(collection_id.to_string(), document_id.to_string()).await
}

pub async fn delete_collection(collection_id: &str) -> BluebandResult<()> {
    let blueband_canister = get_blueband_canister().map_err(|e| format!("{:?}", e))?;
    let service = BluebandService::new(blueband_canister);
    service.delete_collection(collection_id.to_string()).await
}

// ============================
// SEARCH OPERATIONS
// ============================

pub async fn search_documents_in_blueband(
    collection_id: &str,
    query: &str,
    limit: Option<u32>,
) -> BluebandResult<Vec<SearchResult>> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;
    
    let service = BluebandService::new(blueband_canister);
    
    let request = SearchRequest {
        collection_id: collection_id.to_string(),
        query: query.to_string(),
        limit,
        filter: None,
        min_score: None,
    };
    
    // Convert Blueband MemorySearchResult to Clanopedia SearchResult
    match service.search(request).await {
        Ok(results) => {
            let converted_results = results
                .into_iter()
                .map(|result| SearchResult {
                    document_id: result.document_id,
                    title: "".to_string(), // Blueband MemorySearchResult doesn't include title
                    content: result.text,
                    score: result.score,
                })
                .collect();
            Ok(converted_results)
        },
        Err(e) => Err(e),
    }
}

// ============================
// ADMIN OPERATIONS
// ============================

pub async fn transfer_genesis_admin(
    collection_id: &str,
    new_genesis: Principal,
) -> BluebandResult<()> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;
    
    let service = BluebandService::new(blueband_canister);
    service.transfer_genesis_admin(
        collection_id.to_string(),
        new_genesis.to_string(),
    ).await
}

// ============================
// CYCLES OPERATIONS
// ============================

pub async fn get_blueband_cycles_balance() -> u64 {
    match get_blueband_canister() {
        Ok(blueband_canister) => {
            let service = BluebandService::new(blueband_canister);
            service.get_canister_cycles().await
        },
        Err(_) => 0,
    }
}

pub async fn fund_blueband_cycles(_cycles_amount: u64) -> BluebandResult<u64> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;
    
    let service = BluebandService::new(blueband_canister);
    Ok(service.wallet_receive().await)
}