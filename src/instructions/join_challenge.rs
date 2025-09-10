use crate::ScreenWarErrors;
pub use {
    crate::state::{Challenge, User},
    pinocchio::{
        account_info::AccountInfo,
        instruction::{Seed, Signer},
        program_error::ProgramError,
        pubkey::find_program_address,
        sysvars::{clock::Clock, rent::Rent, Sysvar},
        ProgramResult,
    },
    pinocchio_system::instructions::CreateAccount,
};

pub struct JoinChallenge<'a> {
    pub accounts: JoinChallengeAccounts<'a>,
    pub instruction_data: JoinChallengeInstructionData,
}

pub struct JoinChallengeAccounts<'a> {
    pub user: &'a AccountInfo,
    pub challenge: &'a AccountInfo,
    pub user_pda: &'a AccountInfo,
    pub rent_sysvar: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub user_pda_bump: u8,
}
pub struct JoinChallengeInstructionData {
    pub challenge_id: u32,
}

impl<'a> TryFrom<&'a [AccountInfo]> for JoinChallengeAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [user, challenge, user_pda, rent_sysvar, system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys)?;
        };

        let (user_pda_key, user_pda_bump) =
            find_program_address(&[b"user", user.key().as_slice()], &crate::ID);

        if (&user_pda_key).ne(user_pda.key()) {
            return Err(ScreenWarErrors::InvalidUserPDA.into());
        };

        // dev : Safer -> no need for signer validations or init_if checks

        Ok(Self {
            user,
            challenge,
            user_pda,
            rent_sysvar,
            system_program,
            user_pda_bump,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for JoinChallengeInstructionData {
    type Error = ProgramError;

    fn try_from(instruction_data: &'a [u8]) -> Result<Self, Self::Error> {
        if instruction_data.len().ne(&4usize) {
            return Err(ProgramError::InvalidInstructionData);
        }

        let challenge_id = u32::from_le_bytes(instruction_data.try_into().unwrap());

        Ok(Self { challenge_id })
    }
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for JoinChallenge<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, instruction_data): (&'a [AccountInfo], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = JoinChallengeAccounts::try_from(accounts)?;
        let instruction_data = JoinChallengeInstructionData::try_from(instruction_data)?;

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

impl<'a> JoinChallenge<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2;

    pub fn process(&mut self) -> ProgramResult {
        let mut challenge_ptr = self.accounts.challenge.try_borrow_mut_data()?;
        let challenge: &mut Challenge = Challenge::load_mut(&mut challenge_ptr)?;

        //// validate challenge has not started
        let now = Clock::get()?.unix_timestamp;

        if now > challenge.start {
            return Err(ScreenWarErrors::JoinedLate.into());
        };

        ///// increment challenge participants
        challenge.total_participants = challenge
            .total_participants
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        //// create and initialize user_pda
        let space = User::LEN;
        let rent = Rent::get()?.minimum_balance(space);
        let user_pda_bump_binding = [self.accounts.user_pda_bump];
        let seeds = &[
            Seed::from(b"user"),
            Seed::from(self.accounts.user.key()),
            Seed::from(&user_pda_bump_binding),
        ];
        let pda_signature = Signer::from(seeds);

        CreateAccount {
            from: self.accounts.user,
            to: self.accounts.user_pda,
            lamports: rent,
            space: space as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&[pda_signature])?;

        let mut user_pda_ptr = self.accounts.user_pda.try_borrow_mut_data()?;
        let user_pda = User::load_mut(&mut user_pda_ptr)?;

        *user_pda = User {
            user: *self.accounts.user.key(),
            challenge_id: self.instruction_data.challenge_id,
            bump: self.accounts.user_pda_bump,
            ..User::default()
        };

        Ok(())
    }
}
