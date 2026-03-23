use soroban_sdk::{symbol_short, testutils::Address as _, vec, Address, Env};

use crate::{Escrow, EscrowClient, DEFAULT_FEE_BASIS_POINTS, MAX_FEE_BASIS_POINTS};

// ==================== HELPER FUNCTIONS ====================

/// Setup function to create environment and register contract
fn setup() -> (Env, Address, EscrowClient<'static>) {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);
    (env, contract_id, client)
}

/// Setup with treasury initialized
fn setup_with_treasury() -> (Env, Address, EscrowClient<'static>, Address, Address) {
    let (env, contract_id, client) = setup();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    // Initialize treasury with default 2.5% fee
    client
        .mock_all_auths()
        .initialize_treasury(&admin, &treasury, &DEFAULT_FEE_BASIS_POINTS);

    (env, contract_id, client, admin, treasury)
}

// ==================== TREASURY INITIALIZATION TESTS ====================

#[test]
fn test_initialize_treasury_success() {
    let (env, _contract_id, client) = setup();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    // Initialize treasury
    client
        .mock_all_auths()
        .initialize_treasury(&admin, &treasury, &DEFAULT_FEE_BASIS_POINTS);

    // Verify treasury config
    let config = client.get_treasury_config();
    assert_eq!(config.address, treasury);
    assert_eq!(config.fee_basis_points, DEFAULT_FEE_BASIS_POINTS);

    // Verify admin
    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, admin);
}

#[test]
fn test_initialize_treasury_with_zero_fee() {
    let (env, _contract_id, client) = setup();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    // Initialize with 0% fee
    client
        .mock_all_auths()
        .initialize_treasury(&admin, &treasury, &0);

    let config = client.get_treasury_config();
    assert_eq!(config.fee_basis_points, 0);
}

#[test]
fn test_initialize_treasury_with_max_fee() {
    let (env, _contract_id, client) = setup();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    // Initialize with 100% fee
    client
        .mock_all_auths()
        .initialize_treasury(&admin, &treasury, &MAX_FEE_BASIS_POINTS);

    let config = client.get_treasury_config();
    assert_eq!(config.fee_basis_points, MAX_FEE_BASIS_POINTS);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_initialize_treasury_already_initialized() {
    let (env, _contract_id, client) = setup();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    // First initialization
    client
        .mock_all_auths()
        .initialize_treasury(&admin, &treasury, &DEFAULT_FEE_BASIS_POINTS);

    // Second initialization should fail
    let admin2 = Address::generate(&env);
    client
        .mock_all_auths()
        .initialize_treasury(&admin2, &treasury, &DEFAULT_FEE_BASIS_POINTS);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_initialize_treasury_invalid_fee() {
    let (env, _contract_id, client) = setup();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    // Fee exceeding 100% should fail
    client
        .mock_all_auths()
        .initialize_treasury(&admin, &treasury, &(MAX_FEE_BASIS_POINTS + 1));
}

// ==================== TREASURY CONFIG UPDATE TESTS ====================

#[test]
fn test_update_treasury_config_success() {
    let (env, _contract_id, client, admin, _treasury) = setup_with_treasury();
    let new_treasury = Address::generate(&env);
    let new_fee: u32 = 500; // 5%

    // Update treasury config
    client
        .mock_all_auths()
        .update_treasury_config(&admin, &new_treasury, &new_fee);

    // Verify updated config
    let config = client.get_treasury_config();
    assert_eq!(config.address, new_treasury);
    assert_eq!(config.fee_basis_points, new_fee);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_update_treasury_config_unauthorized() {
    let (env, _contract_id, client, _admin, _treasury) = setup_with_treasury();
    let unauthorized = Address::generate(&env);
    let new_treasury = Address::generate(&env);

    // Unauthorized update should fail
    client
        .mock_all_auths()
        .update_treasury_config(&unauthorized, &new_treasury, &500);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_update_treasury_config_invalid_fee() {
    let (env, _contract_id, client, admin, _treasury) = setup_with_treasury();
    let new_treasury = Address::generate(&env);

    // Fee exceeding 100% should fail
    client.mock_all_auths().update_treasury_config(
        &admin,
        &new_treasury,
        &(MAX_FEE_BASIS_POINTS + 1),
    );
}

// ==================== FEE CALCULATION TESTS ====================

#[test]
fn test_calculate_protocol_fee_default() {
    let (env, _contract_id, client, _admin, _treasury) = setup_with_treasury();

    // Test with 2.5% fee (250 basis points)
    let amount: i128 = 1_000_0000000; // 1000 tokens
    let fee = client.calculate_protocol_fee(&amount);

    // Expected: (1000 * 250) / 10000 = 25 tokens
    let expected_fee: i128 = 25_0000000;
    assert_eq!(fee, expected_fee);
}

#[test]
fn test_calculate_protocol_fee_zero() {
    let (env, _contract_id, client) = setup();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    // Initialize with 0% fee
    client
        .mock_all_auths()
        .initialize_treasury(&admin, &treasury, &0);

    let amount: i128 = 1_000_0000000;
    let fee = client.calculate_protocol_fee(&amount);
    assert_eq!(fee, 0);
}

#[test]
fn test_calculate_protocol_fee_100_percent() {
    let (env, _contract_id, client) = setup();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    // Initialize with 100% fee
    client
        .mock_all_auths()
        .initialize_treasury(&admin, &treasury, &MAX_FEE_BASIS_POINTS);

    let amount: i128 = 1_000_0000000;
    let fee = client.calculate_protocol_fee(&amount);
    assert_eq!(fee, amount);
}

#[test]
fn test_calculate_protocol_fee_small_amount() {
    let (env, _contract_id, client, _admin, _treasury) = setup_with_treasury();

    // Small amount that results in fractional fee (should round down)
    let amount: i128 = 100; // Very small amount
    let fee = client.calculate_protocol_fee(&amount);

    // (100 * 250) / 10000 = 2.5 -> rounds to 2
    assert_eq!(fee, 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_calculate_protocol_fee_negative_amount() {
    let (env, _contract_id, client, _admin, _treasury) = setup_with_treasury();

    let amount: i128 = -1000;
    client.calculate_protocol_fee(&amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_calculate_protocol_fee_not_initialized() {
    let (env, _contract_id, client) = setup();

    let amount: i128 = 1_000_0000000;
    client.calculate_protocol_fee(&amount);
}

// ==================== GET TREASURY CONFIG TESTS ====================

#[test]
fn test_get_treasury_config_success() {
    let (env, _contract_id, client, admin, treasury) = setup_with_treasury();

    let config = client.get_treasury_config();
    assert_eq!(config.address, treasury);
    assert_eq!(config.fee_basis_points, DEFAULT_FEE_BASIS_POINTS);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_get_treasury_config_not_initialized() {
    let (env, _contract_id, client) = setup();

    client.get_treasury_config();
}

// ==================== GET ADMIN TESTS ====================

#[test]
fn test_get_admin_success() {
    let (env, _contract_id, client, admin, _treasury) = setup_with_treasury();

    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_get_admin_not_initialized() {
    let (env, _contract_id, client) = setup();

    client.get_admin();
}

// ==================== EXISTING FUNCTION TESTS ====================

#[test]
fn test_hello() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let result = client.hello(&symbol_short!("World"));
    assert_eq!(result, symbol_short!("World"));
}

// ==================== CONTRACT CREATION TESTS ====================

#[test]
fn test_create_contract_success() {
    let (env, _contract_id, client, _admin, _treasury) = setup_with_treasury();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let token = Address::generate(&env);
    let milestones = vec![&env, 200_0000000_i128, 400_0000000_i128, 600_0000000_i128];

    env.mock_all_auths();
    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones, &token);
    assert_eq!(id, 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_create_contract_invalid_milestone_amount() {
    let (env, _contract_id, client, _admin, _treasury) = setup_with_treasury();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let token = Address::generate(&env);
    let milestones = vec![&env, 0_i128]; // Invalid: zero amount

    env.mock_all_auths();
    client.create_contract(&client_addr, &freelancer_addr, &milestones, &token);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_create_contract_negative_milestone() {
    let (env, _contract_id, client, _admin, _treasury) = setup_with_treasury();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let token = Address::generate(&env);
    let milestones = vec![&env, -100_i128]; // Invalid: negative amount

    env.mock_all_auths();
    client.create_contract(&client_addr, &freelancer_addr, &milestones, &token);
}

// ==================== DEPOSIT FUNDS TESTS ====================

#[test]
fn test_deposit_funds_contract_not_found() {
    let (env, _contract_id, client, _admin, _treasury) = setup_with_treasury();

    let token = Address::generate(&env);

    // Try to deposit to non-existent contract should fail
    // Note: This will fail with storage error since contract doesn't exist
    // The actual deposit_funds requires a real token contract for full testing
    // This test documents the expected behavior
    env.mock_all_auths();
    let result = client.try_deposit_funds(&999, &100_0000000_i128, &token);
    assert!(result.is_err());
}

// ==================== RELEASE MILESTONE TESTS ====================

#[test]
fn test_release_milestone_contract_not_found() {
    let (env, _contract_id, client, _admin, _treasury) = setup_with_treasury();

    // Try to release milestone for non-existent contract
    env.mock_all_auths();
    let result = client.try_release_milestone(&999, &0);
    assert!(result.is_err());
}

// ==================== ISSUE REPUTATION TESTS ====================

#[test]
fn test_issue_reputation() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let freelancer = Address::generate(&env);
    let result = client.issue_reputation(&freelancer, &5);
    assert!(result);
}

// ==================== EDGE CASE TESTS ====================

#[test]
fn test_fee_calculation_precision() {
    let (env, _contract_id, client) = setup();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    // Test with 1% fee (100 basis points)
    client
        .mock_all_auths()
        .initialize_treasury(&admin, &treasury, &100);

    // Amount that tests precision: 10000 stroops with 1% fee = 100 stroops
    let amount: i128 = 10000;
    let fee = client.calculate_protocol_fee(&amount);
    assert_eq!(fee, 100);
}

#[test]
fn test_multiple_contracts() {
    let (env, _contract_id, client, _admin, _treasury) = setup_with_treasury();

    let client1 = Address::generate(&env);
    let freelancer1 = Address::generate(&env);
    let token = Address::generate(&env);
    let milestones1 = vec![&env, 100_0000000_i128];

    env.mock_all_auths();
    let id1 = client.create_contract(&client1, &freelancer1, &milestones1, &token);
    assert_eq!(id1, 1);

    let client2 = Address::generate(&env);
    let freelancer2 = Address::generate(&env);
    let milestones2 = vec![&env, 200_0000000_i128];

    env.mock_all_auths();
    let id2 = client.create_contract(&client2, &freelancer2, &milestones2, &token);
    assert_eq!(id2, 2);
}

// ==================== INTEGRATION TESTS ====================

#[test]
fn test_treasury_with_multiple_contracts() {
    let (env, _contract_id, client, _admin, treasury) = setup_with_treasury();

    // Setup participants
    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let token = Address::generate(&env);

    // Create first contract
    let milestones1 = vec![&env, 100_0000000_i128, 200_0000000_i128];
    env.mock_all_auths();
    let contract_id1 = client.create_contract(&client_addr, &freelancer_addr, &milestones1, &token);
    assert_eq!(contract_id1, 1);

    // Create second contract
    let milestones2 = vec![&env, 300_0000000_i128];
    env.mock_all_auths();
    let contract_id2 = client.create_contract(&client_addr, &freelancer_addr, &milestones2, &token);
    assert_eq!(contract_id2, 2);

    // Verify treasury config still intact
    let config = client.get_treasury_config();
    assert_eq!(config.address, treasury);
    assert_eq!(config.fee_basis_points, DEFAULT_FEE_BASIS_POINTS);
}

#[test]
fn test_treasury_config_persistence() {
    let (env, _contract_id, client, admin, _treasury) = setup_with_treasury();

    // Get initial config
    let config1 = client.get_treasury_config();

    // Update config
    let new_treasury = Address::generate(&env);
    let new_fee: u32 = 1000; // 10%

    client
        .mock_all_auths()
        .update_treasury_config(&admin, &new_treasury, &new_fee);

    // Verify update persisted
    let config2 = client.get_treasury_config();
    assert_eq!(config2.address, new_treasury);
    assert_eq!(config2.fee_basis_points, new_fee);
    assert_ne!(config1.fee_basis_points, config2.fee_basis_points);
}
