mod test_helpers;
use {
    crate::test_helpers::{one_day, two_hours},
    screenwars_pinocchio::Challenge,
    test_helpers::{
        execute_create_challenge, execute_initialize, get_timestamp, set_timestamp,
        setup_escrow_test, Env, Global, Pubkey, Signer, SolanaKiteError,
    },
};

#[test]
pub fn test_initialize_screen_wars() -> Result<(), SolanaKiteError> {
    let mut env: Env = setup_escrow_test();

    let global_pda = execute_initialize(&mut env)?;

    // fetch global pda account
    let account_info = env.litesvm.get_account(&global_pda);
    let raw_data = account_info.unwrap().data;
    let decoded_data = Global::load(&raw_data).unwrap();

    // assertions
    assert!(Pubkey::from(decoded_data.admin) == env.admin.pubkey());
    assert!(Pubkey::from(decoded_data.treasury) == global_pda);
    assert!(decoded_data.challenge_ids == 1);
    assert!(decoded_data.treasury_profits == 0);
    assert!(!decoded_data.challenge_creation_paused);

    // dev : for debugging
    // println!("data : {:?}", decoded_data);

    Ok(())
}

#[test]
pub fn test_create_challenge() -> Result<(), SolanaKiteError> {
    let mut env: Env = setup_escrow_test();
    execute_initialize(&mut env)?;

    let now = get_timestamp(&env);
    let start_time = now + one_day;
    let daily_timer = two_hours - 1;

    let (global_pda, challenge_pda) =
        execute_create_challenge(&mut env, "jeff", 1, start_time, daily_timer)?;

    let global_raw_data = &env.litesvm.get_account(&global_pda).unwrap().data;
    let challenge_raw_data = env.litesvm.get_account(&challenge_pda).unwrap().data;

    let global: &Global = Global::load(&global_raw_data).unwrap();
    let challenge: &Challenge = Challenge::load(&challenge_raw_data).unwrap();

    assert!(
        global.challenge_ids == 2,
        "challenge ids must be incremented by 1 after creation"
    );

    assert!(
        challenge.challenge_id == 1,
        "first challenge's id must be equal to 1"
    );
    assert!(
        Pubkey::from(challenge.creator) == env.jeff.pubkey(),
        "since jeff created challenge he must be the nominated as creator"
    );
    // assert correct start
    // assert end after 21 days from start
    // assert winner and winner streak to be default values

    // dev : for debugging
    // println!("challenge PDA : {:?}", challenge);
    // println!("global PDA : {:?}", global);

    Ok(())
}
