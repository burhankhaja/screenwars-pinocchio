use pinocchio::{pubkey::Pubkey};

#[repr(C)]
#[derive(Default, Debug)]
pub struct User {
    pub user: Pubkey,
    pub challenge_id: u32,
    pub locked_balance: u64,
    pub streak: u8,
    pub bump: u8,
}
