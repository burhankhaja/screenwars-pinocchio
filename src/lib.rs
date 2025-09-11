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
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    match _instruction_data.split_first() {
        Some((Initialize::DISCRIMINATOR, _)) => Initialize::try_from(_accounts)?.process()?,
        Some((CreateChallenge::DISCRIMINATOR, data)) => {
            CreateChallenge::try_from((_accounts, data))?.process()?
        }
        Some((JoinChallenge::DISCRIMINATOR, data)) => {
            JoinChallenge::try_from((_accounts, data))?.process()?
        }

        _ => Err(ProgramError::InvalidInstructionData)?,
    }
    Ok(())
}
