use {
    crate::{state::Global, ScreenWarErrors},
    pinocchio::{
        account_info::AccountInfo, program_error::ProgramError,  ProgramResult,
        instruction::{Seed, Signer}
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
        todo!();
    }
}

//@audit-issue :: add Admin Signer validations otherwise anyone can get unauthorized access
impl<'a> TryFrom<&'a [AccountInfo]> for TakeProfitAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        //@audit-issue:: validate Global Pda ?

        todo!();
    }
}

impl<'a> TryFrom<&'a [u8]> for TakeProfitInstructionData {
    type Error = ProgramError;

    fn try_from(instruction_data: &'a [u8]) -> Result<Self, Self::Error> {
        todo!();
    }
}

impl<'a> TakeProfit<'a> {
    pub const DISCRIMINATOR: &'a u8 = &6;

    pub fn process(&mut self) -> ProgramResult {
        // get mutable ref to Global Pda
        let mut global_raw_data = self.accounts.global.try_borrow_mut_data()?;
        let global = Global::load_mut(&mut global_raw_data)?;

        // validate protocol is solvent to payoff all user funds + rewards
        Self::validate_solvency(global.treasury_profits, self.instruction_data.amount)?;

        // transfer
        Self::withdraw_from_treasury(self.accounts.global, self.accounts.admin, self.instruction_data.amount, self.accounts.global_bump)?;

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

    pub fn withdraw_from_treasury(global: &AccountInfo, admin: &AccountInfo, amount: u64, global_bump: u8) -> ProgramResult {
      
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
        global.treasury_profits = global.treasury_profits.checked_sub(amount).ok_or(ProgramError::ArithmeticOverflow)?; 

        Ok(())
    }



}
