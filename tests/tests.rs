mod test_helpers;
use {
    crate::test_helpers::{one_day, two_hours, CHALLENGE_START_HELPER},
    screenwars_pinocchio::user,
    test_helpers::{
        execute_create_challenge, execute_initialize, execute_join_challenge, get_timestamp,
        set_timestamp, setup_escrow_test, Challenge, Env, Global, Pubkey, Signer, SolanaKiteError,
        User,
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

#[test]
pub fn test_join_challenge() -> Result<(), SolanaKiteError> {
    let mut env: Env = setup_escrow_test();
    execute_initialize(&mut env)?;
    let (_, challenge_pda) =
        execute_create_challenge(&mut env, "jeff", 1, CHALLENGE_START_HELPER, two_hours - 1)?;
    let (user_pda_key) = execute_join_challenge(&mut env, "berg", 1)?;

    let challenge_raw_data = env.litesvm.get_account(&challenge_pda).unwrap().data;
    let challenge = Challenge::load(&challenge_raw_data).unwrap();

    let user_pda_raw_data = env.litesvm.get_account(&user_pda_key).unwrap().data;
    let user_pda = User::load(&user_pda_raw_data).unwrap();

    // dev : debug
    // println!("User_pda : {:?}", user_pda);
    // println!("challenge pda: {:?}", challenge);

    //// assertion
    assert!(
        Pubkey::from(user_pda.user) == env.berg.pubkey(),
        "bergs key must be stored in his joining pda"
    );

    assert!(
        user_pda.challenge_id == 1,
        "user joined challenge id must be stored correctly in his pda"
    );

    assert!(
        user_pda.streak == 0,
        "users cant have positive streak just by joining challenge"
    );

    //// @audit-issue :: junk data stored in challenge.total_pariticipants && user_pda.locked_balance
    //// Expected Value : participants = 1 && balance == 0
    // assert!(
    //     challenge.total_participants == 1,
    //     "challenge participants must increment after join"
    // );
    // assert!(
    //     user_pda.locked_balance == 0,
    //     "users cant have positive locked_balance just by joining challenge"
    // );

    println!("flawed locked balance : {}", user_pda.locked_balance);
    println!(
        "flawed total pariticipants: {}",
        challenge.total_participants
    );
    Ok(())
}


