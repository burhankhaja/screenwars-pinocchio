use {
    crate::{state::Global, ScreenWarErrors},
    pinocchio::{
        account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
    },
};

pub struct ToggleChallengeCreation<'a> {
    pub accounts: ToggleChallengeCreationAccounts<'a>,
    pub instruction_data: ToggleChallengeCreationInstructionData,
}

pub struct ToggleChallengeCreationAccounts<'a> {
    pub user: &'a AccountInfo,
    pub global: &'a AccountInfo,
}

pub struct ToggleChallengeCreationInstructionData {
    pub pause: bool,
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for ToggleChallengeCreation<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, instruction_data): (&'a [AccountInfo], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        todo!();
    }
}

//@audit-issue :: add Admin Signer validations otherwise anyone can get unauthorized access
impl<'a> TryFrom<&'a [AccountInfo]> for ToggleChallengeCreationAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        //@audit-issue:: validate Global Pda ?

        todo!();
    }
}

impl<'a> TryFrom<&'a [u8]> for ToggleChallengeCreationInstructionData {
    type Error = ProgramError;

    fn try_from(instruction_data: &'a [u8]) -> Result<Self, Self::Error> {
        todo!();
    }
}

impl<'a> ToggleChallengeCreation<'a> {
    pub const DISCRIMINATOR: &'a u8 = &5;

    pub fn process(&mut self) -> ProgramResult {
        // get mutable ref to Global Pda
        let mut global_raw_data = self.accounts.global.try_borrow_mut_data()?;
        let global = Global::load_mut(&mut global_raw_data)?;

        // validate toggle
        if global
            .challenge_creation_paused
            .eq(&self.instruction_data.pause)
        {
            return Err(ScreenWarErrors::ChallengeStateAlreadySet.into());
        }

        // toggle pause state
        global.challenge_creation_paused = self.instruction_data.pause;

        Ok(())
    }
}
