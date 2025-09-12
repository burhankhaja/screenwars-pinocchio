#![no_std]

pub mod custom_errors;
pub mod instructions;
pub mod mock_oracle;
pub mod state;

pub use {
    custom_errors::ScreenWarErrors,
    instructions::*,
    mock_oracle::*,
    pinocchio::{
        account_info::AccountInfo, entrypoint, nostd_panic_handler, program_error::ProgramError,
        ProgramResult,
    },
    state::*,
};

nostd_panic_handler!();
entrypoint!(process_instruction);

pinocchio_pubkey::declare_id!("ScreenWars111111111111111111111111111111111");

pub fn process_instruction(
    _program_id: &[u8; 32],
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.split_first() {
        // initialize
        Some((Initialize::DISCRIMINATOR, _)) => Initialize::try_from(accounts)?.process()?,

        // create challenge
        Some((CreateChallenge::DISCRIMINATOR, data)) => {
            CreateChallenge::try_from((accounts, data))?.process()?
        }

        // join challenge
        Some((JoinChallenge::DISCRIMINATOR, data)) => {
            JoinChallenge::try_from((accounts, data))?.process()?
        }

        // sync and lock
        Some((SyncLock::DISCRIMINATOR, data)) => SyncLock::try_from((accounts, data))?.process()?,

        // claim winner position
        Some((ClaimWinnerPosition::DISCRIMINATOR, data)) => {
            ClaimWinnerPosition::try_from((accounts, data))?.process()?
        }

        // withdraw locked funds
        Some((Withdraw::DISCRIMINATOR, data)) => Withdraw::try_from((accounts, data))?.process()?,

        // claim rewards as winner
        Some((ClaimRewards::WINNER_REWARD_DISCRIMINATOR, data)) => {
            ClaimRewards::try_from((accounts, data))?.process_winner_rewards()?
        }

        // claim rewards as creator
        Some((ClaimRewards::CREATOR_REWARD_DISCRIMINATOR, data)) => {
            ClaimRewards::try_from((accounts, data))?.process_creator_rewards()?
        }

        // take protocol profits (#admin)
        Some((TakeProfit::DISCRIMINATOR, data)) => {
            TakeProfit::try_from((accounts, data))?.process()?
        }

        // pause unpause challenge creation (#admin)
        Some((ToggleChallengeCreation::DISCRIMINATOR, data)) => {
            ToggleChallengeCreation::try_from((accounts, data))?.process()?
        }

        _ => Err(ProgramError::InvalidInstructionData)?,
    }
    Ok(())
}
