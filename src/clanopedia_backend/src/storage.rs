// src/clanopedia_backend/src/storage.rs

use crate::types::*;
use candid::Principal;
use ic_cdk::api::time;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    DefaultMemoryImpl, StableBTreeMap,
};
use std::cell::RefCell;
use std::collections::HashMap;

// ============================
// STABLE STORAGE
// ============================

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static COLLECTIONS: RefCell<StableBTreeMap<CollectionId, Collection, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0)))
        )
    );

    static PROPOSALS: RefCell<StableBTreeMap<String, Proposal, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
        )
    );
}

// ============================
// COLLECTION OPERATIONS
// ============================

pub fn create_collection(
    collection_id: &CollectionId,
    mut config: CollectionConfig,
    creator: Principal,
) -> ClanopediaResult<()> {
    if COLLECTIONS.with(|c| c.borrow().contains_key(collection_id)) {
        return Err(ClanopediaError::AlreadyExists(format!(
            "Collection {} already exists",
            collection_id
        )));
    }

    // Convert string representations to Principal objects
    let admins: Result<Vec<Principal>, _> = config
        .admins
        .into_iter()
        .map(Principal::from_text)
        .collect();

    let governance_token = config
        .governance_token
        .map(Principal::from_text)
        .transpose()
        .map_err(|e| {
            ClanopediaError::InvalidInput(format!("Invalid governance token principal: {}", e))
        })?;

    let sns_governance_canister = config
        .sns_governance_canister
        .map(Principal::from_text)
        .transpose()
        .map_err(|e| {
            ClanopediaError::InvalidInput(format!("Invalid SNS governance canister principal: {}", e))
        })?;

    let collection = Collection {
        id: collection_id.clone(),
        name: config.name,
        description: config.description,
        admins: admins.unwrap_or_else(|_| vec![creator]), // Fall back to just the creator if any principal is invalid
        threshold: config.threshold,
        governance_token,
        sns_governance_canister,
        governance_model: config.governance_model,
        blueband_collection_id: String::new(),
        cycles_balance: 0,
        proposals: HashMap::new(),
        proposal_counter: 0,
        created_at: time(),
        creator,
        updated_at: time(),
        quorum_threshold: config.quorum_threshold,
        is_permissionless: config.is_permissionless,
    };

    COLLECTIONS.with(|c| {
        c.borrow_mut().insert(collection_id.clone(), collection);
    });

    Ok(())
}

pub fn get_collection(collection_id: &CollectionId) -> ClanopediaResult<Collection> {
    COLLECTIONS.with(|c| {
        c.borrow().get(collection_id).ok_or_else(|| {
            ClanopediaError::NotFound(format!("Collection {} not found", collection_id))
        })
    })
}

pub fn update_collection(
    collection_id: &CollectionId,
    collection: &Collection,
) -> ClanopediaResult<()> {
    if !COLLECTIONS.with(|c| c.borrow().contains_key(collection_id)) {
        return Err(ClanopediaError::NotFound(format!(
            "Collection {} not found",
            collection_id
        )));
    }

    COLLECTIONS.with(|c| {
        c.borrow_mut()
            .insert(collection_id.clone(), collection.clone());
    });

    Ok(())
}

pub fn delete_collection(collection_id: &CollectionId) -> ClanopediaResult<()> {
    if !COLLECTIONS.with(|c| c.borrow().contains_key(collection_id)) {
        return Err(ClanopediaError::NotFound(format!(
            "Collection {} not found",
            collection_id
        )));
    }

    // Delete all proposals for this collection
    let proposal_keys: Vec<String> = PROPOSALS.with(|p| {
        p.borrow()
            .iter()
            .filter(|(key, _)| key.starts_with(&format!("{}:", collection_id)))
            .map(|(k, _)| k.clone())
            .collect()
    });

    for key in proposal_keys {
        PROPOSALS.with(|p| {
            p.borrow_mut().remove(&key);
        });
    }

    // Delete collection
    COLLECTIONS.with(|c| {
        c.borrow_mut().remove(collection_id);
    });

    Ok(())
}

pub fn list_collections() -> Vec<Collection> {
    COLLECTIONS.with(|c| {
        c.borrow()
            .iter()
            .map(|(_, collection)| collection)
            .collect()
    })
}

// ============================
// PROPOSAL OPERATIONS
// ============================

pub fn update_proposal_in_storage(
    collection_id: &CollectionId,
    proposal: &Proposal,
) -> ClanopediaResult<()> {
    let mut collection = get_collection(collection_id)?;

    // Update or remove proposal based on status
    if proposal.status == ProposalStatus::Executed
        || proposal.status == ProposalStatus::Rejected
        || proposal.status == ProposalStatus::Expired
    {
        collection.proposals.remove(&proposal.id);
    } else {
        collection
            .proposals
            .insert(proposal.id.clone(), proposal.clone());
    }

    update_collection(collection_id, &collection)
}
