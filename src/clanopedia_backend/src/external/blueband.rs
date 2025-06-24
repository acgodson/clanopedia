// src/clanopedia_backend/src/external/blueband.rs
use crate::types::*;
use candid::{CandidType, Deserialize, Principal};
use ic_cdk::call;
use serde::Serialize;
use std::result::Result;

// ============================
// BLUEBAND TYPES
// ============================

// Generic result type for Blueband operations
pub type BluebandResult<T> = Result<T, String>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct AddDocumentRequest {
    pub collection_id: CollectionId,
    pub title: String,
    pub content: String,
    pub content_type: Option<ContentType>,
    pub source_url: Option<String>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum ContentType {
    Pdf,
    Html,
    PlainText,
    Markdown,
    Other(String),
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct BulkEmbedResult {
    pub skipped: u32,
    pub errors: Vec<String>,
    pub embedded: u32,
    pub failed: u32,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Collection {
    pub id: String,
    pub updated_at: u64,
    pub genesis_admin: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: u64,
    pub settings: CollectionSettings,
    pub admins: Vec<String>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CollectionSettings {
    pub chunk_overlap: u32,
    pub max_documents: Option<u32>,
    pub embedding_model: String,
    pub auto_embed: bool,
    pub proxy_url: String,
    pub chunk_size: u32,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CollectionStats {
    pub updated_at: u64,
    pub document_count: u32,
    pub created_at: u64,
    pub vector_count: u32,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CollectionWithStats {
    pub collection: Collection,
    pub stats: CollectionStats,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CreateCollectionRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub settings: Option<CollectionSettings>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct DocumentMetadata {
    pub id: String,
    pub total_chunks: u32,
    pub title: String,
    pub size: u64,
    pub content_type: ContentType,
    pub collection_id: String,
    pub is_embedded: bool,
    pub source_url: Option<String>,
    pub timestamp: u64,
    pub checksum: String,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct SearchRequest {
    pub collection_id: String,
    pub query: String,
    pub limit: Option<u32>,
    pub filter: Option<String>,
    pub min_score: Option<f64>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct MemorySearchResult {
    pub document_id: String,
    pub text: String,
    pub chunk_id: String,
    pub score: f64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct VectorMatch {
    pub document_id: String,
    pub document_title: Option<String>,
    pub chunk_id: String,
    pub score: f64,
    pub chunk_text: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct SemanticChunk {
    pub id: String,
    pub document_id: String,
    pub text: String,
    pub token_count: Option<u32>,
    pub char_end: u64,
    pub char_start: u64,
    pub position: u32,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CollectionMetrics {
    pub document_count: u64,
    pub search_count: u64,
}

// ============================
// BLUEBAND SERVICE
// ============================

pub struct BluebandService {
    canister_id: Principal,
}

impl BluebandService {
    pub fn new(canister_id: Principal) -> Self {
        Self { canister_id }
    }

    // Collection operations
    pub async fn create_collection(
        &self,
        request: CreateCollectionRequest,
    ) -> BluebandResult<Collection> {
        let result: Result<(BluebandResult<Collection>,), _> =
            call(self.canister_id, "create_collection", (request,)).await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    pub async fn get_collection(
        &self,
        collection_id: String,
    ) -> BluebandResult<Option<Collection>> {
        let result: Result<(BluebandResult<Option<Collection>>,), _> =
            call(self.canister_id, "get_collection", (collection_id,)).await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    // Document operations
    pub async fn add_document(
        &self,
        request: AddDocumentRequest,
    ) -> BluebandResult<DocumentMetadata> {
        let result: Result<(BluebandResult<DocumentMetadata>,), _> =
            call(self.canister_id, "add_document", (request,)).await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    pub async fn get_document(
        &self,
        collection_id: String,
        document_id: String,
    ) -> BluebandResult<Option<DocumentMetadata>> {
        let result: Result<(BluebandResult<Option<DocumentMetadata>>,), _> = call(
            self.canister_id,
            "get_document",
            (collection_id, document_id),
        )
        .await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    pub async fn get_document_content(
        &self,
        collection_id: String,
        document_id: String,
    ) -> BluebandResult<Option<String>> {
        let result: Result<(BluebandResult<Option<String>>,), _> = call(
            self.canister_id,
            "get_document_content",
            (collection_id, document_id),
        )
        .await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    pub async fn embed_existing_document(
        &self,
        collection_id: String,
        document_id: String,
    ) -> BluebandResult<u32> {
        let result: Result<(BluebandResult<u32>,), _> = call(
            self.canister_id,
            "embed_existing_document",
            (collection_id, document_id),
        )
        .await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    // Search operations
    pub async fn search(&self, request: SearchRequest) -> BluebandResult<Vec<VectorMatch>> {
        let result: Result<(BluebandResult<Vec<VectorMatch>>,), _> =
            call(self.canister_id, "search", (request,)).await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    pub async fn find_similar_documents(
        &self,
        document_id: String,
        collection_id: String,
        limit: Option<u32>,
        min_score: Option<f64>,
    ) -> BluebandResult<Vec<VectorMatch>> {
        let result: Result<(BluebandResult<Vec<VectorMatch>>,), _> = call(
            self.canister_id,
            "find_similar_documents",
            (document_id, collection_id, limit, min_score),
        )
        .await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    // Admin operations
    pub async fn add_collection_admin(
        &self,
        collection_id: String,
        admin: String,
    ) -> BluebandResult<()> {
        let result: Result<(BluebandResult<()>,), _> = call(
            self.canister_id,
            "add_collection_admin",
            (collection_id, admin),
        )
        .await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    pub async fn remove_collection_admin(
        &self,
        collection_id: String,
        admin: String,
    ) -> BluebandResult<()> {
        let result: Result<(BluebandResult<()>,), _> = call(
            self.canister_id,
            "remove_collection_admin",
            (collection_id, admin),
        )
        .await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    pub async fn transfer_genesis_admin(
        &self,
        collection_id: String,
        new_admin: String,
    ) -> BluebandResult<()> {
        let result: Result<(BluebandResult<()>,), _> = call(
            self.canister_id,
            "transfer_genesis_admin",
            (collection_id, new_admin),
        )
        .await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    pub async fn delete_document(
        &self,
        collection_id: String,
        document_id: String,
    ) -> BluebandResult<()> {
        let result: Result<(BluebandResult<()>,), _> = call(
            self.canister_id,
            "delete_document",
            (collection_id, document_id),
        )
        .await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    pub async fn delete_collection(&self, collection_id: String) -> BluebandResult<()> {
        let result: Result<(BluebandResult<()>,), _> =
            call(self.canister_id, "delete_collection", (collection_id,)).await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    // Cycles and stats
    pub async fn get_canister_cycles(&self) -> u64 {
        let result: Result<(u64,), _> = call(self.canister_id, "get_canister_cycles", ()).await;

        match result {
            Ok((cycles,)) => cycles,
            Err(_) => 0,
        }
    }

    pub async fn wallet_receive(&self) -> u64 {
        let result: Result<(u64,), _> = call(self.canister_id, "wallet_receive", ()).await;

        match result {
            Ok((received,)) => received,
            Err(_) => 0,
        }
    }

    // Bulk operations
    pub async fn bulk_embed_collection(
        &self,
        collection_id: String,
    ) -> BluebandResult<BulkEmbedResult> {
        let result: Result<(BluebandResult<BulkEmbedResult>,), _> =
            call(self.canister_id, "bulk_embed_collection", (collection_id,)).await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }

    pub async fn get_collection_metrics(
        &self,
        collection_id: String,
    ) -> BluebandResult<CollectionMetrics> {
        let result: Result<(BluebandResult<CollectionMetrics>,), _> =
            call(self.canister_id, "get_collection_metrics", (collection_id,)).await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(format!("Call failed: {}", e)),
        }
    }
}

// ============================
// BLUEBAND CLIENT FUNCTIONS
// ============================

// Get Blueband canister ID from global state
fn get_blueband_canister() -> ClanopediaResult<Principal> {
    super::super::get_blueband_canister_id()
}

pub async fn create_blueband_collection(
    collection_id: String,
    name: String,
    description: String,
) -> BluebandResult<Collection> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;

    let service = BluebandService::new(blueband_canister);

    let request = CreateCollectionRequest {
        id: collection_id,
        name,
        description: Some(description),
        settings: Some(CollectionSettings {
            chunk_overlap: 25,
            max_documents: None,
            embedding_model: "text-embedding-3-small".to_string(),
            auto_embed: true,
            proxy_url: "https://us-central1-blueband-db-442d8.cloudfunctions.net/proxy".to_string(),
            chunk_size: 300,
        }),
    };

    service.create_collection(request).await
}

pub async fn add_document_to_blueband(
    collection_id: &str,
    document: DocumentRequest,
) -> BluebandResult<DocumentMetadata> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;

    let service = BluebandService::new(blueband_canister);

    let request = AddDocumentRequest {
        title: document.title,
        content: document.content,
        content_type: document.content_type.or(Some(ContentType::PlainText)), // Default to plain text if not specified
        collection_id: collection_id.to_string(),
        source_url: document.source_url,
        author: document.author,
        tags: document.tags,
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
    service
        .embed_existing_document(collection_id.to_string(), document_id.to_string())
        .await
}

pub async fn delete_document(collection_id: &str, document_id: &str) -> BluebandResult<()> {
    let blueband_canister = get_blueband_canister().map_err(|e| format!("{:?}", e))?;
    let service = BluebandService::new(blueband_canister);
    service
        .delete_document(collection_id.to_string(), document_id.to_string())
        .await
}

pub async fn delete_collection(collection_id: &str) -> BluebandResult<()> {
    let blueband_canister = get_blueband_canister().map_err(|e| format!("{:?}", e))?;
    let service = BluebandService::new(blueband_canister);
    service.delete_collection(collection_id.to_string()).await
}

pub async fn get_blueband_cycles_balance() -> u64 {
    match get_blueband_canister() {
        Ok(blueband_canister) => {
            let service = BluebandService::new(blueband_canister);
            service.get_canister_cycles().await
        }
        Err(_) => 0,
    }
}

pub async fn fund_blueband_cycles(_cycles_amount: u64) -> BluebandResult<u64> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;

    let service = BluebandService::new(blueband_canister);
    Ok(service.wallet_receive().await)
}

// Implement get_document_content_from_blueband and get_document_metadata for compatibility
pub async fn get_document_content_from_blueband(
    collection_id: &str,
    document_id: &str,
) -> BluebandResult<Option<String>> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;
    let service = BluebandService::new(blueband_canister);
    service
        .get_document_content(collection_id.to_string(), document_id.to_string())
        .await
}

pub async fn get_document_metadata(
    collection_id: String,
    document_id: String,
) -> BluebandResult<Option<DocumentMetadata>> {
    let blueband_canister = get_blueband_canister().map_err(|e| format!("{:?}", e))?;
    let service = BluebandService::new(blueband_canister);
    service.get_document(collection_id, document_id).await
}

pub async fn transfer_genesis_admin(
    collection_id: &str,
    new_admin: candid::Principal,
) -> BluebandResult<()> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;
    let service = BluebandService::new(blueband_canister);
    service
        .transfer_genesis_admin(collection_id.to_string(), new_admin.to_string())
        .await
}

pub async fn get_collection_metrics(collection_id: &str) -> BluebandResult<CollectionMetrics> {
    let blueband_canister = get_blueband_canister()
        .map_err(|e| format!("Blueband canister not configured: {:?}", e))?;
    let service = BluebandService::new(blueband_canister);
    service.get_collection_metrics(collection_id.to_string()).await
}
