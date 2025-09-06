mod test_helpers;
use test_helpers::{
    execute_initialize, setup_escrow_test, Env, Global, Pubkey, Signer, SolanaKiteError,
};

#[test]
pub fn test_initialize_screen_wars() -> Result<(), SolanaKiteError> {
    let mut env: Env = setup_escrow_test();

    let global_pda = execute_initialize(&mut env)?;

    // fetch global pda account
    let account_info = env.litesvm.get_account(&global_pda);
    let raw_data = account_info.unwrap().data;
    let decoded_data = Global::load(&raw_data).unwrap();

    // log fetched data
    println!("data : {:?}", decoded_data);

    // assertions
    assert!(Pubkey::from(decoded_data.admin) == env.admin.pubkey());
    assert!(Pubkey::from(decoded_data.treasury) == global_pda);
    assert!(decoded_data.challenge_ids == 1);
    assert!(decoded_data.treasury_profits == 0);
    assert!(!decoded_data.challenge_creation_paused);

    Ok(())
}
