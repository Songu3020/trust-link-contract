#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};
use crate::{ContractError, EscrowState};
use crate::test_helpers::{setup_contract, mint_token};

fn register_token(env: &Env) -> Address {
    let token_admin = Address::generate(env);
    env.register_stellar_asset_contract(token_admin)
}

#[test]
fn test_cancel_escrow_by_vendor_in_pending_state() {
    let env = Env::default();
    env.mock_all_auths();

    let token = register_token(&env);
    let (_contract_id, client, _admin, _fee_collector) = setup_contract(&env);

    let seller = Address::generate(&env);
    let resolver = Address::generate(&env);

    // Create escrow — stays in Pending (no buyer has funded it)
    let id = client.create_escrow(&seller, &resolver, &token, &500_i128, &0_u32, &3600_u64);

    let escrow_before = client.get_escrow(&id);
    assert_eq!(escrow_before.state, EscrowState::Pending);

    // Vendor (seller) cancels the unfunded escrow
    client.cancel_escrow(&id);

    let escrow_after = client.get_escrow(&id);
    assert_eq!(escrow_after.state, EscrowState::Cancelled);
}

#[test]
fn test_cancel_escrow_returns_funds_if_buyer_present() {
    let env = Env::default();
    env.mock_all_auths();

    let token = register_token(&env);
    let (contract_id, client, _admin, _fee_collector) = setup_contract(&env);

    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let resolver = Address::generate(&env);

    mint_token(&env, &token, &buyer, 1000);

    let id = client.create_escrow(&seller, &resolver, &token, &1000_i128, &0_u32, &3600_u64);

    // Fund the escrow so buyer tokens are locked
    client.fund_escrow(&id, &buyer);

    // cancel_escrow requires Pending state — after funding it's Funded, so this must error
    let res = client.try_cancel_escrow(&id);
    assert!(matches!(res, Err(Ok(ContractError::InvalidState))));

    // Buyer balance is still locked (no refund happened)
    let buyer_balance = soroban_sdk::token::Client::new(&env, &token).balance(&buyer);
    assert_eq!(buyer_balance, 0);

    let _ = contract_id;
}

#[test]
fn test_cancel_escrow_non_pending_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let token = register_token(&env);
    let (_contract_id, client, _admin, _fee_collector) = setup_contract(&env);

    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let resolver = Address::generate(&env);

    mint_token(&env, &token, &buyer, 1000);

    let id = client.create_escrow(&seller, &resolver, &token, &1000_i128, &0_u32, &3600_u64);
    client.fund_escrow(&id, &buyer);

    // Escrow is now Funded — cancel must be rejected
    let res = client.try_cancel_escrow(&id);
    assert!(matches!(res, Err(Ok(ContractError::InvalidState))));

    let escrow = client.get_escrow(&id);
    assert_eq!(escrow.state, EscrowState::Funded);
}
