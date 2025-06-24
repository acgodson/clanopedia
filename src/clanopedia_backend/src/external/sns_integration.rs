// src/clanopedia_backend/src/sns_integration.rs
use crate::types::{ClanopediaError, ClanopediaResult, ProposalType};
use candid::{CandidType, Deserialize, Principal};
use ic_cdk::api::call::call;

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct MakeProposalRequest {
    pub url: String,
    pub title: String,
    pub summary: String,
    pub action: Option<Action>,
    pub proposer: Option<Principal>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct MakeProposalResponse {
    pub proposal_id: Option<u64>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub enum Action {
    ExecuteGenericNervousSystemFunction { function_id: u64, payload: Vec<u8> },
    // Add other SNS action types as needed
    UpgradeSnsToNextVersion { target_version: Option<u64> },
    ExecuteNervousSystemFunction { function_id: u64, payload: Vec<u8> },
}

// SNS-specific proposal data structures
#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct GetProposalRequest {
    pub proposal_id: u64,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct SnsProposalData {
    pub id: Option<u64>,
    pub proposer: Option<Principal>,
    pub reject_cost_e8s: u64,
    pub proposal: Option<SnsProposal>,
    pub ballots: Vec<SnsBallot>,
    pub initial_voting_period: u64,
    pub current_voting_period: u64,
    pub decided_timestamp_seconds: u64,
    pub executed_timestamp_seconds: u64,
    pub failed_timestamp_seconds: u64,
    pub failure_reason: Option<String>,
    pub latest_tally: Option<SnsTally>,
    pub reward_event_round: u64,
    pub is_eligible_for_rewards: bool,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct SnsProposal {
    pub title: String,
    pub summary: String,
    pub url: String,
    pub action: Option<Action>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct SnsBallot {
    pub vote: SnsVote,
    pub voting_power: u64,
    pub cast_timestamp_seconds: u64,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub enum SnsVote {
    Yes,
    No,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct SnsTally {
    pub yes: u64,
    pub no: u64,
    pub total: u64,
    pub timestamp_seconds: u64,
}

// SNS proposal status enum
#[derive(CandidType, Deserialize, Clone, Debug, PartialEq)]
pub enum SnsProposalStatus {
    Open,
    Rejected,
    Adopted,
    Executed,
    Failed,
}

// Enhanced SNS proposal status checking
pub async fn check_sns_proposal_status(
    sns_governance_canister: Principal,
    proposal_id: u64,
) -> ClanopediaResult<SnsProposalStatus> {
    let request = GetProposalRequest { proposal_id };

    let (response,): (Option<SnsProposalData>,) =
        call(sns_governance_canister, "get_proposal", (request,))
            .await
            .map_err(|e| {
                ic_cdk::println!("SNS proposal status check failed: {:?}", e);
                ClanopediaError::ExternalCallError(format!("SNS call failed: {:?}", e))
            })?;

    if let Some(proposal_data) = response {
        // Determine status based on SNS proposal data
        if proposal_data.executed_timestamp_seconds > 0 {
            Ok(SnsProposalStatus::Executed)
        } else if proposal_data.failed_timestamp_seconds > 0 {
            Ok(SnsProposalStatus::Failed)
        } else if proposal_data.decided_timestamp_seconds > 0 {
            // Check if proposal was adopted or rejected
            if let Some(tally) = proposal_data.latest_tally {
                if tally.yes > tally.no {
                    Ok(SnsProposalStatus::Adopted)
                } else {
                    Ok(SnsProposalStatus::Rejected)
                }
            } else {
                Ok(SnsProposalStatus::Open)
            }
        } else {
            Ok(SnsProposalStatus::Open)
        }
    } else {
        Err(ClanopediaError::NotFound(format!(
            "SNS proposal {} not found",
            proposal_id
        )))
    }
}

// Check if SNS proposal is approved (for backward compatibility)
pub async fn check_sns_proposal_approved(
    sns_governance_canister: Principal,
    proposal_id: u64,
) -> ClanopediaResult<bool> {
    let status = check_sns_proposal_status(sns_governance_canister, proposal_id).await?;
    Ok(status == SnsProposalStatus::Adopted || status == SnsProposalStatus::Executed)
}
