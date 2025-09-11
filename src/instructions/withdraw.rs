use {
    crate::{
        state::{Challenge, User},
        ScreenWarErrors,
    },
    pinocchio::{
        account_info::AccountInfo,
        instruction::{Seed, Signer},
        program_error::ProgramError,
        pubkey::find_program_address,
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
    pub user_pda: &'a AccountInfo,
    pub clock_sysvar: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub global_bump: u8,
}

pub struct WithdrawInstructionData {
    pub challenge_id: u32,
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for Withdraw<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, instruction_data): (&'a [AccountInfo], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = WithdrawAccounts::try_from(accounts)?;
        let instruction_data = WithdrawInstructionData::try_from(instruction_data)?;

        // validate correct challenge pda
        let (challenge_pda_key, _) = find_program_address(
            &[b"challenge", &instruction_data.challenge_id.to_le_bytes()],
            &crate::ID,
        );

        if challenge_pda_key.ne(accounts.challenge.key()) {
            return Err(ScreenWarErrors::InvalidChallengePDA.into());
        }

        // return Self
        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> TryFrom<&'a [AccountInfo]> for WithdrawAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [user, global, challenge, user_pda, clock_sysvar, system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !user.is_signer() {
            return Err(ScreenWarErrors::NotSigner.into());
        }

        let (user_pda_key, _) = find_program_address(&[b"user", user.key().as_slice()], &crate::ID);

        if (&user_pda_key).ne(user_pda.key()) {
            return Err(ScreenWarErrors::InvalidUserPDA.into());
        };

        let (global_pda_key, global_bump) = find_program_address(&[b"global"], &crate::ID);
        if global.key().ne(&global_pda_key) {
            return Err(ProgramError::InvalidSeeds);
        };

        Ok(Self {
            user,
            global,
            challenge,
            user_pda,
            clock_sysvar,
            system_program,
            global_bump,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for WithdrawInstructionData {
    type Error = ProgramError;

    fn try_from(instruction_data: &'a [u8]) -> Result<Self, Self::Error> {
        if instruction_data.len().ne(&4usize) {
            return Err(ProgramError::InvalidInstructionData);
        }

        let challenge_id = u32::from_le_bytes(instruction_data.try_into().unwrap());

        Ok(Self { challenge_id })
    }
}

impl<'a> Withdraw<'a> {
    pub const DISCRIMINATOR: &'a u8 = &3;

    pub fn process(&mut self) -> ProgramResult {
        // get reference to user and challenge pda datas
        let mut challenge_raw_data = self.accounts.challenge.try_borrow_mut_data()?;
        let challenge = Challenge::load_mut(&mut challenge_raw_data)?;

        let user_pda_raw_data = self.accounts.user_pda.try_borrow_data()?;
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

        // close user_pda
        Self::close_user_pda(challenge)?;

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

    pub fn close_user_pda(challenge: &mut Challenge) -> ProgramResult {
        todo!()
        // Ok(())
    }
}
