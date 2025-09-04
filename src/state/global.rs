use core::mem::size_of;
use pinocchio::{program_error::ProgramError, pubkey::Pubkey};

#[repr(C)]
#[derive(Default, Debug)]
pub struct Global {
    pub admin: Pubkey,
    pub treasury: Pubkey,
    pub treasury_profits: u64,
    pub challenge_ids: u32,
    pub challenge_creation_paused: bool,
    pub bump: u8,
}

impl Global {
    pub const LEN: usize = size_of::<Pubkey>()
        + size_of::<Pubkey>()
        + size_of::<u64>()
        + size_of::<u32>()
        + size_of::<bool>()
        + size_of::<u8>();

    pub fn load_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
        // concise :  use core::mem::transmute; ==> Ok(unsafe { &mut *core::mem::transmute::<*mut u8, *mut Self>(bytes.as_mut_ptr()) })

        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        let ptr = bytes.as_mut_ptr() as *mut Self;
        let global = unsafe { &mut *ptr };

        Ok(global)
    }

    pub fn load(bytes: &[u8]) -> Result<&Self, ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        let ptr = bytes.as_ptr() as *const Self;
        let global = unsafe { &*ptr };

        Ok(global)
    }
}
