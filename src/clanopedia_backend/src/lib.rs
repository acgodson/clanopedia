// src/clanopedia_backend/src/lib.rs - Final fixes

use candid::Principal;
use ic_cdk::api::caller;
use ic_cdk::{query, update};
use ic_cdk_macros::*;
use ic_cdk::api::time;
use std::collections::HashMap;

mod blueband_client;
mod blueband_interface;
mod governance;
mod storage;
mod token_interface;
mod types;
mod cycles;
mod random;

// Re-export specific types and functions instead of using glob imports
pub use types::{
    Collection, Proposal, ProposalType, Vote, ClanopediaError, ClanopediaResult,
    CollectionConfig, DocumentRequest, SearchResult, BluebandDocument, BluebandConfig,
    GovernanceModelConfig, CollectionId, ProposalId, DocumentId, GovernanceModel,
    ProposalStatus, PROPOSAL_DURATION_NANOS,
};

pub use blueband_interface::{
    BluebandService, BluebandResult, Collection as BluebandCollection,
    DocumentMetadata, SearchRequest, MemorySearchResult, VectorMatch,
};

pub use cycles::{
    CyclesStatus, estimate_embedding_cost,
};

// Global state for Blueband canister ID
thread_local! {
    static BLUEBAND_CANISTER_ID: std::cell::RefCell<Option<Principal>> = 
        std::cell::RefCell::new(None);
}

fn set_blueband_canister_id(canister_id: Principal) {
    BLUEBAND_CANISTER_ID.with(|id| {
        *id.borrow_mut() = Some(canister_id);
    });
}

pub fn get_blueband_canister_id() -> ClanopediaResult<Principal> {
    BLUEBAND_CANISTER_ID.with(|id| {
        id.borrow()
            .clone()
            .ok_or_else(|| ClanopediaError::InvalidInput("Blueband canister not initialized".to_string()))
    })
}

// Constants
const MIN_CYCLES_BALANCE: u64 = 1_000_000_000; // 1B cycles minimum

// ============================
// INITIALIZATION
// ============================

#[init]
fn init() {
    // Initialize can be empty for now since we use thread_local storage
}

#[pre_upgrade]
fn pre_upgrade() {
    // Save state to stable storage if needed
}

#[post_upgrade]
fn post_upgrade() {
    // Load state from stable storage if needed
}

// ============================
// COLLECTION MANAGEMENT
// ============================

#[query]
fn get_collection(collection_id: String) -> ClanopediaResult<Collection> {
    storage::get_collection(&collection_id)
}

#[query]
fn list_collections() -> ClanopediaResult<Vec<Collection>> {
    Ok(storage::list_collections())
}

#[update]
async fn create_collection_endpoint(
    collection_id: CollectionId,
    config: CollectionConfig,
) -> ClanopediaResult<CollectionId> {
    let caller = ic_cdk::caller();
    storage::create_collection(&collection_id, config, caller)?;
    Ok(collection_id)
}

#[update]
async fn update_collection(
    collection_id: CollectionId,
    config: CollectionConfig,
) -> ClanopediaResult<()> {
    let caller = ic_cdk::caller();
    let collection = storage::get_collection(&collection_id)?;
    
    if !collection.admins.contains(&caller) {
        return Err(ClanopediaError::NotAuthorized);
    }

    let mut updated_collection = collection;
    updated_collection.name = config.name;
    updated_collection.description = config.description;
    updated_collection.admins = config.admins;
    updated_collection.threshold = config.threshold;
    updated_collection.governance_token = config.governance_token;
    updated_collection.governance_model = config.governance_model;
    updated_collection.members = config.members;
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
async fn add_document(
    collection_id: CollectionId,
    document: DocumentRequest,
) -> ClanopediaResult<String> {
    let caller = ic_cdk::caller();
    
    // Check if caller is admin
    if !is_admin(&collection_id, caller) {
        return Err(ClanopediaError::Unauthorized(
            "Only admins can add documents".to_string(),
        ));
    }
    
    // Add document to Blueband (without embedding)
    match blueband_client::add_document_to_blueband(&collection_id, document).await {
        Ok(metadata) => Ok(metadata.id),
        Err(e) => Err(ClanopediaError::BluebandError(e)),
    }
}

#[update]
async fn create_embed_proposal(
    collection_id: CollectionId,
    documents: Vec<String>,
) -> ClanopediaResult<ProposalId> {
    let caller = caller();
    validate_clanopedia_operation("create_proposal")?;
    let _collection = storage::get_collection(&collection_id)?;
    if !is_admin(&collection_id, caller) {
        return Err(ClanopediaError::NotAuthorized);
    }
    let proposal_type = ProposalType::EmbedDocument { documents };
    governance::create_proposal(&collection_id, proposal_type, caller, "Embed documents".to_string()).await
}

#[update]
async fn create_batch_embed_proposal(
    collection_id: CollectionId,
    document_ids: Vec<String>,
) -> ClanopediaResult<ProposalId> {
    let caller = caller();
    validate_clanopedia_operation("create_proposal")?;
    let _collection = storage::get_collection(&collection_id)?;
    if !is_admin(&collection_id, caller) {
        return Err(ClanopediaError::NotAuthorized);
    }
    let proposal_type = ProposalType::BatchEmbed { document_ids };
    governance::create_proposal(&collection_id, proposal_type, caller, "Batch embed documents".to_string()).await
}

// ============================
// GOVERNANCE OPERATIONS
// ============================

#[query]
fn get_proposal(proposal_id: String) -> ClanopediaResult<Proposal> {
    governance::get_proposal(&proposal_id)
}

#[query]
fn get_active_proposals_endpoint(collection_id: String) -> ClanopediaResult<Vec<Proposal>> {
    governance::get_active_proposals(&collection_id)
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
async fn execute_proposal_endpoint(collection_id: String, proposal_id: String) -> ClanopediaResult<()> {
    governance::execute_proposal(&collection_id, &proposal_id).await
}

#[query]
fn get_proposal_status_endpoint(collection_id: String, proposal_id: String) -> ClanopediaResult<ProposalStatus> {
    governance::get_proposal_status(&collection_id, proposal_id)
}

// ============================
// ADMIN OPERATIONS
// ============================

#[update]
async fn create_admin_proposal(collection_id: String, new_admin: Principal) -> ClanopediaResult<ProposalId> {
    let caller = caller();
    let proposal_type = ProposalType::AddAdmin { admin: new_admin };
    governance::create_proposal(&collection_id, proposal_type, caller, "Add new admin".to_string()).await
}

#[update]
async fn create_remove_admin_proposal(collection_id: String, admin_to_remove: Principal) -> ClanopediaResult<ProposalId> {
    let caller = caller();
    let proposal_type = ProposalType::RemoveAdmin { admin: admin_to_remove };
    governance::create_proposal(&collection_id, proposal_type, caller, "Remove admin".to_string()).await
}

#[update]
async fn create_threshold_proposal(collection_id: String, new_threshold: u32) -> ClanopediaResult<ProposalId> {
    let caller = caller();
    let proposal_type = ProposalType::ChangeThreshold { new_threshold };
    governance::create_proposal(&collection_id, proposal_type, caller, "Change threshold".to_string()).await
}

#[update]
async fn create_transfer_genesis_proposal(collection_id: String, new_genesis: Principal) -> ClanopediaResult<ProposalId> {
    let caller = caller();
    let proposal_type = ProposalType::TransferGenesis { new_genesis };
    governance::create_proposal(&collection_id, proposal_type, caller, "Transfer genesis ownership".to_string()).await
}

#[update]
async fn create_governance_model_proposal(collection_id: String, new_model: GovernanceModel) -> ClanopediaResult<ProposalId> {
    let caller = caller();
    let proposal_type = ProposalType::ChangeGovernanceModel { model: new_model };
    governance::create_proposal(&collection_id, proposal_type, caller, "Change governance model".to_string()).await
}

// ============================
// SEARCH OPERATIONS (Direct to Blueband)
// ============================

#[query]
async fn search_documents(
    collection_id: CollectionId,
    query: String,
    limit: Option<u32>,
) -> ClanopediaResult<Vec<SearchResult>> {
    let collection = storage::get_collection(&collection_id)?;
    blueband_client::search_documents_in_blueband(&collection.blueband_collection_id, &query, limit)
        .await
        .map_err(|e| ClanopediaError::BluebandError(e))
}

#[query]
async fn get_document(
    collection_id: CollectionId,
    document_id: DocumentId,
) -> ClanopediaResult<Option<String>> {
    let collection = storage::get_collection(&collection_id)?;
    blueband_client::get_document_content_from_blueband(&collection.blueband_collection_id, &document_id)
        .await
        .map_err(|e| ClanopediaError::BluebandError(e))
}

#[query]
async fn get_document_metadata(
    collection_id: CollectionId,
    document_id: DocumentId,
) -> ClanopediaResult<Option<crate::blueband_interface::DocumentMetadata>> {
    let collection = storage::get_collection(&collection_id)?;
    blueband_client::get_document_metadata(collection.blueband_collection_id, document_id)
        .await
        .map_err(|e| ClanopediaError::BluebandError(e))
}

// ============================
// CYCLES MANAGEMENT
// ============================

#[query]
async fn check_cycles_status_endpoint() -> ClanopediaResult<CyclesStatus> {
    cycles::check_cycles_status().await
}

#[query]
async fn get_cycles_health() -> ClanopediaResult<String> {
    cycles::get_cycles_health_report().await
}

#[query]
async fn get_funding_estimate(planned_docs: u32) -> ClanopediaResult<String> {
    cycles::get_funding_recommendation(planned_docs).await
}

#[update]
async fn transfer_cycles_to_blueband(amount: u64) -> ClanopediaResult<()> {
    cycles::fund_blueband_canister(amount).await
}

// ============================
// UTILITY FUNCTIONS
// ============================

#[update]
async fn cleanup_expired_proposals_endpoint(collection_id: String) -> ClanopediaResult<u32> {
    governance::cleanup_expired_proposals(&collection_id).await
}

#[query]
fn is_admin_check(collection_id: CollectionId, user: Principal) -> bool {
    is_admin(&collection_id, user)
}

#[query]
fn get_caller() -> Principal {
    ic_cdk::caller()
}

#[query]
fn get_clanopedia_canister_cycles() -> u64 {
    ic_cdk::api::canister_balance()
}

// ============================
// HEALTH CHECK
// ============================

#[query]
fn health_check() -> String {
    "Clanopedia Backend is running".to_string()
}

#[query]
async fn full_health_check() -> String {
    let clanopedia_cycles = ic_cdk::api::canister_balance();
    let blueband_status = match get_blueband_canister_id() {
        Ok(_blueband_canister) => {
            "(cycles check stubbed)".to_string()
        },
        Err(_) => "❌ Not configured".to_string(),
    };
    format!(
        "🟢 Clanopedia Status:\n\
         - Canister Cycles: {}\n\
         - Blueband Connection: {}\n\
         - Ready for operations",
        format_cycles(clanopedia_cycles),
        blueband_status
    )
}

// Helper function to format cycles
fn format_cycles(cycles: u64) -> String {
    if cycles >= 1_000_000_000_000 {
        format!("{:.1}T", cycles as f64 / 1_000_000_000_000.0)
    } else if cycles >= 1_000_000_000 {
        format!("{:.1}B", cycles as f64 / 1_000_000_000.0)
    } else if cycles >= 1_000_000 {
        format!("{:.1}M", cycles as f64 / 1_000_000.0)
    } else {
        format!("{}", cycles)
    }
}

// Add missing functions
fn validate_clanopedia_operation(_operation_type: &str) -> ClanopediaResult<()> {
    let cycles_balance = ic_cdk::api::canister_balance();
    if cycles_balance < MIN_CYCLES_BALANCE {
        return Err(ClanopediaError::InsufficientCycles("Insufficient cycles".to_string()));
    }
    Ok(())
}

async fn can_execute_embed_proposal(
    collection_id: &str,
    documents: Vec<String>,
) -> ClanopediaResult<(bool, String)> {
    let proposal = Proposal {
        id: "temp".to_string(),
        collection_id: collection_id.to_string(),
        proposal_type: ProposalType::EmbedDocument { documents: documents.clone() },
        creator: caller(),
        description: "".to_string(),
        created_at: time(),
        expires_at: time() + PROPOSAL_DURATION_NANOS,
        status: ProposalStatus::Active,
        votes: HashMap::new(),
        token_votes: HashMap::new(),
        executed: false,
        executed_at: None,
        executed_by: None,
        threshold: 0,
        threshold_met: false,
    };
    
    cycles::can_execute_embed_proposal(&proposal, documents).await
}

async fn pre_execution_cycles_check(
    collection_id: &str,
    proposal_type: &ProposalType,
) -> ClanopediaResult<()> {
    let documents = match proposal_type {
        ProposalType::EmbedDocument { documents } => documents.clone(),
        ProposalType::BatchEmbed { document_ids } => document_ids.iter().map(|id| id.to_string()).collect(),
        _ => vec![],
    };
    
    if !documents.is_empty() {
        let (can_execute, message) = can_execute_embed_proposal(collection_id, documents).await?;
        if !can_execute {
            return Err(ClanopediaError::InsufficientCycles(message));
        }
    }
    
    Ok(())
}

// Helper function to check if a user is an admin of a collection
fn is_admin(collection_id: &str, user: Principal) -> bool {
    match storage::get_collection(&collection_id.to_string()) {
        Ok(collection) => collection.admins.contains(&user),
        Err(_) => false,
    }
}

// Helper function to get current time in nanoseconds
fn current_time_ns() -> u64 {
    ic_cdk::api::time()
}

// Export candid interface
ic_cdk::export_candid!();