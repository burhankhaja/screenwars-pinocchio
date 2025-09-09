use {
    core::mem::size_of,
    pinocchio::{program_error::ProgramError, pubkey::Pubkey},
};

#[repr(C)]
#[derive(Default, Debug)]
pub struct Challenge {
    pub creator: Pubkey,
    pub challenge_id: u32,
    pub daily_timer: i64,
    pub start: i64,
    pub end: i64,
    pub total_slashed: u64,
    pub winner: Pubkey,
    pub winner_streak: u8,
    pub winner_has_claimed: bool,
    pub creator_has_claimed: bool,
    pub total_participants: u32,
    pub bump: u8,
}

impl Challenge {
    pub const LEN: usize = size_of::<Pubkey>()
        + size_of::<u32>()
        + size_of::<i64>()
        + size_of::<i64>()
        + size_of::<i64>()
        + size_of::<u64>()
        + size_of::<Pubkey>()
        + size_of::<u8>()
        + size_of::<bool>()
        + size_of::<bool>()
        + size_of::<u32>()
        + size_of::<u8>();

    pub fn load_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if bytes.len().ne(&Self::LEN) {
            return Err(ProgramError::InvalidAccountData);
        };

        let ptr = bytes.as_mut_ptr() as *mut Self; //dev : coersion of (as_mut_ptr)  to (bytes as *mut [u8] as *mut u8) , since bytes itself is fat pointer and contains lenght part too
        let challenge = unsafe { &mut *ptr };

        Ok(challenge)
    }

    pub fn load(bytes: &[u8]) -> Result<&Self, ProgramError> {
        if bytes.len().ne(&Self::LEN) {
            return Err(ProgramError::InvalidAccountData);
        };

        let ptr = bytes.as_ptr() as *const Self;
        let challenge = unsafe { &*ptr };

        Ok(challenge)
    }
}
