use {
    crate::{
        state::{Challenge, User},
        ScreenWarErrors,
    },
    pinocchio::{
        account_info::AccountInfo,
        instruction::{Seed, Signer},
        program_error::ProgramError,
        sysvars::{clock::Clock, Sysvar},
        ProgramResult,
    },
    pinocchio_system::instructions::Transfer,
};

pub struct Withdraw<'a> {
    pub accounts: WithdrawAccounts<'a>,
    pub instruction_data: WithdrawInstructionData,
}

pub struct WithdrawAccounts<'a> {
    pub user: &'a AccountInfo,
    pub global: &'a AccountInfo,
    pub challenge: &'a AccountInfo,
    pub user_account: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub global_bump: u8,
}

pub struct WithdrawInstructionData {
    pub amount: u64,
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for Withdraw<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, instruction_data): (&'a [AccountInfo], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        todo!();
    }
}

impl<'a> TryFrom<&'a [AccountInfo]> for WithdrawAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        //@audit-issue:: Dont forget to validate ALL PDAS IN TRY_FROM BLOCKS ??

        todo!();
    }
}

impl<'a> TryFrom<&'a [u8]> for WithdrawInstructionData {
    type Error = ProgramError;

    fn try_from(instruction_data: &'a [u8]) -> Result<Self, Self::Error> {
        todo!();
    }
}

impl<'a> Withdraw<'a> {
    pub const DISCRIMINATOR: &'a u8 = &3;

    pub fn process(&mut self) -> ProgramResult {
        // get the mutable reference to user and challenge pda datas
        let challenge_raw_data = self.accounts.challenge.try_borrow_data()?;
        let challenge = Challenge::load(&challenge_raw_data)?;

        let user_pda_raw_data = self.accounts.user_account.try_borrow_data()?;
        let user_pda = User::load(&user_pda_raw_data)?;

        // validations
        Self::validate_contention_period_is_over(challenge.end)?;
        Self::validate_user_is_enrolled_in_challenge(
            challenge.challenge_id,
            user_pda.challenge_id,
        )?;

        // transfer
        Self::transfer_sol(
            self.accounts.global,
            self.accounts.user,
            user_pda.locked_balance,
            self.accounts.global_bump,
        )?;

        // close challenge_pda
        todo!();

        Ok(())
    }

    pub fn validate_contention_period_is_over(end: i64) -> ProgramResult {
        let now = Clock::get()?.unix_timestamp;
        let five_days = 5 * 24 * 60 * 60;
        let contention_period = end
            .checked_add(five_days)
            .ok_or(ScreenWarErrors::ContentionPhase)?;

        if contention_period > now {
            return Err(ScreenWarErrors::ContentionPhase.into());
        }

        Ok(())
    }

    pub fn validate_user_is_enrolled_in_challenge(
        challenge_pda_id: u32,
        users_pda_challenge_id: u32,
    ) -> ProgramResult {
        if challenge_pda_id.ne(&users_pda_challenge_id) {
            return Err(ScreenWarErrors::NotEnrolled.into());
        }

        Ok(())
    }

    pub fn transfer_sol(
        global: &AccountInfo,
        user: &AccountInfo,
        locked_balance: u64,
        global_bump: u8,
    ) -> ProgramResult {
        if locked_balance > 0 {
            let global_bump_binding = [global_bump];
            let seeds = &[Seed::from(b"global"), Seed::from(&global_bump_binding)];
            let global_pda_signature = Signer::from(seeds);

            Transfer {
                from: global,
                to: user,
                lamports: locked_balance,
            }
            .invoke_signed(&[global_pda_signature])?;

            // dev : checkout pinocchio::instructions::TransferWithSeeds
        }

        Ok(())
    }
}
