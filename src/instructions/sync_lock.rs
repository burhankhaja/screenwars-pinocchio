use {
    crate::{
        mock_oracle::{mock_offchain_oracle_component, DebugData},
        state::{Challenge, User},
        ScreenWarErrors,
    },
    pinocchio::{
        account_info::AccountInfo,
        instruction::{Seed, Signer},
        program_error::{self, ProgramError},
        pubkey::{find_program_address, Pubkey},
        sysvars::{clock::Clock, Sysvar},
        ProgramResult,
    },
    pinocchio_system::instructions::Transfer,
};

pub struct SyncLock<'a> {
    pub accounts: SyncLockAccounts<'a>,
    pub instruction_data: SyncLockInstructionData,
}

pub struct SyncLockAccounts<'a> {
    pub user: &'a AccountInfo,
    pub global: &'a AccountInfo,
    pub challenge: &'a AccountInfo,
    pub user_pda: &'a AccountInfo,
    pub clock_sysvar: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub global_bump: u8,
}

pub struct SyncLockInstructionData {
    pub challenge_id: u32,
    pub debug_data: Option<DebugData>, // dev-practice : later try with scenarios where there is another Type after option, see how deserialization of instrucitions become different
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for SyncLock<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, instruction_data): (&'a [AccountInfo], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = SyncLockAccounts::try_from(accounts)?;
        let instruction_data = SyncLockInstructionData::try_from(instruction_data)?;

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

impl<'a> TryFrom<&'a [AccountInfo]> for SyncLockAccounts<'a> {
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

impl<'a> TryFrom<&'a [u8]> for SyncLockInstructionData {
    type Error = ProgramError;

    fn try_from(instruction_data: &'a [u8]) -> Result<Self, Self::Error> {
        // dev : mental_model
        // challenge_id = 8*4 = 4 bytes
        // Option (None/Some) = 1 byte
        // DebugData
        //   .. bool = 1 byte
        //   ... u8 = 1 byte
        //   ... bool = 1 byte
        // 8 bytes array
        // challenge_id :  bytes[0..=3]  [0,1,2,3]
        // Option<None/Some> : bytes[4]  [4]
        // DebugData :  bytes[5..=7]     [5,6,7] : [user_passed, days_not_synced, synced_today]
        //

        // Option<DebugData>
        //         pub struct DebugData {
        //     pub user_passed: bool,
        //     pub days_not_synced: u8,
        //     pub synced_today: bool,
        // }

        let challenge_id = u32::from_le_bytes(instruction_data[0..=3].try_into().unwrap());
        let debug_data: Option<DebugData> = match instruction_data[4] {
            0 => None,
            1 => Some(DebugData {
                user_passed: instruction_data[5] != 0,
                days_not_synced: instruction_data[6],
                synced_today: instruction_data[7] != 0,
            }),
            _ => None,
        };

        Ok(Self {
            challenge_id,
            debug_data,
        })
    }
}

impl<'a> SyncLock<'a> {
    pub const DISCRIMINATOR: &'a u8 = &9;
    pub const DAILY_LAMPORTS: u64 = 10_000_000;

    pub fn process(&mut self) -> ProgramResult {
        // get mutable refs
        let mut challenge_raw_data = self.accounts.challenge.try_borrow_mut_data()?;
        let mut challenge = Challenge::load_mut(&mut challenge_raw_data)?;

        let mut user_pda_raw_data = self.accounts.user_pda.try_borrow_mut_data()?;
        let user_pda = User::load_mut(&mut user_pda_raw_data)?;

        // validations
        let now = Clock::get()?.unix_timestamp;
        Self::validate_challenge_has_started(now, challenge.start)?;
        Self::validate_challenge_has_not_ended(now, challenge.end)?;

        let (user_passed_today, days_not_synced, synced_today) =
            mock_offchain_oracle_component(&self.instruction_data.debug_data)?;

        Self::validated_today_not_synced_already(synced_today)?;

        let today = 1;

        // deposit
        Self::deposit_total_daily_lamports(self, days_not_synced + today)?;

        let mut days_not_synced_or_failed = days_not_synced;

        if !user_passed_today {
            days_not_synced_or_failed += 1;
        }

        if days_not_synced_or_failed > 0 {
            Self::reset_streak(user_pda)?;

            let current_balance = user_pda.locked_balance;
            let lb_penalty = Self::calculate_exponential_penalty_on_locked_balance(
                current_balance,
                days_not_synced_or_failed,
            )?;

            // slash
            Self::update_users_locked_balance(user_pda, -(lb_penalty as i64))?;

            // dev : total penalty is applied by slashing all the  daily_lamports + 25% of previous locked_balance
            // :: (SyncLock::DAILY_LAMPORTS * days_not_synced_or_failed) + lb_penalty
            let total_penalty = SyncLock::DAILY_LAMPORTS
                .checked_mul(days_not_synced_or_failed as u64)
                .ok_or(ScreenWarErrors::IntegerOverflow)?
                .checked_add(lb_penalty)
                .ok_or(ScreenWarErrors::IntegerOverflow)?;

            // slashed amounts are future rewards
            Self::update_total_slashed_in_challenge(challenge, total_penalty)?;
        }

        if user_passed_today {
            Self::increment_streak(user_pda)?;
            Self::update_users_locked_balance(user_pda, SyncLock::DAILY_LAMPORTS as i64)?;
            // increase
        }

        Ok(())
    }

    pub fn validate_challenge_has_started(now: i64, challenge_start: i64) -> ProgramResult {
        if now < challenge_start {
            return Err(ScreenWarErrors::ChallengeNotStarted.into());
        }
        Ok(())
    }

    pub fn validate_challenge_has_not_ended(now: i64, challenge_end: i64) -> ProgramResult {
        if now > challenge_end {
            return Err(ScreenWarErrors::ChallengeEnded.into());
        }
        Ok(())
    }

    pub fn validated_today_not_synced_already(synced_today: bool) -> ProgramResult {
        if synced_today {
            return Err(ScreenWarErrors::AlreadySynced.into());
        }
        Ok(())
    }

    pub fn deposit_total_daily_lamports(&mut self, days_to_update: u8) -> ProgramResult {
        let lamports = (days_to_update as u64)
            .checked_mul(Self::DAILY_LAMPORTS)
            .ok_or(ScreenWarErrors::IntegerOverflow)?;

        Transfer {
            from: self.accounts.user,
            to: self.accounts.global,
            lamports: lamports,
        }
        .invoke()?;

        Ok(())
    }

    // dev : Adjusts the user's locked balance by `amount`, which can be positive (credit) or negative (slash).
    pub fn update_users_locked_balance(user_pda: &mut User, amount: i64) -> ProgramResult {
        let new_balance = (user_pda.locked_balance as i64)
            .checked_add(amount)
            .ok_or(ScreenWarErrors::IntegerBoundsExceeded)?;

        user_pda.locked_balance = if new_balance >= 0 {
            new_balance as u64
        } else {
            -(new_balance) as u64
        };

        Ok(())
    }

    pub fn calculate_exponential_penalty_on_locked_balance(
        current_balance: u64,
        days_not_synced_or_failed: u8,
    ) -> Result<u64, ProgramError> {
        if current_balance == 0 {
            Ok(0)
        } else {
            const SCALE: u128 = 1_000_000;
            const RATE_75_PERCENT: u128 = 750_000;

            // Apply compounding: (RATE^days) / (SCALE^days)

            // RATE_75_PERCENT ^ days_not_synced_or_failed
            let numerator = RATE_75_PERCENT
                .checked_pow(days_not_synced_or_failed as u32)
                .ok_or(ScreenWarErrors::IntegerOverflow)?;

            // SCALE ^ days_not_synced_or_failed
            let denominator = SCALE
                .checked_pow(days_not_synced_or_failed as u32)
                .ok_or(ScreenWarErrors::IntegerOverflow)?;

            // numerator * SCALE / denominator
            let multiplier = numerator
                .checked_mul(SCALE)
                .ok_or(ScreenWarErrors::IntegerOverflow)?
                .checked_div(denominator)
                .ok_or(ScreenWarErrors::IntegerUnderflow)?; // bring back to 1x SCALE

            let balance_u128 = current_balance as u128;

            // balance_u128 * multiplier / SCALE
            let final_balance = balance_u128
                .checked_mul(multiplier)
                .ok_or(ScreenWarErrors::IntegerOverflow)?
                .checked_div(SCALE)
                .ok_or(ScreenWarErrors::IntegerUnderflow)?;

            // balance_u128 - final_balance
            let penalty = balance_u128
                .checked_sub(final_balance)
                .ok_or(ScreenWarErrors::IntegerUnderflow)?;

            Ok(penalty as u64) // => safe downcast, since unscaled amounts
        }
    }

    pub fn reset_streak(user_pda: &mut User) -> ProgramResult {
        user_pda.streak = 0;
        Ok(())
    }

    pub fn increment_streak(user_pda: &mut User) -> ProgramResult {
        user_pda.streak += 1;
        Ok(())
    }

    pub fn update_total_slashed_in_challenge(
        challenge: &mut Challenge,
        amount: u64,
    ) -> ProgramResult {
        challenge.total_slashed = challenge
            .total_slashed
            .checked_add(amount)
            .ok_or(ScreenWarErrors::IntegerOverflow)?;

        Ok(())
    }
}
