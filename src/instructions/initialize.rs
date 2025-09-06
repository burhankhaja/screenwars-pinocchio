use crate::state::Global;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::find_program_address,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};

use pinocchio_system::instructions::CreateAccount;

pub struct Initialize<'a> {
    pub accounts: InitializeAccounts<'a>,
}

pub struct InitializeAccounts<'a> {
    pub admin: &'a AccountInfo,
    pub global_pda: &'a AccountInfo,
    pub rent_sysvar: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub bump: u8,
}

impl<'a> TryFrom<&'a [AccountInfo]> for Initialize<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let accounts = InitializeAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

impl<'a> TryFrom<&'a [AccountInfo]> for InitializeAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [admin, global_pda, rent_sysvar, system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        let (global_pda_key, global_bump) = find_program_address(&[b"global"], &crate::ID);

        if global_pda_key.ne(global_pda.key()) {
            return Err(ProgramError::InvalidSeeds);
        }

        Ok(Self {
            admin,
            global_pda,
            rent_sysvar,
            system_program,
            bump: global_bump,
        })
    }
}

impl<'a> Initialize<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0;

    pub fn process(&mut self) -> ProgramResult {
        //// create global pda
        let global_space = Global::LEN;
        let global_rent = Rent::get()?.minimum_balance(global_space);

        let bump_binding = [self.accounts.bump]; // dev : lives till this function scope, otherwise gets dropped right at Seed::from([x]) due to limited scope
        let seeds = [Seed::from(b"global"), Seed::from(&bump_binding)];

        let pda_signer = Signer::from(&seeds);

        CreateAccount {
            from: self.accounts.admin,
            to: self.accounts.global_pda,
            lamports: global_rent,
            space: global_space as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&[pda_signer])?;

        //// initialize global pda data
        let mut global_pda_data = self.accounts.global_pda.try_borrow_mut_data()?; // dev-improvements : what if i take AccountInfo in load*() to abstract away try_borrow* logic

        let global: &mut Global = Global::load_mut(&mut global_pda_data)?;

        *global = Global {
            admin: *self.accounts.admin.key(),
            treasury: *self.accounts.global_pda.key(),
            challenge_ids: 1,
            bump: self.accounts.bump,
            ..Global::default()
        };

        Ok(())
    }
}
