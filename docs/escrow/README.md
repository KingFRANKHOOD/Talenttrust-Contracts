# Escrow Contract Documentation

The Escrow contract provides a secure, milestone-based payment system for freelancers and clients.

## Key Features

### Idempotent Milestone Release
To prevent duplicate payments and replay-style calls, the `release_milestone` function incorporates idempotency protection. 

- **Mechanism**: Each milestone tracks its `released` status.
- **Protection**: If a client attempts to release a milestone that has already been marked as released, the contract will panic with `Error::AlreadyReleased`.
- **Benefit**: This ensures that even if a transaction is retried or a duplicate call is made, the freelancer only receives the payment once per milestone.

### On-Chain Metadata References
The contract supports storing references to off-chain work evidence and deliverables.

- **Storage**: When a milestone is released, the caller provides a `work_evidence` string (e.g., an IPFS CID, a URL, or a hash of the deliverable).
- **Verification**: This creates a permanent, on-chain link between the payment and the work performed, providing transparency and a trail for future reference or dispute resolution.

## Contract Interface

### `create_contract(client: Address, freelancer: Address, milestone_amounts: Vec<i128>)`
Initializes the escrow with participants and payment stages. Can only be called once.

### `deposit_funds(amount: i128)`
Allows the client to deposit funds into the contract. Requires client authorization.

### `release_milestone(milestone_id: u32, work_evidence: String)`
Releases a specific milestone to the freelancer. 
- **Requires**: Client authorization.
- **Idempotency**: Prevents duplicate releases.
- **Metadata**: Stores the provided `work_evidence`.

### `get_milestones() -> Vec<Milestone>`
Returns the current state of all milestones, including their amounts, release status, and work evidence.

## Error Codes
- `AlreadyInitialized (1)`: Contract already has a client and freelancer.
- `NotInitialized (2)`: Contract has not been set up.
- `IndexOutOfBounds (3)`: The provided milestone ID does not exist.
- `AlreadyReleased (4)`: The milestone has already been released.
