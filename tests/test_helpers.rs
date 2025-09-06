pub use litesvm::LiteSVM;
pub use screenwars_pinocchio::{state::Global, ID};
pub use solana_instruction::{AccountMeta, Instruction};
pub use solana_keypair::Keypair;
pub use solana_kite::{get_pda_and_bump, send_transaction_from_instructions, SolanaKiteError};
pub use solana_program::{system_program::ID as SYSTEM_ID, sysvar::rent::ID as RENT_ID};
pub use solana_pubkey::Pubkey;
pub use solana_signer::Signer;

pub struct Env {
    pub litesvm: LiteSVM,
    pub program_id: Pubkey,
    pub admin: Keypair,
    pub jeff: Keypair,
    pub berg: Keypair,
    pub shrinath: Keypair,
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

    // return Env
    Env {
        litesvm,
        program_id,
        admin,
        jeff,
        berg,
        shrinath,
    }
}

pub fn get_program_id() -> Pubkey {
    Pubkey::from(ID) //pinocchio::crate::ID -> [u8; 32] --> LiteSVM's Pubkey type
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
