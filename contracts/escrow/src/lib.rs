#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, Env, Symbol, Vec,
};

/// Maximum fee basis points (100% = 10000 basis points)
pub const MAX_FEE_BASIS_POINTS: u32 = 10000;

/// Default protocol fee: 2.5% = 250 basis points
pub const DEFAULT_FEE_BASIS_POINTS: u32 = 250;

/// Data keys for contract storage
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    TreasuryConfig,
    Contract(u32),
    Milestone(u32, u32),
    ContractStatus(u32),
    NextContractId,
}

/// Status of an escrow contract
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractStatus {
    Created = 0,
    Funded = 1,
    Completed = 2,
    Disputed = 3,
}

/// Milestone structure for escrow payments
#[contracttype]
#[derive(Clone, Debug)]
pub struct Milestone {
    pub amount: i128,
    pub released: bool,
}

/// Treasury configuration for protocol fee collection
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryConfig {
    /// Address where protocol fees are sent
    pub address: Address,
    /// Fee percentage in basis points (10000 = 100%)
    pub fee_basis_points: u32,
}

/// Escrow contract structure
#[contracttype]
#[derive(Clone, Debug)]
pub struct EscrowContract {
    pub client: Address,
    pub freelancer: Address,
    pub total_amount: i128,
    pub milestone_count: u32,
}

/// Custom errors for the escrow contract
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EscrowError {
    /// Treasury not initialized
    TreasuryNotInitialized = 1,
    /// Invalid fee percentage (exceeds 100%)
    InvalidFeePercentage = 2,
    /// Unauthorized access
    Unauthorized = 3,
    /// Contract not found
    ContractNotFound = 4,
    /// Milestone not found
    MilestoneNotFound = 5,
    /// Milestone already released
    MilestoneAlreadyReleased = 6,
    /// Insufficient funds
    InsufficientFunds = 7,
    /// Invalid amount
    InvalidAmount = 8,
    /// Treasury already initialized
    TreasuryAlreadyInitialized = 9,
    /// Arithmetic overflow
    ArithmeticOverflow = 10,
}

#[contract]
pub struct Escrow;

/// Event topics for audit trail
pub mod topics {
    use soroban_sdk::symbol_short;
    pub const TREASURY_CONFIG_SET: soroban_sdk::Symbol = symbol_short!("TR_CFG");
    pub const PROTOCOL_FEE_COLLECTED: soroban_sdk::Symbol = symbol_short!("FEE");
    pub const TREASURY_PAYOUT: soroban_sdk::Symbol = symbol_short!("PAYOUT");
    pub const MILESTONE_RELEASED: soroban_sdk::Symbol = symbol_short!("RELEASE");
}

#[contractimpl]
impl Escrow {
    // ==================== TREASURY FUNCTIONS ====================

    /// Initialize the treasury configuration.
    /// Can only be called once by the contract deployer (admin).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - The admin address (must be authorized)
    /// * `treasury_address` - The address where protocol fees are sent
    /// * `fee_basis_points` - Fee percentage in basis points (10000 = 100%, 250 = 2.5%)
    ///
    /// # Errors
    /// * `TreasuryAlreadyInitialized` - If treasury is already configured
    /// * `InvalidFeePercentage` - If fee exceeds 100%
    /// * `Unauthorized` - If caller is not the admin
    pub fn initialize_treasury(
        env: Env,
        admin: Address,
        treasury_address: Address,
        fee_basis_points: u32,
    ) -> Result<(), EscrowError> {
        // Verify admin authorization
        admin.require_auth();

        // Check if treasury is already initialized
        if env.storage().persistent().has(&DataKey::TreasuryConfig) {
            return Err(EscrowError::TreasuryAlreadyInitialized);
        }

        // Validate fee percentage (max 100%)
        if fee_basis_points > MAX_FEE_BASIS_POINTS {
            return Err(EscrowError::InvalidFeePercentage);
        }

        // Store admin
        env.storage().persistent().set(&DataKey::Admin, &admin);

        // Create and store treasury config
        let config = TreasuryConfig {
            address: treasury_address.clone(),
            fee_basis_points,
        };
        env.storage()
            .persistent()
            .set(&DataKey::TreasuryConfig, &config);

        // Emit audit event
        env.events().publish(
            (topics::TREASURY_CONFIG_SET,),
            (admin, treasury_address, fee_basis_points),
        );

        Ok(())
    }

    /// Update the treasury configuration.
    /// Only callable by the admin.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - The admin address (must be authorized)
    /// * `new_treasury_address` - The new treasury address
    /// * `new_fee_basis_points` - New fee percentage in basis points
    ///
    /// # Errors
    /// * `TreasuryNotInitialized` - If treasury not yet initialized
    /// * `InvalidFeePercentage` - If fee exceeds 100%
    /// * `Unauthorized` - If caller is not the admin
    pub fn update_treasury_config(
        env: Env,
        admin: Address,
        new_treasury_address: Address,
        new_fee_basis_points: u32,
    ) -> Result<(), EscrowError> {
        // Verify admin authorization
        admin.require_auth();

        // Verify caller is the stored admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::Unauthorized)?;
        if admin != stored_admin {
            return Err(EscrowError::Unauthorized);
        }

        // Validate fee percentage
        if new_fee_basis_points > MAX_FEE_BASIS_POINTS {
            return Err(EscrowError::InvalidFeePercentage);
        }

        // Update treasury config
        let config = TreasuryConfig {
            address: new_treasury_address.clone(),
            fee_basis_points: new_fee_basis_points,
        };
        env.storage()
            .persistent()
            .set(&DataKey::TreasuryConfig, &config);

        // Emit audit event
        env.events().publish(
            (topics::TREASURY_CONFIG_SET,),
            (admin, new_treasury_address, new_fee_basis_points),
        );

        Ok(())
    }

    /// Get the current treasury configuration.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// * `Ok(TreasuryConfig)` - The current treasury configuration
    ///
    /// # Errors
    /// * `TreasuryNotInitialized` - If treasury not yet initialized
    pub fn get_treasury_config(env: Env) -> Result<TreasuryConfig, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::TreasuryConfig)
            .ok_or(EscrowError::TreasuryNotInitialized)
    }

    /// Calculate the protocol fee for a given amount.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `amount` - The payment amount to calculate fee for
    ///
    /// # Returns
    /// * `Ok(i128)` - The calculated fee amount
    ///
    /// # Errors
    /// * `TreasuryNotInitialized` - If treasury not yet initialized
    /// * `ArithmeticOverflow` - If calculation overflows
    pub fn calculate_protocol_fee(env: Env, amount: i128) -> Result<i128, EscrowError> {
        if amount < 0 {
            return Err(EscrowError::InvalidAmount);
        }

        let config = Self::get_treasury_config(env)?;

        // Calculate fee: (amount * fee_basis_points) / 10000
        // Use checked arithmetic to prevent overflow
        let fee = amount
            .checked_mul(config.fee_basis_points as i128)
            .ok_or(EscrowError::ArithmeticOverflow)?
            .checked_div(MAX_FEE_BASIS_POINTS as i128)
            .ok_or(EscrowError::ArithmeticOverflow)?;

        Ok(fee)
    }

    /// Transfer protocol fees to the treasury address.
    /// Internal function used during milestone releases.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `token` - The token contract address
    /// * `amount` - The total payment amount
    ///
    /// # Returns
    /// * `Ok(i128)` - The net amount after fee deduction
    ///
    /// # Errors
    /// * Various `EscrowError` variants on failure
    fn transfer_protocol_fee(
        env: &Env,
        token: &Address,
        from: &Address,
        amount: i128,
    ) -> Result<i128, EscrowError> {
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        let config = Self::get_treasury_config(env.clone())?;
        let fee = Self::calculate_protocol_fee(env.clone(), amount)?;
        let net_amount = amount
            .checked_sub(fee)
            .ok_or(EscrowError::ArithmeticOverflow)?;

        if fee > 0 {
            // Transfer fee to treasury
            let token_client = token::Client::new(env, token);
            token_client.transfer(from, &config.address, &fee);

            // Emit audit event
            env.events().publish(
                (topics::PROTOCOL_FEE_COLLECTED,),
                (config.address.clone(), fee, amount),
            );
        }

        Ok(net_amount)
    }

    /// Direct payout to treasury (for manual fee collection or other purposes).
    /// Only callable by admin.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - The admin address (must be authorized)
    /// * `token` - The token contract address
    /// * `amount` - The amount to transfer to treasury
    ///
    /// # Errors
    /// * `Unauthorized` - If caller is not admin
    /// * `TreasuryNotInitialized` - If treasury not configured
    /// * `InvalidAmount` - If amount is invalid
    pub fn payout_treasury(
        env: Env,
        admin: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), EscrowError> {
        // Verify admin authorization
        admin.require_auth();

        // Verify caller is the stored admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::Unauthorized)?;
        if admin != stored_admin {
            return Err(EscrowError::Unauthorized);
        }

        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        let config = Self::get_treasury_config(env.clone())?;

        // Transfer to treasury
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &config.address, &amount);

        // Emit audit event
        env.events()
            .publish((topics::TREASURY_PAYOUT,), (config.address, amount));

        Ok(())
    }

    // ==================== ESCROW FUNCTIONS ====================

    /// Create a new escrow contract. Client and freelancer addresses are stored
    /// for access control. Milestones define payment amounts.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `client` - The client address (must authorize)
    /// * `freelancer` - The freelancer address
    /// * `milestone_amounts` - Vector of milestone payment amounts
    /// * `token` - The token contract address for payments
    ///
    /// # Returns
    /// * `Ok(u32)` - The contract ID
    pub fn create_contract(
        env: Env,
        client: Address,
        freelancer: Address,
        milestone_amounts: Vec<i128>,
        token: Address,
    ) -> Result<u32, EscrowError> {
        // Client must authorize
        client.require_auth();

        // Get or initialize next contract ID
        let contract_id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::NextContractId)
            .unwrap_or(1);

        // Calculate total amount
        let mut total_amount: i128 = 0;
        for i in 0..milestone_amounts.len() {
            let amount = milestone_amounts.get(i).ok_or(EscrowError::InvalidAmount)?;
            if amount <= 0 {
                return Err(EscrowError::InvalidAmount);
            }
            total_amount = total_amount
                .checked_add(amount)
                .ok_or(EscrowError::ArithmeticOverflow)?;

            // Store milestone
            let milestone = Milestone {
                amount,
                released: false,
            };
            env.storage()
                .persistent()
                .set(&DataKey::Milestone(contract_id, i as u32), &milestone);
        }

        // Store contract
        let escrow_contract = EscrowContract {
            client: client.clone(),
            freelancer: freelancer.clone(),
            total_amount,
            milestone_count: milestone_amounts.len() as u32,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Contract(contract_id), &escrow_contract);

        // Store contract status
        env.storage().persistent().set(
            &DataKey::ContractStatus(contract_id),
            &ContractStatus::Created,
        );

        // Store token address for this contract
        env.storage().persistent().set(
            &DataKey::Contract(contract_id),
            &(token, client, freelancer),
        );

        // Increment next contract ID
        env.storage()
            .persistent()
            .set(&DataKey::NextContractId, &(contract_id + 1));

        Ok(contract_id)
    }

    /// Deposit funds into escrow. Only the client may call this.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    /// * `amount` - The amount to deposit
    /// * `token` - The token contract address
    ///
    /// # Returns
    /// * `Ok(())` on success
    pub fn deposit_funds(
        env: Env,
        contract_id: u32,
        amount: i128,
        token: Address,
    ) -> Result<(), EscrowError> {
        // Retrieve contract
        let (stored_token, client, _): (Address, Address, Address) = env
            .storage()
            .persistent()
            .get(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)?;

        // Verify token matches
        if token != stored_token {
            return Err(EscrowError::InvalidAmount);
        }

        // Client must authorize
        client.require_auth();

        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        // Transfer tokens from client to contract
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&client, &env.current_contract_address(), &amount);

        // Update status to funded
        env.storage().persistent().set(
            &DataKey::ContractStatus(contract_id),
            &ContractStatus::Funded,
        );

        Ok(())
    }

    /// Release a milestone payment to the freelancer after verification.
    /// Deducts protocol fee and transfers to treasury.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    /// * `milestone_id` - The milestone index to release
    ///
    /// # Returns
    /// * `Ok(())` on success
    pub fn release_milestone(
        env: Env,
        contract_id: u32,
        milestone_id: u32,
    ) -> Result<(), EscrowError> {
        // Retrieve contract
        let (token, client, freelancer): (Address, Address, Address) = env
            .storage()
            .persistent()
            .get(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)?;

        // Client must authorize
        client.require_auth();

        // Retrieve milestone
        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(contract_id, milestone_id))
            .ok_or(EscrowError::MilestoneNotFound)?;

        // Check if already released
        if milestone.released {
            return Err(EscrowError::MilestoneAlreadyReleased);
        }

        // Calculate and transfer protocol fee
        let net_amount = Self::transfer_protocol_fee(
            &env,
            &token,
            &env.current_contract_address(),
            milestone.amount,
        )?;

        // Transfer net amount to freelancer
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &freelancer, &net_amount);

        // Mark milestone as released
        milestone.released = true;
        env.storage()
            .persistent()
            .set(&DataKey::Milestone(contract_id, milestone_id), &milestone);

        // Emit milestone released event
        env.events().publish(
            (topics::MILESTONE_RELEASED,),
            (contract_id, milestone_id, freelancer, net_amount),
        );

        Ok(())
    }

    /// Issue a reputation credential for the freelancer after contract completion.
    pub fn issue_reputation(_env: Env, _freelancer: Address, _rating: i128) -> bool {
        // Reputation credential issuance.
        true
    }

    /// Get the admin address.
    pub fn get_admin(env: Env) -> Result<Address, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::Unauthorized)
    }

    /// Hello-world style function for testing and CI.
    pub fn hello(_env: Env, to: Symbol) -> Symbol {
        to
    }
}

#[cfg(test)]
mod test;
