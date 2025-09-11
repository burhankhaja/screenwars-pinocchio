use {
    crate::{
        state::{Challenge, Global},
        ScreenWarErrors,
    },
    pinocchio::{
        account_info::AccountInfo,
        instruction::{Seed, Signer},
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvars::{clock::Clock, Sysvar},
        ProgramResult,
    },
    pinocchio_system::instructions::Transfer,
};

pub struct ClaimRewards<'a> {
    pub accounts: ClaimRewardsAccounts<'a>,
    pub instruction_data: ClaimRewardsInstructionData,
}

pub struct ClaimRewardsAccounts<'a> {
    pub user: &'a AccountInfo,
    pub challenge: &'a AccountInfo,
    pub global: &'a AccountInfo,
    pub clock_sysvar: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub global_bump: u8,
}

pub struct ClaimRewardsInstructionData {
    pub amount: u64,
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for ClaimRewards<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, instruction_data): (&'a [AccountInfo], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        todo!();
    }
}

impl<'a> TryFrom<&'a [AccountInfo]> for ClaimRewardsAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        //@audit-issue:: Dont forget to validate challenge & global Pdas
        // dev: Add Signer validations in ClaimRewards Impl!

        todo!();
    }
}

impl<'a> TryFrom<&'a [u8]> for ClaimRewardsInstructionData {
    type Error = ProgramError;

    fn try_from(instruction_data: &'a [u8]) -> Result<Self, Self::Error> {
        todo!();
    }
}

impl<'a> ClaimRewards<'a> {
    pub const WINNER_REWARD_DISCRIMINATOR: &'a u8 = &7;
    pub const CREATOR_REWARD_DISCRIMINATOR: &'a u8 = &8;

    pub fn process_winner_rewards(&mut self) -> ProgramResult {
        // get mutable references to global and challenge pdas
        let mut challenge_raw_data = self.accounts.challenge.try_borrow_mut_data()?;
        let mut challenge = Challenge::load_mut(&mut challenge_raw_data)?;

        let mut global_raw_data = self.accounts.global.try_borrow_mut_data()?;
        let global = Global::load_mut(&mut global_raw_data)?;

        Self::validate_caller_is_winner(self.accounts.user, challenge.winner)?;
        Self::validate_contention_period_is_over(challenge.end)?;

        let (winner_rewards, _, treasury_profits) =
            Self::calculate_rewards(challenge.total_slashed)?;

        let claimed_by_creator = challenge.creator_has_claimed;
        if claimed_by_creator {
            Self::close_challenge_account(&mut challenge)?;
        } else {
            Self::update_treasury_profits(global, treasury_profits)?;
        }

        // dev : winner state is nullified with default pubkey after claiming to prevent fund draining
        Self::set_winner_claimed(challenge)?;
        Self::transfer_rewards(
            self.accounts.global,
            self.accounts.user,
            winner_rewards,
            self.accounts.global_bump,
        )?;
        Ok(())
    }

    pub fn process_creator_rewards(&mut self) -> ProgramResult {
        // get mutable references to global and challenge pdas
        let mut challenge_raw_data = self.accounts.challenge.try_borrow_mut_data()?;
        let mut challenge = Challenge::load_mut(&mut challenge_raw_data)?;

        let mut global_raw_data = self.accounts.global.try_borrow_mut_data()?;
        let global = Global::load_mut(&mut global_raw_data)?;

        Self::validate_caller_is_creator(self.accounts.user, challenge.creator)?;
        Self::validate_contention_period_is_over(challenge.end)?;

        let (_, creator_rewards, treasury_profits) =
            Self::calculate_rewards(challenge.total_slashed)?;

        let claimed_by_winner = challenge.winner_has_claimed;
        if claimed_by_winner {
            Self::close_challenge_account(&mut challenge)?;
        } else {
            Self::update_treasury_profits(global, treasury_profits)?;
        }

        // dev : creator state is nullified with default pubkey after claiming to prevent fund draining
        Self::set_creator_claimed(challenge)?;
        Self::transfer_rewards(
            self.accounts.global,
            self.accounts.user,
            creator_rewards,
            self.accounts.global_bump,
        )?;
        Ok(())
    }

    pub fn validate_caller_is_winner(caller: &AccountInfo, winner: Pubkey) -> ProgramResult {
        if caller.key().ne(&winner) {
            return Err(ScreenWarErrors::NotWinner.into());
        }

        Ok(())
    }

    pub fn validate_caller_is_creator(caller: &AccountInfo, creator: Pubkey) -> ProgramResult {
        if caller.key().ne(&creator) {
            return Err(ScreenWarErrors::NotCreator.into());
        }

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

    pub fn transfer_rewards(
        global: &AccountInfo,
        user: &AccountInfo,
        rewards: u64,
        global_bump: u8,
    ) -> ProgramResult {
        if rewards > 0 {
            let global_bump_binding = [global_bump];
            let seeds = &[Seed::from(b"global"), Seed::from(&global_bump_binding)];
            let global_pda_signature = Signer::from(seeds);

            Transfer {
                from: global,
                to: user,
                lamports: rewards,
            }
            .invoke_signed(&[global_pda_signature])?;
        }

        Ok(())
    }

    // set winner claimed
    pub fn set_winner_claimed(challenge: &mut Challenge) -> ProgramResult {
        challenge.winner_has_claimed = true;
        challenge.winner = Pubkey::default();
        Ok(())
    }
    // set creator claimed
    pub fn set_creator_claimed(challenge: &mut Challenge) -> ProgramResult {
        challenge.creator_has_claimed = true;
        challenge.creator = Pubkey::default();
        Ok(())
    }

    // dev : scaling logic reference : https://github.com/burhankhaja/screen_wars/blob/main/programs/screen_wars/src/instructions/claim_rewards.rs#L83-L106
    pub fn calculate_rewards(total_slashed: u64) -> Result<(u64, u64, u64), ProgramError> {
        const SCALE: u128 = 1_000_000;
        const SCALED_50_PERCENT: u128 = 500_000;
        const SCALED_10_PERCENT: u128 = 100_000;

        let total_slashed = total_slashed as u128;

        // (total_slashed * ((SCALED_50_PERCENT * SCALE) / SCALE)) / SCALE;
        let winner_rewards: u128 = SCALED_50_PERCENT
            .checked_mul(SCALE)
            .ok_or(ScreenWarErrors::IntegerOverflow)?
            .checked_div(SCALE)
            .ok_or(ScreenWarErrors::IntegerUnderflow)?
            .checked_mul(total_slashed)
            .ok_or(ScreenWarErrors::IntegerOverflow)?
            .checked_div(SCALE)
            .ok_or(ScreenWarErrors::IntegerUnderflow)?;

        // (total_slashed * ((SCALED_10_PERCENT * SCALE) / SCALE)) / SCALE;
        let creator_reward: u128 = SCALED_10_PERCENT
            .checked_mul(SCALE)
            .ok_or(ScreenWarErrors::IntegerOverflow)?
            .checked_div(SCALE)
            .ok_or(ScreenWarErrors::IntegerUnderflow)?
            .checked_mul(total_slashed)
            .ok_or(ScreenWarErrors::IntegerOverflow)?
            .checked_div(SCALE)
            .ok_or(ScreenWarErrors::IntegerUnderflow)?;

        let non_protocol_rewards = winner_rewards
            .checked_add(creator_reward)
            .ok_or(ScreenWarErrors::IntegerOverflow)?;

        let protocol_profits = total_slashed
            .checked_sub(non_protocol_rewards)
            .ok_or(ScreenWarErrors::IntegerUnderflow)?;

        Ok((
            winner_rewards as u64,
            creator_reward as u64,
            protocol_profits as u64,
        ))
    }

    // close_challenge_account -> to_do!()
    pub fn close_challenge_account(challenge: &mut Challenge) -> ProgramResult {
        todo!();
    }

    // update treasury profits
    pub fn update_treasury_profits(global: &mut Global, amount: u64) -> ProgramResult {
        global.treasury_profits = global
            .treasury_profits
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }
}
