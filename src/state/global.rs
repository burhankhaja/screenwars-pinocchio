use pinocchio::{pubkey::Pubkey};

#[repr(C)]
#[derive(Default, Debug)]
pub struct Global {
    pub admin : Pubkey,
    pub treasury : Pubkey,
    pub treasury_profits : u64,
    pub challenge_ids: u32,
    pub challenge_creation_paused: bool,
    pub bump: u8,
}