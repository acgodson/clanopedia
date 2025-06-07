// src/clanopedia_backend/src/cycles.rs - Fixed error formatting

use crate::types::*;
use candid::{CandidType, Principal};
use ic_cdk::api::call;
use ic_cdk::api::time;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

// ============================
// SIMPLE CONSTANTS
// ============================

// Clanopedia operation costs (fixed, low since we store minimal data)
const GOVERNANCE_OPERATION_COST: u64 = 1_000_000;      // 1M cycles
const MIN_CLANOPEDIA_BALANCE: u64 = 10_000_000;     // 10M cycles

// Blueband operation estimates (based on Blueband's own calculations)
const EMBEDDING_COST_PER_DOC: u64 = 10_000_000;    // 10M cycles per document
const SEARCH_COST: u64 = 1_000_000;                 // 1M cycles per search
const MIN_BLUEBAND_BALANCE: u64 = 50_000_000;    // 50M cycles (Blueband's minimum)

// Stable storage for Blueband canister ID
thread_local! {
    static BLUEBAND_CANISTER_ID: std::cell::RefCell<Option<Principal>> = std::cell::RefCell::new(None);
}

pub fn set_blueband_canister_id(canister_id: Principal) {
    BLUEBAND_CANISTER_ID.with(|id| {
        *id.borrow_mut() = Some(canister_id);
    });
}

pub fn get_blueband_canister_id() -> ClanopediaResult<Principal> {
    BLUEBAND_CANISTER_ID.with(|id| {
        id.borrow()
            .ok_or_else(|| ClanopediaError::InvalidInput("Blueband canister not initialized".to_string()))
    })
}

// ============================
// SIMPLE STATUS CHECK
// ============================

#[derive(CandidType, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CyclesStatus {
    pub clanopedia_balance: u64,
    pub blueband_balance: u64,
    pub clanopedia_healthy: bool,
    pub blueband_healthy: bool,
}

pub async fn check_cycles_status() -> ClanopediaResult<CyclesStatus> {
    let clanopedia_balance = ic_cdk::api::canister_balance();
    let blueband_balance = get_blueband_balance().await?;

    let clanopedia_healthy = clanopedia_balance >= MIN_CLANOPEDIA_BALANCE;
    let blueband_healthy = blueband_balance >= MIN_BLUEBAND_BALANCE;

    Ok(CyclesStatus {
        clanopedia_balance,
        blueband_balance,
        clanopedia_healthy,
        blueband_healthy,
    })
}

// ============================
// OPERATION VALIDATION
// ============================

pub fn validate_clanopedia_operation() -> ClanopediaResult<()> {
    let balance = ic_cdk::api::canister_balance();
    if balance < MIN_CLANOPEDIA_BALANCE {
        return Err(ClanopediaError::InsufficientCycles("Insufficient cycles for operation".to_string()));
    }
    Ok(())
}

pub async fn can_execute_embed_proposal(
    _proposal: &Proposal,
    documents: Vec<String>,
) -> ClanopediaResult<(bool, String)> {
    let cycles_status = check_cycles_status().await?;
    let cost = estimate_embedding_cost(documents).await?;

    if !cycles_status.clanopedia_healthy || !cycles_status.blueband_healthy {
        let message = format!(
            "Insufficient cycles. Clanopedia balance: {}, Blueband balance: {}, Required: {}",
            cycles_status.clanopedia_balance,
            cycles_status.blueband_balance,
            cost.total_cost
        );
        return Ok((false, message));
    }

    if cycles_status.clanopedia_balance < cost.total_cost {
        let message = format!(
            "Insufficient cycles for operation. Balance: {}, Required: {}",
            cycles_status.clanopedia_balance, cost.total_cost
        );
        return Ok((false, message));
    }

    Ok((true, "Sufficient cycles available".to_string()))
}

// ============================
// FUNDING OPERATIONS
// ============================

pub async fn fund_blueband_canister(amount: u64) -> ClanopediaResult<()> {
    validate_clanopedia_operation()?;
    let balance = ic_cdk::api::canister_balance();
    if balance < amount {
        return Err(ClanopediaError::InsufficientCycles("Insufficient cycles for transfer".to_string()));
    }
    let blueband_canister = get_blueband_canister_id()?;
    ic_cdk::api::call::call_with_payment::<_, ()>(
        blueband_canister,
        "deposit_cycles",
        (),
        amount,
    )
    .await
    .map_err(|e| ClanopediaError::BluebandError(format!("Failed to transfer cycles: {:?}", e)))?;
    Ok(())
}

// ============================
// REPORTING
// ============================

pub async fn get_cycles_health_report() -> ClanopediaResult<String> {
    let status = check_cycles_status().await?;
    
    let report = format!(
        "Clanopedia Cycles: {} ({})\nBlueband Cycles: {} ({})",
        format_cycles(status.clanopedia_balance),
        if status.clanopedia_healthy { "✅" } else { "⚠️" },
        format_cycles(status.blueband_balance),
        if status.blueband_healthy { "✅" } else { "⚠️" }
    );
    
    Ok(report)
}

pub async fn get_funding_recommendation(planned_docs: u32) -> ClanopediaResult<String> {
    let status = check_cycles_status().await?;
    let cost = estimate_embedding_cost(vec![]).await?;
    
    let recommendation = format!(
        "Current Status:\n\
         - Clanopedia: {} cycles ({})\n\
         - Blueband: {} cycles ({})\n\
         Required for {} documents: {} cycles\n\
         Recommendation: {}",
        format_cycles(status.clanopedia_balance),
        if status.clanopedia_healthy { "Healthy" } else { "Low" },
        format_cycles(status.blueband_balance),
        if status.blueband_healthy { "Healthy" } else { "Low" },
        planned_docs,
        format_cycles(cost.total_cost),
        if status.clanopedia_healthy && status.blueband_healthy {
            "No additional funding needed"
        } else {
            "Consider adding more cycles"
        }
    );
    
    Ok(recommendation)
}

// ============================
// PRE-EXECUTION CHECKS
// ============================

pub async fn pre_execution_cycles_check(
    proposal_type: &ProposalType,
    _blueband_canister: Principal,
) -> ClanopediaResult<()> {
    // Check Clanopedia can handle execution
    validate_clanopedia_operation()?;
    
    // Check Blueband has sufficient cycles for the specific operation
    match proposal_type {
        ProposalType::EmbedDocument { documents } => {
            let (can_execute, message) = can_execute_embed_proposal(
                &Proposal {
                    id: "temp".to_string(),
                    collection_id: "temp".to_string(),
                    proposal_type: proposal_type.clone(),
                    creator: Principal::anonymous(),
                    description: "".to_string(),
                    created_at: time(),
                    expires_at: time() + 7 * 24 * 60 * 60 * 1_000_000_000,
                    status: ProposalStatus::Active,
                    votes: HashMap::new(),
                    token_votes: HashMap::new(),
                    executed: false,
                    executed_at: None,
                    executed_by: None,
                    threshold: 0,
                    threshold_met: false,
                },
                documents.clone()
            ).await?;
            if !can_execute {
                return Err(ClanopediaError::InsufficientCycles(message));
            }
        },
        ProposalType::BatchEmbed { document_ids } => {
            let (can_execute, message) = can_execute_embed_proposal(
                &Proposal {
                    id: "temp".to_string(),
                    collection_id: "temp".to_string(),
                    proposal_type: proposal_type.clone(),
                    creator: Principal::anonymous(),
                    description: "".to_string(),
                    created_at: time(),
                    expires_at: time() + 7 * 24 * 60 * 60 * 1_000_000_000,
                    status: ProposalStatus::Active,
                    votes: HashMap::new(),
                    token_votes: HashMap::new(),
                    executed: false,
                    executed_at: None,
                    executed_by: None,
                    threshold: 0,
                    threshold_met: false,
                },
                document_ids.iter().map(|id| id.to_string()).collect()
            ).await?;
            if !can_execute {
                return Err(ClanopediaError::InsufficientCycles(message));
            }
        },
        // Other proposal types don't require Blueband cycles
        _ => {}
    }
    
    Ok(())
}

// ============================
// UTILITIES
// ============================

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

pub async fn get_blueband_balance() -> ClanopediaResult<u64> {
    let blueband_canister = get_blueband_canister_id()?;
    match call::call::<_, (u64,)>(blueband_canister, "get_cycles_balance", ()).await {
        Ok((balance,)) => Ok(balance),
        Err(e) => Err(ClanopediaError::ExternalCallError(format!(
            "Failed to get Blueband balance: {:?}",
            e
        ))),
    }
}

pub async fn estimate_embedding_cost(documents: Vec<String>) -> ClanopediaResult<CostMetrics> {
    let num_docs = documents.len() as u64;
    let base_cost = EMBEDDING_COST_PER_DOC * num_docs;
    let buffer_amount = (base_cost as f64 * 0.1) as u64; // 10% buffer
    let total_cost = base_cost + buffer_amount;

    Ok(CostMetrics {
        base_cost,
        total_cost,
        per_doc_cost: EMBEDDING_COST_PER_DOC,
        buffer_amount,
    })
}

fn estimate_search_cost(query_count: u32) -> u64 {
    SEARCH_COST * query_count as u64
}

pub async fn estimate_query_cost(operation_type: &str, doc_count: u32) -> ClanopediaResult<u64> {
    match operation_type {
        "embed" => Ok(estimate_embedding_cost(vec![]).await?.total_cost),
        "search" => Ok(estimate_search_cost(doc_count)),
        _ => Err(ClanopediaError::InvalidArgument(format!("Unknown operation type: {}", operation_type)))
    }
}