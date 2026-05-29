#![cfg(test)]

use crate::test_helpers::setup_contract;
use crate::{ContractError, Escrow, EscrowClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

/// BUG-020 (#165): initialize must reject admin == fee_collector so the two
/// roles are always backed by separate keys.
#[test]
fn test_initialize_same_admin_and_fee_collector_reverts() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);
    let shared = Address::generate(&env);

    let result = client.try_initialize(&shared, &shared, &0_u32);
    assert!(matches!(result, Err(Ok(ContractError::InvalidAddress))));
}

/// Sanity: distinct admin/fee_collector still initialize successfully.
#[test]
fn test_initialize_distinct_addresses_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (_contract_id, client, admin, fee_collector) = setup_contract(&env);

    let config = client.get_contract_config();
    assert_eq!(config.admin, admin);
    assert_eq!(config.fee_collector, fee_collector);
}

/// BUG-018 (#163): set_admin must reject a no-op rotation to the current admin.
#[test]
fn test_set_admin_same_address_reverts() {
    let env = Env::default();
    env.mock_all_auths();

    let (_contract_id, client, admin, _fee_collector) = setup_contract(&env);

    let result = client.try_set_admin(&admin);
    assert!(matches!(result, Err(Ok(ContractError::SameAddress))));
}

/// set_admin still succeeds when rotating to a genuinely different address.
#[test]
fn test_set_admin_new_address_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let (_contract_id, client, _admin, _fee_collector) = setup_contract(&env);
    let new_admin = Address::generate(&env);

    client.set_admin(&new_admin);
    assert_eq!(client.get_contract_config().admin, new_admin);
}
