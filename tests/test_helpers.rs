pub use {
    litesvm::LiteSVM,
    screenwars_pinocchio::{Global, ID},
    solana_clock::Clock,
    solana_instruction::{AccountMeta, Instruction},
    solana_keypair::Keypair,
    solana_kite::{get_pda_and_bump, send_transaction_from_instructions, SolanaKiteError},
    solana_program::{system_program::ID as SYSTEM_ID, sysvar::rent::ID as RENT_ID},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
};

pub const JAN_2025: i64 = 1735689600;
pub const two_hours: i64 = 2 * (60 * 60);
pub const one_day: i64 = two_hours * 12;
pub const one_week: i64 = one_day * 7;
pub const three_weeks: i64 = one_week * 3;

pub struct Env {
    pub litesvm: LiteSVM,
    pub program_id: Pubkey,
    pub admin: Keypair,
    pub jeff: Keypair,
    pub berg: Keypair,
    pub shrinath: Keypair,
}

pub fn map_actor_from_id<'a>(env: *const Env, actor: &str) -> &'a Keypair {
    unsafe {
        match actor {
            "admin" => &(*env).admin,
            "jeff" => &(*env).jeff,
            "berg" => &(*env).berg,
            "shrinath" => &(*env).shrinath,
            _ => panic!("Invalid role id: {}", actor),
        }
    }
}

pub fn get_program_id() -> Pubkey {
    Pubkey::from(ID)
}

pub unsafe fn set_timestamp(env: *mut Env, timestamp: i64) {
    let clock = &mut (*env).litesvm.get_sysvar::<Clock>();
    clock.unix_timestamp = timestamp;
    (*env).litesvm.set_sysvar(clock);
}

pub fn get_timestamp(env: *const Env) -> i64 {
    unsafe {
        let clock = &(*env).litesvm.get_sysvar::<Clock>();
        clock.unix_timestamp
    }
}

pub fn setup_escrow_test() -> Env {
    let mut litesvm = LiteSVM::new();
    let program_id = get_program_id();

    litesvm
        .add_program_from_file(program_id, "target/deploy/screenwars_pinocchio.so")
        .unwrap();

    // generate keypairs
    let admin = Keypair::new();
    let jeff = Keypair::new();
    let berg = Keypair::new();
    let shrinath = Keypair::new();

    // fund keypairs
    litesvm.airdrop(&admin.pubkey(), 1_000_000_000).unwrap();
    litesvm.airdrop(&jeff.pubkey(), 1_000_000_000).unwrap();
    litesvm.airdrop(&berg.pubkey(), 1_000_000_000).unwrap();
    litesvm.airdrop(&shrinath.pubkey(), 1_000_000_000).unwrap();

    let mut env = Env {
        litesvm,
        program_id,
        admin,
        jeff,
        berg,
        shrinath,
    };

    // set initial timestamp to Jan 1 2025
    unsafe {
        set_timestamp(&mut env, JAN_2025);
    }

    // return Env
    env
}

pub struct InitializeAccounts {
    pub admin: Pubkey,
    pub global_pda: Pubkey,
    pub rent_sysvar: Pubkey,
    pub system_program: Pubkey,
}

pub fn build_initialize_accounts(admin: Pubkey, global_pda: Pubkey) -> InitializeAccounts {
    let rent_sysvar = Pubkey::from(RENT_ID.to_bytes());
    let system_program = Pubkey::from(SYSTEM_ID.to_bytes());

    InitializeAccounts {
        admin,
        global_pda,
        rent_sysvar,
        system_program,
    }
}

pub fn build_initialize_instruction(initialize_accounts: InitializeAccounts) -> Instruction {
    let program_id = get_program_id();
    let accounts = vec![
        AccountMeta::new(initialize_accounts.admin, true),
        AccountMeta::new(initialize_accounts.global_pda, false),
        AccountMeta::new_readonly(initialize_accounts.rent_sysvar, false),
        AccountMeta::new_readonly(initialize_accounts.system_program, false),
    ];

    let data = vec![0u8]; // initialize discriminator

    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub fn execute_initialize(env: &mut Env) -> Result<Pubkey, SolanaKiteError> {
    let (global_pda, _) = get_pda_and_bump(&[b"global".as_ref().into()], &env.program_id);
    let accounts = build_initialize_accounts(env.admin.pubkey(), global_pda);
    let instructions = build_initialize_instruction(accounts);

    send_transaction_from_instructions(
        &mut env.litesvm,
        vec![instructions],
        &[&env.admin],
        &env.admin.pubkey(),
    )?;

    Ok(global_pda)
}

#[derive(Clone, Copy)]
pub struct CreateChallengeAccounts {
    pub creator: Pubkey,
    pub global_pda: Pubkey,
    pub challenge_pda: Pubkey,
    pub rent_sysvar: Pubkey,
    pub system_program: Pubkey,
}

pub fn build_create_challenge_accounts(
    creator: Pubkey,
    challenge_id: u32,
) -> CreateChallengeAccounts {
    let program_id = get_program_id();
    let (global_pda, _) = get_pda_and_bump(&[b"global".as_ref().into()], &program_id);
    let (challenge_pda, _) = get_pda_and_bump(
        &[
            b"challenge".as_ref().into(),
            challenge_id.to_le_bytes().as_ref().into(),
        ],
        &program_id,
    );
    let rent_sysvar = Pubkey::from(RENT_ID.to_bytes());
    let system_program = Pubkey::from(SYSTEM_ID.to_bytes());

    CreateChallengeAccounts {
        creator,
        global_pda,
        challenge_pda,
        rent_sysvar,
        system_program,
    }
}

pub fn build_create_challenge_instruction(
    start_time: i64,
    daily_timer: i64,
    create_challenge_accounts: CreateChallengeAccounts,
) -> Instruction {
    let program_id = get_program_id();

    let accounts = vec![
        AccountMeta::new(create_challenge_accounts.creator, true),
        AccountMeta::new(create_challenge_accounts.global_pda, false),
        AccountMeta::new(create_challenge_accounts.challenge_pda, false),
        AccountMeta::new_readonly(create_challenge_accounts.rent_sysvar, false),
        AccountMeta::new_readonly(create_challenge_accounts.system_program, false),
    ];

    let mut data = vec![1u8]; // discriminator
    data.extend_from_slice(&start_time.to_le_bytes());
    data.extend_from_slice(&daily_timer.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub fn execute_create_challenge(
    env: &mut Env,
    creator_actor: &str,
    challenge_id: u32,
    start_time: i64,
    daily_timer: i64,
) -> Result<(Pubkey, Pubkey), SolanaKiteError> {
    let creator = map_actor_from_id(&*env, creator_actor);
    let creator_key = creator.pubkey();
    let accounts = build_create_challenge_accounts(creator_key, challenge_id);
    let instructions = build_create_challenge_instruction(start_time, daily_timer, accounts);

    send_transaction_from_instructions(
        &mut env.litesvm,
        vec![instructions],
        &[creator],
        &creator_key,
    )?;

    Ok((accounts.global_pda, accounts.challenge_pda))
}
