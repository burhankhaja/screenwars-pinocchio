use {
    crate::{state::Global, ScreenWarErrors},
    pinocchio::{
        account_info::AccountInfo,
        instruction::{Seed, Signer},
        program_error::ProgramError,
        pubkey::find_program_address,
        ProgramResult,
    },
    pinocchio_system::instructions::Transfer,
};

pub struct TakeProfit<'a> {
    pub accounts: TakeProfitAccounts<'a>,
    pub instruction_data: TakeProfitInstructionData,
}

pub struct TakeProfitAccounts<'a> {
    pub admin: &'a AccountInfo,
    pub global: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub global_bump: u8,
}

pub struct TakeProfitInstructionData {
    pub amount: u64,
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for TakeProfit<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, instruction_data): (&'a [AccountInfo], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = TakeProfitAccounts::try_from(accounts)?;
        let instruction_data = TakeProfitInstructionData::try_from(instruction_data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> TryFrom<&'a [AccountInfo]> for TakeProfitAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [admin, global, system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // dev : later admin key is validated against global.admin in validate_admin() function
        if !admin.is_signer() {
            return Err(ScreenWarErrors::NotSigner)?;
        }

        let (global_pda_key, global_bump) = find_program_address(&[b"global"], &crate::ID);
        if global.key().ne(&global_pda_key) {
            return Err(ProgramError::InvalidSeeds);
        };

        Ok(Self {
            admin,
            global,
            system_program,
            global_bump,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for TakeProfitInstructionData {
    type Error = ProgramError;

    fn try_from(instruction_data: &'a [u8]) -> Result<Self, Self::Error> {
        if instruction_data.len().ne(&8usize) {
            return Err(ProgramError::InvalidInstructionData);
        }

        let amount = u64::from_le_bytes(instruction_data.try_into().unwrap());

        Ok(Self { amount })
    }
}

impl<'a> TakeProfit<'a> {
    pub const DISCRIMINATOR: &'a u8 = &6;

    pub fn process(&mut self) -> ProgramResult {
        // get mutable ref to Global Pda
        let mut global_raw_data = self.accounts.global.try_borrow_mut_data()?;
        let global = Global::load_mut(&mut global_raw_data)?;

        // validate admin
        Self::validate_admin(global, self.accounts.admin)?;

        // validate protocol is solvent to payoff all user funds + rewards
        Self::validate_solvency(global.treasury_profits, self.instruction_data.amount)?;

        // transfer
        Self::withdraw_from_treasury(
            self.accounts.global,
            self.accounts.admin,
            self.instruction_data.amount,
            self.accounts.global_bump,
        )?;

        // decrease global profits
        Self::update_treasury_profits(global, self.instruction_data.amount)?;

        Ok(())
    }

    pub fn validate_solvency(treasury_profits: u64, amount: u64) -> ProgramResult {
        if amount > treasury_profits {
            return Err(ScreenWarErrors::OverClaim.into());
        }
        Ok(())
    }

    pub fn validate_admin(global: &mut Global, caller: &AccountInfo) -> ProgramResult {
        if global.admin.ne(caller.key()) {
            return Err(ScreenWarErrors::NotAdmin)?;
        };

        Ok(())
    }

    pub fn withdraw_from_treasury(
        global: &AccountInfo,
        admin: &AccountInfo,
        amount: u64,
        global_bump: u8,
    ) -> ProgramResult {
        if amount > 0 {
            let global_bump_binding = [global_bump];
            let seeds = &[Seed::from(b"global"), Seed::from(&global_bump_binding)];
            let global_pda_signature = Signer::from(seeds);

            Transfer {
                from: global,
                to: admin,
                lamports: amount,
            }
            .invoke_signed(&[global_pda_signature])?;
        }

        Ok(())
    }

    pub fn update_treasury_profits(global: &mut Global, amount: u64) -> ProgramResult {
        global.treasury_profits = global
            .treasury_profits
            .checked_sub(amount)
            .ok_or(ScreenWarErrors::IntegerUnderflow)?;

        Ok(())
    }
}
