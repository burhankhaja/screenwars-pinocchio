#![no_std]

use pinocchio::{
    account_info::AccountInfo, entrypoint, nostd_panic_handler, program_error::ProgramError,
    ProgramResult,
};

pub mod instructions;
pub mod state;
pub use instructions::*;
pub use state::*;

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
        _ => Err(ProgramError::InvalidInstructionData)?,
    }
    Ok(())
}
