use pinocchio::{pubkey::Pubkey};

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
