#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, vec, Address, Env, Map, String,
    Symbol, Vec,
};

pub const POLICY_COUNTER: Symbol = symbol_short!("policy");
pub const POLICY_STORAGE_KEY: Symbol = symbol_short!("policykey");
pub const CLAIM_COUNTER: Symbol = symbol_short!("claim");
pub const CLAIM_STORAGE_KEY: Symbol = symbol_short!("claimkey");
pub const ADMIN: Symbol = symbol_short!("admin");
pub const BASE_RATE: f32 = 0.1;
pub const TOKEN_KEY: Symbol = symbol_short!("token");

#[derive(Clone)]
#[contracttype]
pub struct Policy {
    pub id: u64,
    pub policy_holder: Address,
    pub premium: i128,
    pub coverage_amount: i128,
    pub start_date: u64,
    pub end_date: u64,
    pub is_active: bool,
    pub is_claimed: bool,
    pub clause: String,
}

#[derive(Clone)]
#[contracttype]
pub struct Claim {
    pub policy_id: u64,
    pub policy_holder: Address,
    pub claim_amount: i128,
    pub reason: String,
    pub is_approved: bool,
}

#[contract]
pub struct Insurance;

#[contractimpl]
impl Insurance {
    pub fn initialize(env: Env, token_address: Address, admin: Address) {
        env.storage().instance().set(&TOKEN_KEY, &token_address);
        env.storage().instance().set(&ADMIN, &admin);
    }

    pub fn calculate_premium(coverage_amount: i128, duration_in_days: u64) -> i128 {
        assert!(coverage_amount > 0);
        assert!(duration_in_days > 0);
        let premium = (coverage_amount as f32 * BASE_RATE) * (duration_in_days as f32 / 365.0);
        let final_premium = premium.min(coverage_amount as f32);
        final_premium.ceil() as i128
    }

    pub fn create_policy(
        env: Env,
        policy_holder: Address,
        coverage_amount: i128,
        duration_in_days: u64,
        clause: String,
    ) {
        let premium: i128 = Self::calculate_premium(coverage_amount, duration_in_days);

        let mut policy_count = env.storage().instance().get(&POLICY_COUNTER).unwrap_or(0);
        let start_time = env.ledger().timestamp();
        let end_time = start_time + duration_in_days * 24 * 60 * 60; // Convert days to seconds
        policy_holder.require_auth();
        let policy = Policy {
            id: policy_count,
            policy_holder: policy_holder.clone(),
            premium,
            coverage_amount,
            start_date: start_time,
            end_date: end_time,
            is_active: true,
            is_claimed: false,
            clause: clause,
        };
        let mut policies: Map<u64, Policy> =
            env.storage().instance().get(&POLICY_STORAGE_KEY).unwrap();

        policies.set(policy.id, policy);
        env.storage().instance().set(&POLICY_STORAGE_KEY, &policies);
        policy_count += 1;
        env.storage().instance().set(&POLICY_COUNTER, &policy_count);
        let token = env.storage().instance().get(&TOKEN_KEY).unwrap();
        token::Client::new(&env, &token).transfer(
            &policy_holder,
            &env.current_contract_address(),
            &premium,
        );

        env.events()
            .publish((POLICY_STORAGE_KEY, symbol_short!("policy")), policies);
    }

    pub fn submit_claim(env: Env, policy_id: u64, claim_amount: i128, reason: String) {
        let data = env.storage().instance().get::<_, Policy>(&policy_id);

        let policy = match data {
            Some(result) => result,
            None => panic!("policy Id does not exist!"),
        };

        assert!(policy.is_active == true);
        assert!(policy.is_claimed == false);
        assert!(
            env.ledger().timestamp() >= policy.start_date
                && env.ledger().timestamp() <= policy.end_date
        );
        assert!(claim_amount <= policy.coverage_amount);
        policy.policy_holder.require_auth();
        let mut claim_count = env.storage().instance().get(&CLAIM_COUNTER).unwrap_or(0);
        let claim = Claim {
            policy_id: policy.id,
            policy_holder: policy.policy_holder.clone(),
            claim_amount,
            reason,
            is_approved: false,
        };

        let mut claim_data: Map<u64, Claim> =
            env.storage().instance().get(&CLAIM_STORAGE_KEY).unwrap();
        claim_data.set(claim_count, claim);
        claim_count += 1;
        env.storage()
            .instance()
            .set(&CLAIM_STORAGE_KEY, &claim_data);
        env.storage().instance().set(&CLAIM_COUNTER, &claim_count);
    }

    pub fn approve_claim(env: Env, claim_id: u64) {
        let admin_address = env
            .storage()
            .instance()
            .get::<_, Address>(&ADMIN)
            .expect("No admin address found");
        admin_address.require_auth();

        let data = env.storage().instance().get::<_, Claim>(&claim_id);

        let mut claim = match data {
            Some(result) => result,
            None => panic!("Claim Id does not exist!"),
        };
        assert!(claim.is_approved == false, "claim already claimed");

        let policy = env.storage().instance().get::<_, Policy>(&claim.policy_id);
        let mut policy_data = match policy {
            Some(result) => result,
            None => panic!("Policy Id does not exist!"),
        };
        assert!(policy_data.is_claimed == false, "Policy already claimed");
        assert!(policy_data.is_active == true, "Policy is not active");

        policy_data.is_claimed = true;
        policy_data.is_active = false;
        claim.is_approved = true;

        env.storage().instance().set(&CLAIM_STORAGE_KEY, &claim);
        env.storage()
            .instance()
            .set(&POLICY_STORAGE_KEY, &policy_data);
        let token = env.storage().instance().get(&TOKEN_KEY).unwrap();

        token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &policy_data.policy_holder,
            &claim.claim_amount,
        );
    }

    // withdraw unclaimed....

    pub fn hello(env: Env, to: String) -> Vec<String> {
        vec![&env, String::from_str(&env, "Hello"), to]
    }
}

mod test;
