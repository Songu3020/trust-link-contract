#![cfg(test)]

use crate::{Escrow, EscrowClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_get_contract_config() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, Escrow);
    let client = EscrowClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let fee_collector = Address::generate(&env);
    let arbitration_fee_bps = 500;

    client.initialize(&admin, &fee_collector, &arbitration_fee_bps);

    // Default fee_bps should be 0 initially
    let mut config = client.get_contract_config();
    assert_eq!(config.admin, admin);
    assert_eq!(config.fee_bps, 0);
    assert_eq!(config.fee_collector, fee_collector);
    assert_eq!(config.escrow_count, 0);

    // Update fee and check again
    client.set_protocol_fee(&admin, &150);
    config = client.get_contract_config();
    assert_eq!(config.fee_bps, 150);
    
    // Create an escrow to increment the counter
    let seller = Address::generate(&env);
    let resolver = Address::generate(&env);
    let token = Address::generate(&env);
    
    client.create_escrow(
        &seller,
        &resolver,
        &token,
        &1000_0000000,
        &100, // fee_bps
        &86400, // shipping_window
    );
    
    config = client.get_contract_config();
    assert_eq!(config.escrow_count, 1);

    // Rotate admin and check again
    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);
    config = client.get_contract_config();
    assert_eq!(config.admin, new_admin);
}
