use {
    crate::{
        state::{Challenge, User},
        ScreenWarErrors,
    },
    pinocchio::{
        account_info::AccountInfo,
        program_error::ProgramError,
        pubkey::{find_program_address, Pubkey},
        sysvars::{clock::Clock, Sysvar},
        ProgramResult,
    },
};

pub struct ClaimWinnerPosition<'a> {
    pub accounts: ClaimWinnerPositionAccounts<'a>,
    pub instruction_data: ClaimWinnerPositionInstructionData,
}

pub struct ClaimWinnerPositionAccounts<'a> {
    pub user: &'a AccountInfo,
    pub challenge: &'a AccountInfo,
    pub user_pda: &'a AccountInfo,
    pub clock_sysvar: &'a AccountInfo,
}

pub struct ClaimWinnerPositionInstructionData {
    pub challenge_id: u32,
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for ClaimWinnerPosition<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, instruction_data): (&'a [AccountInfo], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = ClaimWinnerPositionAccounts::try_from(accounts)?;
        let instruction_data = ClaimWinnerPositionInstructionData::try_from(instruction_data)?;

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

impl<'a> TryFrom<&'a [AccountInfo]> for ClaimWinnerPositionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [user, challenge, user_pda, clock_sysvar] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !user.is_signer() {
            return Err(ScreenWarErrors::NotSigner.into());
        }

        let (user_pda_key, _) = find_program_address(&[b"user", user.key().as_slice()], &crate::ID);

        if (&user_pda_key).ne(user_pda.key()) {
            return Err(ScreenWarErrors::InvalidUserPDA.into());
        };

        Ok(Self {
            user,
            challenge,
            user_pda,
            clock_sysvar,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for ClaimWinnerPositionInstructionData {
    type Error = ProgramError;

    fn try_from(instruction_data: &'a [u8]) -> Result<Self, Self::Error> {
        if instruction_data.len().ne(&4usize) {
            return Err(ProgramError::InvalidInstructionData);
        }

        let challenge_id = u32::from_le_bytes(instruction_data.try_into().unwrap());

        Ok(Self { challenge_id })
    }
}

impl<'a> ClaimWinnerPosition<'a> {
    pub const DISCRIMINATOR: &'a u8 = &4;

    pub fn process(&mut self) -> ProgramResult {
        // get the mutable reference to challenge and simple reference to User pdas
        let mut challenge_raw_data = self.accounts.challenge.try_borrow_mut_data()?;
        let challenge = Challenge::load_mut(&mut challenge_raw_data)?;

        let user_pda_raw_data = self.accounts.user_pda.try_borrow_data()?;
        let user_pda = User::load(&user_pda_raw_data)?;

        // validations
        let now = Clock::get()?.unix_timestamp;
        Self::validate_challenge_has_ended(now, challenge.end)?;
        Self::validate_reward_claiming_has_not_started(now, challenge.end)?;
        Self::validate_user_is_enrolled_in_challenge(
            challenge.challenge_id,
            user_pda.challenge_id,
        )?;

        // set winner
        if challenge.winner == Pubkey::default() {
            Self::write_winner(challenge, user_pda)?;
        } else {
            if challenge.winner_streak > user_pda.streak {
                return Err(ScreenWarErrors::LowerStreak.into());
            }

            Self::write_winner(challenge, user_pda)?;
        }
        Ok(())
    }

    pub fn validate_challenge_has_ended(now: i64, end: i64) -> ProgramResult {
        if end > now {
            return Err(ScreenWarErrors::ChallengeNotEnded.into());
        };

        Ok(())
    }

    pub fn validate_reward_claiming_has_not_started(now: i64, end: i64) -> ProgramResult {
        let five_days = 5 * 24 * 60 * 60;

        if now > end + five_days {
            return Err(ScreenWarErrors::ContentionExpired.into());
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

    pub fn write_winner(challenge: &mut Challenge, user_pda: &User) -> ProgramResult {
        challenge.winner = user_pda.user;
        challenge.winner_streak = user_pda.streak;

        Ok(())
    }
}
