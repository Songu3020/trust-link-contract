#![cfg(test)]

use crate::{Escrow, EscrowCancelled, EscrowClient};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, IntoVal, Symbol, TryFromVal, Val, Vec};

fn setup_env() -> (Env, Address, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let resolver = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_collector = Address::generate(&env);

    (env, admin, seller, resolver, token, fee_collector)
}

fn has_cancel_event(env: &Env, contract_id: &Address, escrow_id: u64, seller: &Address) -> bool {
    let expected_topic = vec![&env, Symbol::new(env, "escrow_cancelled").into_val(env)];
    env.events().all().into_iter().any(|(event_contract, topics, data)| {
        event_contract == *contract_id
            && topics == expected_topic
            && EscrowCancelled::try_from_val(env, &data)
                .map(|event| event.escrow_id == escrow_id && &event.seller == seller)
                .unwrap_or(false)
    })
}

#[test]
fn test_escrow_ids_monotonic_and_unique() {
    let (env, admin, seller, resolver, token, fee_collector) = setup_env();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    client.initialize(&admin, &fee_collector, &0_u32);

    let mut ids = Vec::new(&env);
    for i in 1..=10 {
        let id = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
        assert_eq!(id, i as u64);
        ids.push_back(id);
    }

    // Verify persistence: new client instance sees counter at 11
    let client2 = EscrowClient::new(&env, &contract_id);
    let next_id = client2.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    assert_eq!(next_id, 11);
}

// Verify multiple sequential creation returns correct IDs
#[test]
fn test_escrow_ids_increment_sequentially() {
    let (env, admin, seller, resolver, token, fee_collector) = setup_env();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);
    client.initialize(&admin, &fee_collector, &0_u32);

    let id1 = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    let id2 = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    let id3 = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

// Verify that cancellation of an escrow does not reset the counter or cause duplicates
#[test]
fn test_cancelled_escrow_does_not_reset_counter() {
    let (env, admin, seller, resolver, token, fee_collector) = setup_env();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);
    client.initialize(&admin, &fee_collector, &0_u32);

    let id1 = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    let id2 = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    
    // Ensure cancellation of #1 doesn't reset counter to 1 or 2
    client.cancel_escrow(&id1);
    assert!(has_cancel_event(&env, &contract_id, id1, &seller));

    // Create a new escrow after cancellation
    let next_id = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    assert_eq!(next_id, 3);
}

// Verify that the IDs don't skip unexpectedly after cancellation
#[test]
fn test_escrow_counter_does_not_skip_after_cancellation() {
    let (env, admin, seller, resolver, token, fee_collector) = setup_env();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);
    client.initialize(&admin, &fee_collector, &0_u32);

    let id1 = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    let id2 = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    
    client.cancel_escrow(&id1);
    
    let id3 = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    let id4 = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
    assert_eq!(id4, 4);
    
    // Verify cancelled state is correctly kept in storage
    let cancelled_escrow = client.get_escrow(&id1);
    assert_eq!(cancelled_escrow.state, crate::EscrowState::Canceled);
}

// Verify multiple cancellations handle the indexing properly
#[test]
fn test_multiple_cancellations() {
    let (env, admin, seller, resolver, token, fee_collector) = setup_env();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);
    client.initialize(&admin, &fee_collector, &0_u32);

    let id1 = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    let id2 = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    let id3 = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    
    client.cancel_escrow(&id1);
    client.cancel_escrow(&id2);
    
    let next_id = client.create_escrow(&seller, &resolver, &token, &100_i128, &0_u32, &3600_u64);
    assert_eq!(next_id, 4);
}
