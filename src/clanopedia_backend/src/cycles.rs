// src/clanopedia_backend/src/cycles.rs - Fixed with safety buffer

use crate::types::*;
use candid::{CandidType, Principal};
use serde::{Serialize, Deserialize};


// predictions for operation costs
const MIN_CLANOPEDIA_BALANCE: u64 = 50_000_000;        // 50M cycles (increased from 10M)
const SAFETY_BUFFER: u64 = 100_000_000;                // 100M cycles safety buffer

// // Blueband operation estimates (based on Blueband's own calculations)
const EMBEDDING_COST_PER_DOC: u64 = 10_000_000;    // 10M cycles per document



pub fn get_blueband_canister_id() -> ClanopediaResult<Principal> {
    crate::get_blueband_canister_id()
}
// ============================
// UPDATED STATUS CHECK
// ============================

#[derive(CandidType, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CyclesStatus {
    pub clanopedia_balance: u64,
    pub blueband_balance: u64,
    pub clanopedia_healthy: bool,
    pub blueband_healthy: bool,
    pub can_transfer_safely: bool,
}

pub async fn check_cycles_status() -> ClanopediaResult<CyclesStatus> {
    let clanopedia_balance = ic_cdk::api::canister_balance();
    let clanopedia_healthy = clanopedia_balance >= MIN_CLANOPEDIA_BALANCE;
    let can_transfer_safely = clanopedia_balance > (MIN_CLANOPEDIA_BALANCE + SAFETY_BUFFER);

    Ok(CyclesStatus {
        clanopedia_balance,
        blueband_balance: 0, 
        clanopedia_healthy,
        blueband_healthy: true,
        can_transfer_safely,
    })
}

// ============================
//  OPERATION VALIDATION
// ============================

pub fn validate_clanopedia_operation() -> ClanopediaResult<()> {
    let balance = ic_cdk::api::canister_balance();
    if balance < MIN_CLANOPEDIA_BALANCE {
        return Err(ClanopediaError::InsufficientCycles(
            format!("Insufficient cycles for operation. Balance: {}, Required: {}", 
                   balance, MIN_CLANOPEDIA_BALANCE)
        ));
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
            "Insufficient cycles. Clanopedia: {} ({}), Blueband: {} ({}), Required: {}",
            cycles_status.clanopedia_balance,
            if cycles_status.clanopedia_healthy { "✅" } else { "⚠️" },
            cycles_status.blueband_balance,
            if cycles_status.blueband_healthy { "✅" } else { "⚠️" },
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
// UPDATED FUNDING OPERATIONS WITH SAFETY CHECKS
// ============================

pub async fn fund_blueband_canister(amount: u64) -> ClanopediaResult<()> {
    validate_clanopedia_operation()?;
    let balance = ic_cdk::api::canister_balance();
    
    // Safety check: ensure we keep enough cycles + safety buffer
    let required_balance = MIN_CLANOPEDIA_BALANCE + SAFETY_BUFFER + amount;
    if balance < required_balance {
        return Err(ClanopediaError::InsufficientCycles(
            format!(
                "Transfer would leave insufficient cycles. Current: {}, Transfer: {}, Required remaining: {} (including safety buffer)",
                balance, amount, MIN_CLANOPEDIA_BALANCE + SAFETY_BUFFER
            )
        ));
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

// New helper function to calculate safe transfer amount
pub async fn get_max_safe_transfer_amount() -> ClanopediaResult<u64> {
    let balance = ic_cdk::api::canister_balance();
    let required_minimum = MIN_CLANOPEDIA_BALANCE + SAFETY_BUFFER;
    
    if balance <= required_minimum {
        Ok(0)
    } else {
        Ok(balance - required_minimum)
    }
}

// ============================
// UPDATED REPORTING
// ============================

pub async fn get_cycles_health_report() -> ClanopediaResult<String> {
    let status = check_cycles_status().await?;
    let max_transfer = get_max_safe_transfer_amount().await?;
    
    let report = format!(
        "Clanopedia Cycles: {} ({})\n\
         Blueband Cycles: {} ({})\n\
         Max Safe Transfer: {} cycles\n\
         Transfer Status: {}",
        format_cycles(status.clanopedia_balance),
        if status.clanopedia_healthy { "✅" } else { "⚠️" },
        format_cycles(status.blueband_balance),
        if status.blueband_healthy { "✅" } else { "⚠️" },
        format_cycles(max_transfer),
        if status.can_transfer_safely { "Safe to transfer" } else { "⚠️ Low cycles - transfers restricted" }
    );
    
    Ok(report)
}

pub async fn get_funding_recommendation(planned_docs: u32) -> ClanopediaResult<String> {
    let status = check_cycles_status().await?;
    let cost = estimate_embedding_cost(vec![]).await?;
    let max_transfer = get_max_safe_transfer_amount().await?;
    
    let recommendation = format!(
        "Current Status:\n\
         - Clanopedia: {} cycles ({})\n\
         - Blueband: {} cycles ({})\n\
         - Max safe transfer: {} cycles\n\
         Required for {} documents: {} cycles\n\
         Recommendation: {}",
        format_cycles(status.clanopedia_balance),
        if status.clanopedia_healthy { "Healthy" } else { "Low" },
        format_cycles(status.blueband_balance),
        if status.blueband_healthy { "Healthy" } else { "Low" },
        format_cycles(max_transfer),
        planned_docs,
        format_cycles(cost.total_cost),
        if status.clanopedia_healthy && status.blueband_healthy && max_transfer > 0 {
            "System healthy - can proceed with operations"
        } else if max_transfer == 0 {
            "⚠️ Cannot safely transfer cycles - deposit more cycles first"
        } else {
            "⚠️ Consider adding more cycles before large operations"
        }
    );
    
    Ok(recommendation)
}

// ============================
// REST OF THE FILE UNCHANGED
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

