// src/clanopedia_backend/src/token_interface.rs - Fixed import conflicts

use candid::{CandidType, Principal, Nat};
use ic_cdk::api::call::call;
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc1::transfer::TransferArg;
use crate::types::{ClanopediaResult, ClanopediaError};

// ICRC-1 Token Interface
#[derive(CandidType, candid::Deserialize)]
pub struct TokenMetadata {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub fee: u64,
    pub total_supply: u128,
}

#[derive(CandidType, candid::Deserialize)]
pub struct TokenAccount {
    pub owner: Principal,
    pub subaccount: Option<Vec<u8>>,
}

#[derive(CandidType, candid::Deserialize)]
pub struct TokenTransferArgs {
    pub from_subaccount: Option<Vec<u8>>,
    pub to: TokenAccount,
    pub amount: u128,
    pub fee: Option<u128>,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
}

// Get token balance for a principal
pub async fn get_token_balance(token_canister: Option<Principal>, owner: Principal) -> ClanopediaResult<Nat> {
    if let Some(canister) = token_canister {
        let account = Account {
            owner,
            subaccount: None,
        };
        
        match call::<_, (Nat,)>(canister, "icrc1_balance_of", (account,)).await {
            Ok((balance,)) => Ok(balance),
            Err(e) => Err(ClanopediaError::BluebandError(format!("Failed to get token balance: {:?}", e))),
        }
    } else {
        Ok(Nat::from(0u64))
    }
}

// Get total number of token holders (proxy: total supply)
pub async fn get_token_holders_count(token_canister: Option<Principal>) -> Nat {
    if let Some(canister) = token_canister {
        // Call total_supply on the token canister
        match call(canister, "icrc1_total_supply", ()).await {
            Ok((supply,)) => supply,
            Err(_) => Nat::from(0u64),
        }
    } else {
        Nat::from(0u64)
    }
}

// Transfer tokens
pub async fn transfer_tokens(
    _from: Principal,
    to: Principal,
    amount: Nat,
    token_canister: Principal,
) -> Result<Nat, String> {
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to: Account {
            owner: to,
            subaccount: None,
        },
        fee: None,
        memo: None,
        created_at_time: None,
        amount,
    };

    match call(token_canister, "icrc1_transfer", (transfer_arg,)).await {
        Ok((block_index,)) => Ok(block_index),
        Err(e) => Err(format!("Transfer failed: {:?}", e)),
    }
}

// Helper: Get total supply from an ICRC-1 token canister
pub async fn get_token_total_supply(token_canister: Option<Principal>) -> ClanopediaResult<Nat> {
    match token_canister {
        None => Ok(Nat::from(0u64)),
        Some(canister) => {
            match call::<_, (Nat,)>(canister, "icrc1_total_supply", ()).await {
                Ok((supply,)) => Ok(supply),
                Err(e) => Err(ClanopediaError::BluebandError(format!("Failed to get total supply: {:?}", e))),
            }
        }
    }
}

pub async fn has_reached_quorum(
    total_votes: Nat,
    quorum_threshold: u32,
    token_canister: Option<Principal>,
) -> ClanopediaResult<bool> {
    let total_supply = get_token_total_supply(token_canister).await?;
    
    if total_supply == Nat::from(0u64) {
        return Ok(false);
    }
    
    // Calculate percentage of total supply that has voted
    let participation_percentage = (total_votes.clone() * Nat::from(100u64)) / total_supply.clone();
    Ok(participation_percentage >= Nat::from(quorum_threshold))
}

pub fn has_reached_threshold(
    votes_for: Nat,
    total_votes: Nat,
    threshold_percentage: u32,
) -> bool {
    if total_votes == Nat::from(0u64) {
        return false;
    }
    
    let approval_percentage = (votes_for.clone() * Nat::from(100u64)) / total_votes.clone();
    approval_percentage >= Nat::from(threshold_percentage)
}

// Helper function to validate token canister
pub async fn validate_token_canister(token_canister: Principal) -> bool {
    match call::<(), (Vec<(String, String)>,)>(token_canister, "icrc1_metadata", ()).await {
        Ok(_) => true,
        Err(e) => {
            ic_cdk::print(format!("Invalid token canister: {:?}", e));
            false
        }
    }
}