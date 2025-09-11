use {
    crate::{state::Global, ScreenWarErrors},
    core::convert::TryFrom,
    pinocchio::{
        account_info::AccountInfo, program_error::ProgramError, pubkey::find_program_address,
        ProgramResult,
    },
};

pub struct ToggleChallengeCreation<'a> {
    pub accounts: ToggleChallengeCreationAccounts<'a>,
    pub instruction_data: ToggleChallengeCreationInstructionData,
}

pub struct ToggleChallengeCreationAccounts<'a> {
    pub admin: &'a AccountInfo,
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
        let accounts = ToggleChallengeCreationAccounts::try_from(accounts)?;
        let instruction_data = ToggleChallengeCreationInstructionData::try_from(instruction_data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> TryFrom<&'a [AccountInfo]> for ToggleChallengeCreationAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [admin, global] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // dev : later admin key is validated against global.admin in validate_admin() function
        if !admin.is_signer() {
            return Err(ScreenWarErrors::NotSigner)?;
        }

        let (global_pda_key, _) = find_program_address(&[b"global"], &crate::ID);
        if global.key().ne(&global_pda_key) {
            return Err(ProgramError::InvalidSeeds);
        };

        Ok(Self { admin, global })
    }
}

impl<'a> TryFrom<&'a [u8]> for ToggleChallengeCreationInstructionData {
    type Error = ProgramError;

    fn try_from(instruction_data: &'a [u8]) -> Result<Self, Self::Error> {
        if instruction_data.len().ne(&1usize) {
            return Err(ProgramError::InvalidInstructionData);
        }

        let pause = match instruction_data[0] {
            0 => false,
            1 => true,
            _ => return Err(ProgramError::InvalidInstructionData),
        };

        Ok(Self { pause })
    }
}

impl<'a> ToggleChallengeCreation<'a> {
    pub const DISCRIMINATOR: &'a u8 = &5;

    pub fn process(&mut self) -> ProgramResult {
        // get mutable ref to Global Pda
        let mut global_raw_data = self.accounts.global.try_borrow_mut_data()?;
        let global = Global::load_mut(&mut global_raw_data)?;

        // validate admin
        Self::validate_admin(global, self.accounts.admin)?;

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

    pub fn validate_admin(global: &mut Global, caller: &AccountInfo) -> ProgramResult {
        if global.admin.ne(caller.key()) {
            return Err(ScreenWarErrors::NotAdmin)?;
        };

        Ok(())
    }
}
