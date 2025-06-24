// src/clanopedia_backend/src/external/token.rs
use candid::{Principal, Nat};
use ic_cdk::call;
use std::result::Result;
use crate::types::*;
use icrc_ledger_types::{
    icrc1::account::Account,
    icrc1::transfer::{TransferArg, TransferError},
};

// ============================
// TOKEN INTERFACE
// ============================

pub type TokenResult<T> = Result<T, TransferError>;

pub struct TokenService {
    canister_id: Principal,
}

impl TokenService {
    pub fn new(canister_id: Principal) -> Self {
        Self { canister_id }
    }

    pub async fn icrc1_balance_of(&self, account: Account) -> TokenResult<Nat> {
        let result: Result<(Nat,), _> = call(
            self.canister_id,
            "icrc1_balance_of",
            (account,),
        ).await;

        match result {
            Ok((balance,)) => Ok(balance),
            Err((_, e)) => Err(TransferError::GenericError { error_code: Nat::from(1u64), message: format!("Call failed: {}", e) }),
        }
    }

    pub async fn icrc1_total_supply(&self) -> TokenResult<Nat> {
        let result: Result<(Nat,), _> = call(
            self.canister_id,
            "icrc1_total_supply",
            (),
        ).await;

        match result {
            Ok((supply,)) => Ok(supply),
            Err((_, e)) => Err(TransferError::GenericError { error_code: Nat::from(1u64), message: format!("Call failed: {}", e) }),
        }
    }

    pub async fn icrc1_transfer(&self, transfer_arg: TransferArg) -> TokenResult<Nat> {
        let result: Result<(TokenResult<Nat>,), _> = call(
            self.canister_id,
            "icrc1_transfer",
            (transfer_arg,),
        ).await;

        match result {
            Ok((result,)) => result,
            Err((_, e)) => Err(TransferError::GenericError { error_code: Nat::from(1u64), message: format!("Call failed: {}", e) }),
        }
    }
}

// ============================
// TOKEN CLIENT FUNCTIONS
// ============================

pub async fn get_token_balance(token_canister: Principal, owner: Principal) -> ClanopediaResult<Nat> {
    let service = TokenService::new(token_canister);
    let account = Account {
        owner,
        subaccount: None,
    };
    service.icrc1_balance_of(account)
        .await
        .map_err(|e| ClanopediaError::ExternalCallError(format!("Token balance check failed: {:?}", e)))
}

pub async fn get_token_total_supply(token_canister: Principal) -> ClanopediaResult<Nat> {
    let service = TokenService::new(token_canister);
    service.icrc1_total_supply()
        .await
        .map_err(|e| ClanopediaError::ExternalCallError(format!("Token total supply check failed: {:?}", e)))
   
}

