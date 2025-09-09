use {
    crate::{
        custom_errors::ScreenWarErrors,
        state::{Challenge, Global},
    },
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

pub struct CreateChallenge<'a> {
    pub accounts: CreateChallengeAccounts<'a>,
    pub instruction_data: CreateChallengeInstructionData,
}

pub struct CreateChallengeAccounts<'a> {
    pub creator: &'a AccountInfo,
    pub global_pda: &'a AccountInfo,
    pub challenge_pda: &'a AccountInfo,
    pub rent_sysvar: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub challenge_bump: u8,
    pub current_challenge_id: u32,
}

pub struct CreateChallengeInstructionData {
    pub start_time: i64,
    pub daily_timer: i64,
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for CreateChallenge<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a [AccountInfo], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = CreateChallengeAccounts::try_from(accounts)?;
        let instruction_data = CreateChallengeInstructionData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> TryFrom<&'a [AccountInfo]> for CreateChallengeAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [creator, global_pda, challenge_pda, rent_sysvar, system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // validate global pda
        let (global_pda_key, _) = find_program_address(&[b"global"], &crate::ID);
        if global_pda.key().ne(&global_pda_key) {
            return Err(ProgramError::InvalidSeeds);
        }

        // fetching global_pda data
        let global_pda_raw_data = global_pda.try_borrow_data()?;
        let global = Global::load(&global_pda_raw_data)?;
        let current_challenge_id = global.challenge_ids;

        // validate challenge pda
        let (challenge_pda_key, challenge_bump) = find_program_address(
            &[b"challenge", &current_challenge_id.to_le_bytes()],
            &crate::ID,
        );

        if challenge_pda.key().ne(&challenge_pda_key) {
            return Err(ScreenWarErrors::InvalidChallengePDA.into());
        }

        // validate challenge creation is not paused
        if global.challenge_creation_paused {
            return Err(ScreenWarErrors::ChallengeCreationPaused.into());
        }

        // return Self
        Ok(Self {
            creator,
            global_pda,
            challenge_pda,
            rent_sysvar,
            system_program,
            challenge_bump,
            current_challenge_id,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for CreateChallengeInstructionData {
    type Error = ProgramError;

    fn try_from(instruction_data: &'a [u8]) -> Result<Self, Self::Error> {
        if instruction_data.len().ne(&(16 as usize)) {
            return Err(ProgramError::InvalidInstructionData);
        };

        // dev
        // first 8 bytes would be start_time
        // second 8 bytes would be daily_timer
        let (start, timer) = instruction_data.split_at(8);

        let start_time = i64::from_le_bytes(start.try_into().unwrap());
        let daily_timer = i64::from_le_bytes(timer.try_into().unwrap());

        // validations
        let now = Clock::get()?.unix_timestamp;
        let two_hours = 2 * (60 * 60);
        let one_day = two_hours * 12;
        let one_week = one_day * 7;

        if start_time < now + one_day {
            return Err(ScreenWarErrors::ChallengeStartsTooSoon)?;
        }

        if start_time >= now + one_week {
            return Err(ScreenWarErrors::ChallengeStartsTooFar)?;
        }

        if daily_timer >= two_hours {
            return Err(ScreenWarErrors::ChallengeExceedsTwoHours)?;
        }

        Ok(Self {
            start_time,
            daily_timer,
        })
    }
}

impl<'a> CreateChallenge<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1;

    pub fn process(&mut self) -> ProgramResult {
        //// initialize challenge pda data
        let three_weeks = 3 * 7 * 24 * 60 * 60;
        let end_time = self
            .instruction_data
            .start_time
            .checked_add(three_weeks)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let space = Challenge::LEN;
        let rent = Rent::get()?.minimum_balance(space);
        let id_binding = self.accounts.current_challenge_id.to_le_bytes();
        let bump_binding = [self.accounts.challenge_bump];
        let seeds = &[
            Seed::from(b"challenge"),
            Seed::from(&id_binding),
            Seed::from(&bump_binding),
        ];

        let signers = Signer::from(seeds);

        CreateAccount {
            from: self.accounts.creator,
            to: self.accounts.challenge_pda,
            lamports: rent,
            space: space as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&[signers])?;

        let mut challenge_pda_raw_data = self.accounts.challenge_pda.try_borrow_mut_data()?;
        let challenge: &mut Challenge = Challenge::load_mut(&mut challenge_pda_raw_data)?;

        *challenge = Challenge {
            creator: *self.accounts.creator.key(),
            challenge_id: self.accounts.current_challenge_id,
            daily_timer: self.instruction_data.daily_timer,
            start: self.instruction_data.start_time,
            end: end_time,
            bump: self.accounts.challenge_bump,
            ..Challenge::default()
        };

        //// increment global challenge ids in global_pda
        let mut global_pda_raw_data = self.accounts.global_pda.try_borrow_mut_data()?;
        let global = Global::load_mut(&mut global_pda_raw_data)?;
        global.challenge_ids = global
            .challenge_ids
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }
}
