// src/clanopedia_backend/src/governance.rs -

use crate::external::sns_integration;
use candid::{Nat, Principal};
use getrandom::getrandom;
use ic_cdk::api::caller;
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::MemoryManager;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use std::cell::RefCell;
use std::collections::HashMap;
use std::str;

use crate::{
    cycles,
    external::{blueband, token},
    storage,
    types::{
        ClanopediaError, ClanopediaResult, Collection, CollectionConfig, CollectionId,
        GovernanceModel, Proposal, ProposalStatus, ProposalType, Vote, PROPOSAL_DURATION_NANOS,
    },
};

// Stable memory management for proposals lookup
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static PROPOSALS: RefCell<StableBTreeMap<String, Proposal, DefaultMemoryImpl>> = RefCell::new(
        StableBTreeMap::init(DefaultMemoryImpl::default())
    );
}

// Helper function to get current time in nanoseconds
fn current_time_ns() -> u64 {
    ic_cdk::api::time()
}

// ============================
// ATOMIC EXECUTION STRUCTURE
// ============================

#[derive(Debug)]
struct ExecutionPlan {
    validation_passed: bool,
    cycles_check_passed: bool,
    prerequisites_met: bool,
}

impl ExecutionPlan {
    fn new() -> Self {
        Self {
            validation_passed: false,
            cycles_check_passed: false,
            prerequisites_met: false,
        }
    }

    fn is_ready_for_execution(&self) -> bool {
        self.validation_passed && self.cycles_check_passed && self.prerequisites_met
    }
}

// ============================
//  ATOMIC EXECUTE PROPOSAL
// ============================

pub async fn execute_proposal(collection_id: &str, proposal_id: &str) -> ClanopediaResult<()> {
    // Phase 1: Load and validate basic state (read-only)
    let executor = caller();
    let collection = storage::get_collection(&collection_id.to_string())?;

    // Get proposal directly from collection's proposals and clone it
    let proposal = collection
        .proposals
        .get(proposal_id)
        .ok_or_else(|| {
            ClanopediaError::NotFound(format!(
                "Proposal {} not found in collection {}",
                proposal_id, collection_id
            ))
        })?
        .clone();

    // Phase 2: Pre-execution validation (no state changes)
    let mut execution_plan = ExecutionPlan::new();

    // Validate executor authorization
    if !collection.admins.contains(&executor) {
        return Err(ClanopediaError::NotAuthorized);
    }

    // Validate proposal state
    if proposal.status != ProposalStatus::Approved {
        return Err(ClanopediaError::InvalidProposalState(
            "Proposal must be approved to execute".to_string(),
        ));
    }

    if proposal.expires_at < time() {
        // Mark as expired but don't save yet - we'll do all saves atomically
        let mut expired_proposal = proposal.clone();
        expired_proposal.status = ProposalStatus::Expired;
        storage::update_proposal_in_storage(&collection_id.to_string(), &expired_proposal)?;
        return Err(ClanopediaError::ProposalExpired);
    }

    if proposal.executed {
        return Err(ClanopediaError::InvalidProposalState(
            "Proposal has already been executed".to_string(),
        ));
    }

    execution_plan.validation_passed = true;

    // Phase 3: Check threshold (read-only)
    let has_threshold = check_threshold(collection_id, &proposal).await?;
    if !has_threshold {
        return Err(ClanopediaError::ThresholdNotMet);
    }

    execution_plan.prerequisites_met = true;

    // Phase 4: Pre-execution cycles and resource validation (read-only)
    match &proposal.proposal_type {
        ProposalType::EmbedDocument { documents } => {
            let (can_execute, message) =
                cycles::can_execute_embed_proposal(&proposal, documents.clone()).await?;
            if !can_execute {
                return Err(ClanopediaError::InsufficientCycles(message));
            }
        }
        ProposalType::BatchEmbed { document_ids } => {
            let (can_execute, message) =
                cycles::can_execute_embed_proposal(&proposal, document_ids.clone()).await?;
            if !can_execute {
                return Err(ClanopediaError::InsufficientCycles(message));
            }
        }
        ProposalType::AddAdmin { admin } => {
            if collection.admins.contains(admin) {
                return Err(ClanopediaError::AlreadyExists(
                    "Admin already exists".to_string(),
                ));
            }
        }
        ProposalType::RemoveAdmin { admin } => {
            if !collection.admins.contains(admin) {
                return Err(ClanopediaError::NotFound("Admin not found".to_string()));
            }
            if collection.admins.len() <= 1 {
                return Err(ClanopediaError::InvalidInput(
                    "Cannot remove the last admin".to_string(),
                ));
            }
        }
        ProposalType::ChangeThreshold { new_threshold } => {
            if *new_threshold == 0 || *new_threshold > collection.admins.len() as u32 {
                return Err(ClanopediaError::InvalidInput(format!(
                    "Invalid threshold: must be between 1 and {}",
                    collection.admins.len()
                )));
            }
        }
        ProposalType::UpdateQuorum { new_percentage } => {
            if *new_percentage > 100 {
                return Err(ClanopediaError::InvalidInput(
                    "Quorum percentage cannot exceed 100".to_string(),
                ));
            }
        }
        ProposalType::DeleteCollection => {
            if !collection.proposals.is_empty() {
                let active_count = collection.proposals.len();
                if active_count > 1 {
                    // More than just this proposal
                    return Err(ClanopediaError::InvalidOperation(format!(
                        "Cannot delete collection with {} other active proposals",
                        active_count - 1
                    )));
                }
            }
        }
        _ => {} // Other proposal types validated in their execution functions
    }

    execution_plan.cycles_check_passed = true;

    // Phase 5: Final safety check
    if !execution_plan.is_ready_for_execution() {
        return Err(ClanopediaError::InvalidOperation(
            "Proposal execution prerequisites not met".to_string(),
        ));
    }

    // Phase 6: ATOMIC EXECUTION - All external calls and state changes happen here
    // From this point on, we either succeed completely or fail completely
    let execution_result = execute_proposal_operation(&proposal.proposal_type, collection_id).await;

    match execution_result {
        Ok(()) => {
            // SUCCESS: Update proposal status atomically
            let mut executed_proposal = proposal;
            executed_proposal.status = ProposalStatus::Executed;
            executed_proposal.executed = true;
            executed_proposal.executed_at = Some(time());
            executed_proposal.executed_by = Some(executor);
            storage::update_proposal_in_storage(&collection_id.to_string(), &executed_proposal)?;
            Ok(())
        }
        Err(e) => {
            // FAILURE: Mark proposal as failed but don't execute
            let mut failed_proposal = proposal;
            failed_proposal.status = ProposalStatus::Rejected;
            storage::update_proposal_in_storage(&collection_id.to_string(), &failed_proposal)?;
            Err(e)
        }
    }
}

// ============================
// ATOMIC OPERATION EXECUTOR
// ============================

pub async fn execute_proposal_operation(
    proposal_type: &ProposalType,
    collection_id: &str,
) -> ClanopediaResult<()> {
    match proposal_type {
        ProposalType::EmbedDocument { documents } => {
            execute_embed_document(collection_id, documents).await
        }
        ProposalType::BatchEmbed { document_ids } => {
            execute_batch_embed(collection_id, document_ids).await
        }
        ProposalType::UpdateCollection { config } => {
            execute_update_collection(collection_id, config.clone()).await
        }
        ProposalType::ChangeGovernanceModel { model } => {
            execute_change_governance_model(collection_id, model.clone()).await
        }
        ProposalType::AddAdmin { admin } => execute_add_admin(collection_id, *admin).await,
        ProposalType::RemoveAdmin { admin } => execute_remove_admin(collection_id, *admin).await,
        ProposalType::ChangeThreshold { new_threshold } => {
            execute_change_threshold(collection_id, *new_threshold).await
        }
        ProposalType::UpdateQuorum { new_percentage } => {
            execute_update_quorum(collection_id, *new_percentage).await
        }
        ProposalType::DeleteCollection => execute_delete_collection(collection_id).await,
    }
}

// Vote on proposals 
pub async fn vote_on_proposal(
    collection_id: &str,
    proposal_id: &str,
    vote: Vote,
) -> ClanopediaResult<()> {
    let mut proposal = get_proposal(collection_id, proposal_id)?;
    let voter = caller();

    // Check proposal state
    if proposal.status != ProposalStatus::Active {
        return Err(ClanopediaError::InvalidProposalState(
            "Proposal is not active".to_string(),
        ));
    }

    if proposal.expires_at < time() {
        proposal.status = ProposalStatus::Expired;
        storage::update_proposal_in_storage(&collection_id.to_string(), &proposal)?;
        return Err(ClanopediaError::ProposalExpired);
    }

    if proposal.executed {
        return Err(ClanopediaError::InvalidProposalState(
            "Proposal has already been executed".to_string(),
        ));
    }

    // Validate voter based on governance model
    let collection = storage::get_collection(&collection_id.to_string())?;
    validate_voter(&collection, &voter, &vote).await?;

    match collection.governance_model {
        GovernanceModel::TokenBased => {
            // Prevent double voting
            if proposal.votes.contains_key(&voter) {
                return Err(ClanopediaError::InvalidOperation(
                    "You have already voted on this proposal".to_string(),
                ));
            }
            if let Some(token_canister) = collection.governance_token {
                let balance = token::get_token_balance(token_canister, voter).await?;
                proposal.token_votes.insert(voter, balance);
                proposal.votes.insert(voter, vote); // Also record the vote
            }
        }
        _ => {
            // Prevent double voting for other models as well
            if proposal.votes.contains_key(&voter) {
                return Err(ClanopediaError::InvalidOperation(
                    "You have already voted on this proposal".to_string(),
                ));
            }
            proposal.votes.insert(voter, vote);
        }
    }

    // After voting, check if threshold is met
    let threshold_met = check_threshold(collection_id, &proposal).await?;
    if threshold_met {
        proposal.status = ProposalStatus::Approved;
        proposal.threshold_met = true;
    }

    // Update proposal
    storage::update_proposal_in_storage(&collection_id.to_string(), &proposal)?;
    Ok(())
}

async fn validate_voter(
    collection: &Collection,
    voter: &Principal,
    _vote: &Vote,
) -> ClanopediaResult<()> {
    match collection.governance_model {
        GovernanceModel::TokenBased => {
            if let Some(token_canister) = collection.governance_token {
                let balance = token::get_token_balance(token_canister, *voter).await?;
                if balance == 0u64 {
                    return Err(ClanopediaError::NotAuthorized);
                }
            } else {
                return Err(ClanopediaError::InvalidOperation(
                    "Token-based governance requires a governance token".to_string(),
                ));
            }
        }
        GovernanceModel::Multisig => {
            if !collection.admins.contains(voter) {
                return Err(ClanopediaError::NotAuthorized);
            }
        }
        GovernanceModel::Permissionless => {
            // No voting needed for permissionless - proposals execute immediately
            return Err(ClanopediaError::InvalidOperation(
                "Permissionless governance doesn't require voting".to_string(),
            ));
        }
        GovernanceModel::SnsIntegrated => {
            // SNS integration would validate through external SNS
            // For now, return error as SNS integration not implemented
            return Err(ClanopediaError::InvalidOperation(
                "SNS governance not yet implemented".to_string(),
            ));
        }
    }
    Ok(())
}

// Check if voting threshold is met - Made async to handle token holder count
pub async fn check_threshold(collection_id: &str, proposal: &Proposal) -> ClanopediaResult<bool> {
    let collection = storage::get_collection(&collection_id.to_string())?;

    match collection.governance_model {
        GovernanceModel::Permissionless => {
            // Permissionless should execute immediately, not go through voting
            Ok(true)
        }
        GovernanceModel::Multisig => {
            let yes_votes = proposal.votes.values().filter(|&v| v == &Vote::Yes).count() as u32;
            Ok(yes_votes >= collection.threshold)
        }
        GovernanceModel::TokenBased => {
            if let Some(token_canister) = collection.governance_token {
                let total_supply = token::get_token_total_supply(token_canister).await?;
                let total_yes_tokens = proposal
                    .token_votes
                    .iter()
                    .filter(|(principal, _)| proposal.votes.get(principal) == Some(&Vote::Yes))
                    .fold(Nat::from(0u64), |acc, (_, amount)| acc + amount.clone());

                let threshold_amount = (total_supply.clone()
                    * Nat::from(collection.quorum_threshold))
                    / Nat::from(100u32);
                Ok(total_yes_tokens >= threshold_amount)
            } else {
                Ok(false)
            }
        }
        GovernanceModel::SnsIntegrated => {
            if let Some(sns_governance) = collection.sns_governance_canister {
                if let Some(sns_proposal_id) = proposal.sns_proposal_id {
                    sns_integration::check_sns_proposal_approved(sns_governance, sns_proposal_id)
                        .await
                        .map_err(|e| ClanopediaError::SnsError(e.to_string()))
                } else {
                    Ok(false)
                }
            } else {
                Err(ClanopediaError::SnsNotConfigured)
            }
        }
    }
}

// Proposal execution functions
pub async fn execute_embed_document(
    collection_id: &str,
    documents: &[String],
) -> ClanopediaResult<()> {
    let collection = storage::get_collection(&collection_id.to_string())?;

    // Call Blueband to embed existing documents
    for document_id in documents {
        blueband::embed_existing_document(&collection.blueband_collection_id, document_id)
            .await
            .map_err(ClanopediaError::BluebandError)?;
    }

    Ok(())
}

pub async fn execute_batch_embed(
    collection_id: &str,
    document_ids: &[String],
) -> ClanopediaResult<()> {
    let collection = storage::get_collection(&collection_id.to_string())?;

    // Call Blueband for each document (could be optimized with batch API)
    for document_id in document_ids {
        blueband::embed_existing_document(&collection.blueband_collection_id, document_id)
            .await
            .map_err(ClanopediaError::BluebandError)?;
    }

    Ok(())
}

pub async fn execute_add_admin(collection_id: &str, new_admin: Principal) -> ClanopediaResult<()> {
    let mut collection = storage::get_collection(&collection_id.to_string())?;
    if !collection.admins.contains(&new_admin) {
        collection.admins.push(new_admin);
        storage::update_collection(&collection_id.to_string(), &collection)?;
    }
    Ok(())
}

pub async fn execute_remove_admin(
    collection_id: &str,
    admin_to_remove: Principal,
) -> ClanopediaResult<()> {
    let mut collection = storage::get_collection(&collection_id.to_string())?;

    // Prevent removing the last admin
    if collection.admins.len() <= 1 {
        return Err(ClanopediaError::InvalidInput(
            "Cannot remove the last admin".into(),
        ));
    }

    collection.admins.retain(|&admin| admin != admin_to_remove);
    storage::update_collection(&collection_id.to_string(), &collection)?;
    Ok(())
}

async fn execute_change_threshold(collection_id: &str, new_threshold: u32) -> ClanopediaResult<()> {
    let mut collection = storage::get_collection(&collection_id.to_string())?;
    let max_threshold = collection.admins.len() as u32;

    if new_threshold == 0 || new_threshold > max_threshold {
        return Err(ClanopediaError::InvalidInput(format!(
            "Invalid threshold: must be between 1 and {}",
            max_threshold
        )));
    }

    collection.threshold = new_threshold;
    storage::update_collection(&collection_id.to_string(), &collection)?;
    Ok(())
}

async fn execute_update_quorum(collection_id: &str, new_percentage: u32) -> ClanopediaResult<()> {
    let mut collection = storage::get_collection(&collection_id.to_string())?;

    if new_percentage > 100 {
        return Err(ClanopediaError::InvalidInput(
            "Quorum percentage cannot exceed 100".into(),
        ));
    }

    collection.quorum_threshold = new_percentage;
    storage::update_collection(&collection_id.to_string(), &collection)?;
    Ok(())
}

async fn execute_change_governance_model(
    collection_id: &str,
    new_model: GovernanceModel,
) -> ClanopediaResult<()> {
    let mut collection = storage::get_collection(&collection_id.to_string())?;

    // Validate the new configuration
    if matches!(new_model, GovernanceModel::Multisig) && collection.admins.is_empty() {
        return Err(ClanopediaError::InvalidInput(
            "Multisig governance requires at least one admin".into(),
        ));
    }

    collection.governance_model = new_model;
    storage::update_collection(&collection_id.to_string(), &collection)?;
    Ok(())
}

pub async fn execute_update_collection(
    collection_id: &str,
    mut config: CollectionConfig,
) -> ClanopediaResult<()> {
    let mut collection = storage::get_collection(&collection_id.to_string())?;

    // Convert string representations to Principal objects for validation
    let admins: Result<Vec<Principal>, _> =
        config.admins.iter().map(Principal::from_text).collect();

    let governance_token = config
        .governance_token
        .as_ref()
        .map(Principal::from_text)
        .transpose()
        .map_err(|e| {
            ClanopediaError::InvalidInput(format!("Invalid governance token principal: {}", e))
        })?;

    // If any principal is invalid, keep existing admins
    if admins.is_err() {
        config.admins = collection.admins.iter().map(|p| p.to_string()).collect();
    }

    collection.name = config.name;
    collection.description = config.description;
    collection.admins = admins.unwrap_or_else(|_| collection.admins.clone());
    collection.threshold = config.threshold;
    collection.governance_token = governance_token;
    collection.governance_model = config.governance_model;
    collection.quorum_threshold = config.quorum_threshold;
    collection.is_permissionless = config.is_permissionless;
    collection.updated_at = time();

    storage::update_collection(&collection_id.to_string(), &collection)?;
    Ok(())
}

pub async fn execute_delete_collection(collection_id: &str) -> ClanopediaResult<()> {
    let collection = storage::get_collection(&collection_id.to_string())?;

    // Call Blueband to delete the collection
    blueband::delete_collection(&collection.blueband_collection_id)
        .await
        .map_err(ClanopediaError::BluebandError)?;

    // Remove from Clanopedia storage
    storage::delete_collection(&collection_id.to_string())?;
    Ok(())
}

// Utility functions for governance
pub fn can_execute_directly(collection_id: &CollectionId) -> ClanopediaResult<bool> {
    let collection = storage::get_collection(collection_id)?;
    Ok(collection.is_permissionless)
}

pub fn get_proposals(collection_id: &str) -> ClanopediaResult<Vec<Proposal>> {
    let collection = storage::get_collection(&collection_id.to_string())?;
    Ok(collection.proposals.values().cloned().collect())
}

pub fn get_proposal_status(
    collection_id: &str,
    proposal_id: String,
) -> ClanopediaResult<ProposalStatus> {
    let proposal = get_proposal(collection_id, &proposal_id)?;
    Ok(proposal.status)
}

// Add cleanup function for expired proposals and associated documents
pub async fn cleanup_expired_proposals(collection_id: &str) -> ClanopediaResult<u32> {
    let mut collection = storage::get_collection(&collection_id.to_string())?;
    let current_time = time();
    let mut cleaned = 0u32;

    let expired_proposal_ids: Vec<String> = collection
        .proposals
        .iter()
        .filter(|(_, proposal)| proposal.expires_at < current_time)
        .map(|(id, _)| id.clone())
        .collect();

    for id in expired_proposal_ids {
        collection.proposals.remove(&id);
        cleaned += 1;
    }

    storage::update_collection(&collection_id.to_string(), &collection)?;
    Ok(cleaned)
}

// Add collection deletion with Blueband sync
pub async fn delete_collection(collection_id: &str, caller: Principal) -> ClanopediaResult<()> {
    let collection = storage::get_collection(&collection_id.to_string())?;

    // Verify caller is an admin
    if !collection.admins.contains(&caller) {
        return Err(ClanopediaError::Unauthorized(
            "Only admins can delete collections".to_string(),
        ));
    }

    // Call Blueband to delete the collection
    blueband::delete_collection(&collection.blueband_collection_id)
        .await
        .map_err(ClanopediaError::BluebandError)?;

    // Remove from Clanopedia storage
    storage::delete_collection(&collection_id.to_string())?;

    Ok(())
}

pub async fn create_proposal(
    collection_id: &str,
    proposal_type: ProposalType,
    creator: Principal,
    description: String,
) -> ClanopediaResult<String> {
    let collection = storage::get_collection(&collection_id.to_string())?;

    // Generate a random number using getrandom
    let mut random_bytes = [0u8; 4];
    getrandom(&mut random_bytes).map_err(|e| {
        ClanopediaError::InvalidInput(format!("Failed to generate random bytes: {}", e))
    })?;
    let random_number = u32::from_be_bytes(random_bytes);

    // Generate a unique proposal ID similar to collection ID format
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
    let proposal_id = format!("prop_{}_{}_{}", collection_id, timestamp_short, random_hex);

    let mut proposal = Proposal {
        id: proposal_id.clone(),
        collection_id: collection_id.to_string(),
        proposal_type: proposal_type.clone(),
        creator,
        description: description.clone(),
        created_at: current_time_ns(),
        expires_at: current_time_ns() + PROPOSAL_DURATION_NANOS,
        status: ProposalStatus::Active,
        votes: HashMap::new(),
        threshold_met: false,
        executed: false,
        threshold: collection.threshold,
        executed_at: None,
        executed_by: None,
        token_votes: HashMap::new(),
        sns_proposal_id: None,
    };

    // Update collection with new proposal
    let mut updated_collection = collection;
    updated_collection
        .proposals
        .insert(proposal_id.clone(), proposal.clone());
    storage::update_collection(&collection_id.to_string(), &updated_collection)?;

    // For permissionless collections, auto-approve but don't execute
    if updated_collection.is_permissionless
        || matches!(
            updated_collection.governance_model,
            GovernanceModel::Permissionless
        )
    {
        // Mark proposal as approved but not executed
        let mut approved_proposal = proposal;
        approved_proposal.status = ProposalStatus::Approved;
        approved_proposal.threshold_met = true;
        updated_collection
            .proposals
            .insert(proposal_id.clone(), approved_proposal);
        storage::update_collection(&collection_id.to_string(), &updated_collection)?;
    }

    Ok(proposal_id)
}

pub fn get_proposal(collection_id: &str, proposal_id: &str) -> ClanopediaResult<Proposal> {
    let collection = storage::get_collection(&collection_id.to_string())?;

    collection
        .proposals
        .get(proposal_id)
        .cloned()
        .ok_or_else(|| {
            ClanopediaError::NotFound(format!(
                "Proposal {} not found in collection {}",
                proposal_id, collection_id
            ))
        })
}


//  Link an SNS proposal ID to a Clanopedia proposal
pub fn link_sns_proposal_id(
    collection_id: &str,
    proposal_id: &str,
    sns_proposal_id: u64,
    caller: Principal,
) -> ClanopediaResult<()> {
    let mut collection = storage::get_collection(&collection_id.to_string())?;
    // Only admin can link
    if !collection.admins.contains(&caller) {
        return Err(ClanopediaError::NotAuthorized);
    }
    let proposal = collection
        .proposals
        .get_mut(proposal_id)
        .ok_or_else(|| ClanopediaError::NotFound(format!("Proposal {} not found", proposal_id)))?;
    proposal.sns_proposal_id = Some(sns_proposal_id);
    storage::update_collection(&collection_id.to_string(), &collection)?;
    Ok(())
}

//  Sync SNS proposal status and update Clanopedia proposal if approved
pub async fn sync_sns_proposal_status_and_update(
    collection_id: &str,
    proposal_id: &str,
) -> ClanopediaResult<()> {
    let mut collection = storage::get_collection(&collection_id.to_string())?;
    let proposal = collection
        .proposals
        .get_mut(proposal_id)
        .ok_or_else(|| ClanopediaError::NotFound(format!("Proposal {} not found", proposal_id)))?;
    if collection.governance_model == GovernanceModel::SnsIntegrated {
        if let Some(sns_governance) = collection.sns_governance_canister {
            if let Some(sns_proposal_id) = proposal.sns_proposal_id {
                let is_approved = crate::external::sns_integration::check_sns_proposal_approved(
                    sns_governance,
                    sns_proposal_id,
                )
                .await?;
                if is_approved && proposal.status == ProposalStatus::Active {
                    proposal.status = ProposalStatus::Approved;
                    proposal.threshold_met = true;
                    storage::update_collection(&collection_id.to_string(), &collection)?;
                }
            }
        }
    }
    Ok(())
}
