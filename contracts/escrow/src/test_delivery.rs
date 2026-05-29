#![cfg(test)]

use crate::test_helpers::{advance_time, create_funded_escrow, setup_contract};
use crate::{ContractError, DeliveryRecorded, EscrowState};
use soroban_sdk::{
    testutils::Address as _, vec, Address, Env, IntoVal, String as SorobanString, Symbol,
    TryFromVal, Val,
};

fn register_token(env: &Env) -> Address {
    let token_admin = Address::generate(env);
    env.register_stellar_asset_contract(token_admin)
}

fn has_event<T, F>(env: &Env, contract_id: &Address, topic: &str, predicate: F) -> bool
where
    T: TryFromVal<Env, Val>,
    F: Fn(&T) -> bool,
{
    let expected_topic = vec![&env, Symbol::new(env, topic).into_val(env)];
    env.events().all().into_iter().any(|(event_contract, topics, data)| {
        event_contract == *contract_id
            && topics == expected_topic
            && T::try_from_val(env, &data).map(|event| predicate(&event)).unwrap_or(false)
    })
}

#[test]
fn test_mark_shipped_transitions_state() {
    let env = Env::default();
    env.mock_all_auths();

    let token = register_token(&env);
    let (contract_id, client, _admin, _fee_collector) = setup_contract(&env);

    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let resolver = Address::generate(&env);

    let id = create_funded_escrow(
        &env, &client, &seller, &buyer, &resolver, &token, 1000, 100, 3600,
    );

    client.mark_shipped(&seller, &id, &SorobanString::from_str(&env, "TRACK-001"));

    assert!(has_event::<crate::EscrowShipped, _>(&env, &contract_id, "escrow_shipped", |event| {
        event.escrow_id == id && event.seller == seller
    }));

    let escrow = client.get_escrow(&id);
    assert_eq!(escrow.state, EscrowState::Shipped);
    assert_eq!(escrow.tracking_id, Some(SorobanString::from_str(&env, "TRACK-001")));
}

#[test]
fn test_mark_shipped_rejects_empty_tracking_id() {
    let env = Env::default();
    env.mock_all_auths();

    let token = register_token(&env);
    let (_contract_id, client, _admin, _fee_collector) = setup_contract(&env);

    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let resolver = Address::generate(&env);

    let id = create_funded_escrow(&env, &client, &seller, &buyer, &resolver, &token, 1000, 100, 3600);

    let res = client.try_mark_shipped(&seller, &id, &SorobanString::from_str(&env, ""));
    assert!(matches!(res, Err(Ok(ContractError::InvalidTrackingId))));

    let escrow = client.get_escrow(&id);
    assert_eq!(escrow.state, EscrowState::Funded);
}

#[test]
fn test_record_delivery_sets_timestamp() {
    let env = Env::default();
    env.mock_all_auths();

    let token = register_token(&env);
    let (contract_id, client, admin, _fee_collector) = setup_contract(&env);

    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let resolver = Address::generate(&env);

    let id = create_funded_escrow(
        &env, &client, &seller, &buyer, &resolver, &token, 1000, 100, 3600,
    );

    client.mark_shipped(&seller, &id, &SorobanString::from_str(&env, "TRACK-002"));

    advance_time(&env, 60);
    let expected_ts = env.ledger().timestamp();

    client.record_delivery(&admin, &id);

    assert!(has_event::<DeliveryRecorded, _>(&env, &contract_id, "delivery_recorded", |event| {
        event.escrow_id == id && event.delivered_at == expected_ts
    }));

    let escrow = client.get_escrow(&id);
    assert_eq!(escrow.state, EscrowState::Shipped);
    assert_eq!(escrow.delivered_at, expected_ts);
}

#[test]
fn test_record_delivery_requires_shipped_state() {
    let env = Env::default();
    env.mock_all_auths();

    let token = register_token(&env);
    let (_contract_id, client, admin, _fee_collector) = setup_contract(&env);

    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let resolver = Address::generate(&env);

    let id = create_funded_escrow(
        &env, &client, &seller, &buyer, &resolver, &token, 1000, 100, 3600,
    );

    let res = client.try_record_delivery(&admin, &id);
    assert!(matches!(res, Err(Ok(crate::ContractError::InvalidState))));
}

#[test]
fn test_confirm_delivery_after_mark_shipped() {
    let env = Env::default();
    env.mock_all_auths();

    let token = register_token(&env);
    let (contract_id, client, _admin, _fee_collector) = setup_contract(&env);

    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let resolver = Address::generate(&env);

    let id = create_funded_escrow(
        &env, &client, &seller, &buyer, &resolver, &token, 1000, 0, 3600,
    );

    client.mark_shipped(&seller, &id, &SorobanString::from_str(&env, "TRACK-003"));
    advance_time(&env, 172801);
    client.confirm_delivery(&buyer, &id);

    assert!(has_event::<crate::EscrowCompleted, _>(&env, &contract_id, "escrow_completed", |event| {
        event.escrow_id == id && event.recipient == seller
    }));

    let escrow = client.get_escrow(&id);
    assert_eq!(escrow.state, EscrowState::Completed);

    let balance = soroban_sdk::token::Client::new(&env, &token).balance(&seller);
    assert_eq!(balance, 1000);

    let _ = contract_id;
}
