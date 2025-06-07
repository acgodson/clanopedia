// src/clanopedia_backend/src/storage.rs - Final fix removing duplicate Storable implementation

use crate::types::*;
use candid::Principal;
use ic_cdk::api::time;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    DefaultMemoryImpl, StableBTreeMap, Storable,
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

    static PROPOSALS: RefCell<StableBTreeMap<(CollectionId, ProposalId), Proposal, Memory>> = RefCell::new(
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
    config: CollectionConfig,
    creator: Principal,
) -> ClanopediaResult<()> {
    if COLLECTIONS.with(|c| c.borrow().contains_key(collection_id)) {
        return Err(ClanopediaError::AlreadyExists(format!(
            "Collection {} already exists",
            collection_id
        )));
    }

    let collection = Collection {
        id: collection_id.clone(),
        name: config.name,
        description: config.description,
        admins: config.admins,
        threshold: config.threshold,
        governance_token: config.governance_token,
        governance_model: config.governance_model,
        genesis_owner: config.genesis_owner,
        members: config.members,
        blueband_collection_id: String::new(), // Set by Blueband interface
        cycles_balance: 0,
        active_proposals: HashMap::new(),
        proposal_counter: 0,
        created_at: time(),
        creator,
        updated_at: time(),
        proposals: Vec::new(),
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
    let proposal_keys: Vec<(CollectionId, ProposalId)> = PROPOSALS.with(|p| {
        p.borrow()
            .iter()
            .filter(|((cid, _), _)| cid == collection_id)
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

pub fn store_proposal(collection_id: &CollectionId, proposal: &Proposal) -> ClanopediaResult<()> {
    if !COLLECTIONS.with(|c| c.borrow().contains_key(collection_id)) {
        return Err(ClanopediaError::NotFound(format!(
            "Collection {} not found",
            collection_id
        )));
    }

    let key = (collection_id.clone(), proposal.id.clone());
    PROPOSALS.with(|p| {
        p.borrow_mut().insert(key, proposal.clone());
    });

    // Update collection's active proposals
    let mut collection = get_collection(collection_id)?;
    collection
        .active_proposals
        .insert(proposal.id.clone(), proposal.clone());
    update_collection(collection_id, &collection)?;

    Ok(())
}

pub fn get_proposal_by_collection_and_id(
    collection_id: &CollectionId,
    proposal_id: &ProposalId,
) -> ClanopediaResult<Proposal> {
    let key = (collection_id.clone(), proposal_id.clone());
    PROPOSALS.with(|p| {
        p.borrow().get(&key).ok_or_else(|| {
            ClanopediaError::NotFound(format!(
                "Proposal {} not found in collection {}",
                proposal_id, collection_id
            ))
        })
    })
}

pub fn update_proposal_in_storage(
    collection_id: &CollectionId,
    proposal: &Proposal,
) -> ClanopediaResult<()> {
    let key = (collection_id.clone(), proposal.id.clone());
    if !PROPOSALS.with(|p| p.borrow().contains_key(&key)) {
        return Err(ClanopediaError::NotFound(format!(
            "Proposal {} not found in collection {}",
            proposal.id, collection_id
        )));
    }

    PROPOSALS.with(|p| {
        p.borrow_mut().insert(key, proposal.clone());
    });

    // Update collection's active proposals
    let mut collection = get_collection(collection_id)?;
    if proposal.status == ProposalStatus::Executed
        || proposal.status == ProposalStatus::Rejected
        || proposal.status == ProposalStatus::Expired
    {
        collection.active_proposals.remove(&proposal.id);
    } else {
        collection
            .active_proposals
            .insert(proposal.id.clone(), proposal.clone());
    }
    update_collection(collection_id, &collection)?;

    Ok(())
}

pub fn list_proposals(collection_id: &CollectionId) -> Vec<Proposal> {
    PROPOSALS.with(|p| {
        p.borrow()
            .iter()
            .filter(|((cid, _), _)| cid == collection_id)
            .map(|(_, proposal)| proposal)
            .collect()
    })
}

pub fn list_active_proposals(collection_id: &CollectionId) -> Vec<Proposal> {
    PROPOSALS.with(|p| {
        p.borrow()
            .iter()
            .filter(|((cid, _), proposal)| {
                cid == collection_id && proposal.status == ProposalStatus::Active
            })
            .map(|(_, proposal)| proposal)
            .collect()
    })
}

// Helper functions for proposal expiry
pub fn is_proposal_expired(proposal: &Proposal) -> bool {
    ic_cdk::api::time() > proposal.expires_at
}

// ============================
// STORABLE IMPLEMENTATIONS
// ============================

// Note: Collection and Proposal Storable implementations are in types.rs to avoid conflicts
