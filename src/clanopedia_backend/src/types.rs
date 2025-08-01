// src/clanopedia_backend/src/types.rs

use candid::{CandidType, Nat, Principal};
use ic_stable_structures::storable::Storable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use crate::external::blueband::ContentType;

pub type CollectionId = String;
pub type ProposalId = String;
pub type DocumentId = String;
pub type ClanopediaResult<T> = Result<T, ClanopediaError>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Collection {
    pub id: CollectionId,
    pub name: String,
    pub description: String,
    pub creator: Principal,
    pub created_at: u64,
    pub updated_at: u64,
    pub admins: Vec<Principal>,
    pub threshold: u32,
    pub governance_token: Option<Principal>,
    pub sns_governance_canister: Option<Principal>,
    pub governance_model: GovernanceModel,
    pub quorum_threshold: u32,
    pub is_permissionless: bool,
    pub blueband_collection_id: String,
    pub proposals: HashMap<ProposalId, Proposal>,
    pub cycles_balance: u64,
    pub proposal_counter: u64,
}

impl Default for Collection {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            creator: Principal::anonymous(),
            created_at: 0,
            updated_at: 0,
            admins: Vec::new(),
            threshold: 0,
            governance_token: None,
            sns_governance_canister: None,
            governance_model: GovernanceModel::Permissionless,
            quorum_threshold: 0,
            is_permissionless: false,
            blueband_collection_id: String::new(),
            proposals: HashMap::new(),
            cycles_balance: 0,
            proposal_counter: 0,
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CollectionConfig {
    pub name: String,
    pub description: String,
    pub admins: Vec<String>,
    pub threshold: u32,
    pub governance_token: Option<String>,
    pub sns_governance_canister: Option<String>,
    pub governance_model: GovernanceModel,
    pub quorum_threshold: u32,
    pub is_permissionless: bool,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Proposal {
    pub id: ProposalId,
    pub collection_id: CollectionId,
    pub proposal_type: ProposalType,
    pub creator: Principal,
    pub description: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub status: ProposalStatus,
    pub votes: HashMap<Principal, Vote>,
    pub token_votes: HashMap<Principal, Nat>,
    pub executed: bool,
    pub executed_at: Option<u64>,
    pub executed_by: Option<Principal>,
    pub threshold: u32,
    pub threshold_met: bool,
    pub sns_proposal_id: Option<u64>
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Vote {
    Yes,
    No,
    Abstain,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum GovernanceModel {
    Permissionless,
    Multisig,
    TokenBased,
    SnsIntegrated,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ProposalStatus {
    Active,
    Approved,
    Rejected,
    Expired,
    Executed,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ProposalType {
    EmbedDocument { documents: Vec<String> },
    BatchEmbed { document_ids: Vec<String> },
    AddAdmin { admin: Principal },
    RemoveAdmin { admin: Principal },
    ChangeThreshold { new_threshold: u32 },
    UpdateQuorum { new_percentage: u32 },
    UpdateCollection { config: CollectionConfig },
    ChangeGovernanceModel { model: GovernanceModel },
    DeleteCollection,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ClanopediaError {
    NotFound(String),
    NotAuthorized,
    InvalidProposalState(String),
    ProposalExpired,
    ThresholdNotMet,
    InsufficientCycles(String),
    ExternalCallError(String),
    StorageError(String),
    InvalidArgument(String),
    AlreadyExists(String),
    InvalidOperation(String),
    BluebandError(String),
    Unauthorized(String),
    InvalidInput(String),
    ProposalAlreadyExecuted,
    SnsError(String),
    SnsNotConfigured,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum StorageError {
    NotFound,
    AlreadyExists,
    Other(String),
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TokenVoteCount {
    pub total_yes: Nat,
    pub total_no: Nat,
    pub total_supply: Nat,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct DocumentRequest {
    pub title: String,
    pub content: String,
    pub content_type: Option<ContentType>,
    pub source_url: Option<String>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct SearchResult {
    pub document_id: DocumentId,
    pub title: String,
    pub content: String,
    pub score: f64,
}

impl fmt::Display for ClanopediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClanopediaError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ClanopediaError::NotAuthorized => write!(f, "Not authorized"),
            ClanopediaError::InvalidProposalState(msg) => {
                write!(f, "Invalid proposal state: {}", msg)
            }
            ClanopediaError::ProposalExpired => write!(f, "Proposal has expired"),
            ClanopediaError::ThresholdNotMet => write!(f, "Voting threshold not met"),
            ClanopediaError::InsufficientCycles(msg) => write!(f, "Insufficient cycles: {}", msg),
            ClanopediaError::ExternalCallError(msg) => write!(f, "External call error: {}", msg),
            ClanopediaError::StorageError(e) => write!(f, "Storage error: {:?}", e),
            ClanopediaError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            ClanopediaError::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            ClanopediaError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            ClanopediaError::BluebandError(msg) => write!(f, "Blueband error: {}", msg),
            ClanopediaError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            ClanopediaError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            ClanopediaError::ProposalAlreadyExecuted => write!(f, "Proposal already executed"),
            ClanopediaError::SnsError(msg) => write!(f, "SNS error: {}", msg),
            ClanopediaError::SnsNotConfigured => write!(f, "SNS not configured"),
        }
    }
}

impl From<ClanopediaError> for String {
    fn from(err: ClanopediaError) -> String {
        err.to_string()
    }
}

impl From<String> for ClanopediaError {
    fn from(err: String) -> Self {
        ClanopediaError::NotFound(err)
    }
}

impl From<&str> for ClanopediaError {
    fn from(err: &str) -> Self {
        ClanopediaError::NotFound(err.to_string())
    }
}

impl From<StorageError> for ClanopediaError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::NotFound => ClanopediaError::NotFound("Resource not found".to_string()),
            StorageError::AlreadyExists => {
                ClanopediaError::AlreadyExists("Resource already exists".to_string())
            }
            StorageError::Other(msg) => ClanopediaError::StorageError(msg),
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct BluebandDocument {
    pub id: DocumentId,
    pub title: String,
    pub content: String,
    pub embedded: bool,
    pub created_at: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct BluebandConfig {
    pub collection: String,
    pub api_key: Option<String>,
}

// Constants
pub const PROPOSAL_DURATION_NANOS: u64 = 7 * 24 * 60 * 60 * 1_000_000_000; // 7 days

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct GovernanceModelConfig {
    pub is_permissionless: bool,
    pub governance_token: Option<Principal>,
    pub quorum_threshold: u32,
    pub threshold: u32,
}

impl Default for GovernanceModel {
    fn default() -> Self {
        GovernanceModel::Permissionless
    }
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CyclesStatus {
    pub clanopedia_balance: u64,
    pub blueband_balance: u64,
    pub clanopedia_healthy: bool,
    pub blueband_healthy: bool,
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationCost {
    pub base_cost: u64,
    pub per_doc_cost: u64,
    pub buffer_percentage: u32,
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostMetrics {
    pub base_cost: u64,
    pub total_cost: u64,
    pub per_doc_cost: u64,
    pub buffer_amount: u64,
}

impl Storable for Proposal {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(candid::encode_one(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap_or_else(|_| Proposal {
            id: String::new(),
            collection_id: String::new(),
            proposal_type: ProposalType::EmbedDocument { documents: vec![] },
            creator: Principal::anonymous(),
            description: String::new(),
            created_at: 0,
            expires_at: 0,
            status: ProposalStatus::Active,
            votes: HashMap::new(),
            token_votes: HashMap::new(),
            executed: false,
            executed_at: None,
            executed_by: None,
            threshold: 0,
            threshold_met: false,
            sns_proposal_id: None
        })
    }

    const BOUND: ic_stable_structures::storable::Bound =
        ic_stable_structures::storable::Bound::Bounded {
            max_size: 1024 * 1024, // 1MB max size for a proposal
            is_fixed_size: false,
        };
}

impl Storable for Collection {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(candid::encode_one(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap_or_else(|_| Collection {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            creator: Principal::anonymous(),
            created_at: 0,
            updated_at: 0,
            admins: Vec::new(),
            threshold: 0,
            governance_token: None,
            sns_governance_canister: None,
            governance_model: GovernanceModel::Permissionless,
            quorum_threshold: 0,
            is_permissionless: false,
            blueband_collection_id: String::new(),
            proposals: HashMap::new(),
            cycles_balance: 0,
            proposal_counter: 0,
        })
    }

    const BOUND: ic_stable_structures::storable::Bound =
        ic_stable_structures::storable::Bound::Bounded {
            max_size: 2 * 1024 * 1024, // 2MB max size for a collection
            is_fixed_size: false,
        };
}
