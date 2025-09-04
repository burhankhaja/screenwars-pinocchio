#![no_std]

use pinocchio::{nostd_panic_handler, entrypoint, account_info::AccountInfo, ProgramResult};

nostd_panic_handler!();
entrypoint!(process_instruction);

pinocchio_pubkey::declare_id!("ScreenWars111111111111111111111111111111111");


pub fn process_instruction(
    _program_id : &[u8; 32],
    _accounts : &[AccountInfo],
    _instruction_data : &[u8],
) -> ProgramResult {
    Ok(())
}