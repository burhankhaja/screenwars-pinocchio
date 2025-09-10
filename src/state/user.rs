use {
    crate::custom_errors::ScreenWarErrors,
    core::mem::size_of,
    pinocchio::{program_error::ProgramError, pubkey::Pubkey},
};

#[repr(C)]
#[derive(Default, Debug)]
pub struct User {
    pub user: Pubkey,
    pub challenge_id: u32,
    pub locked_balance: u64,
    pub streak: u8,
    pub bump: u8,
}

impl User {
    pub const LEN: usize = size_of::<Pubkey>()
        + size_of::<u32>()
        + size_of::<u64>()
        + size_of::<u8>()
        + size_of::<u8>();

    pub fn load_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if bytes.len().ne(&Self::LEN) {
            return Err(ScreenWarErrors::InvalidPdaDataLen.into());
        }
        let ptr = bytes.as_mut_ptr() as *mut Self;

        let user_pda = unsafe { &mut *ptr };

        Ok(user_pda)
    }

    pub fn load(bytes: &[u8]) -> Result<&Self, ProgramError> {
        if bytes.len().ne(&Self::LEN) {
            return Err(ScreenWarErrors::InvalidPdaDataLen.into());
        }
        let ptr = bytes.as_ptr() as *const Self;

        let user_pda = unsafe { &*ptr };

        Ok(user_pda)
    }
}
