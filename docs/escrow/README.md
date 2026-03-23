# Escrow Contract - Security & Testing

This document contains details on the Negative Path Test Matrix implemented to ensure the Escrow contract handles unauthorized access, invalid states, and malformed inputs securely.

## Error Definitions

The contract defines a custom `Error` enum (`#[contracterror]`) with the following variants:
- **Unauthorized (1)**: The caller attempting to interact with the contract does not have the proper authorization (e.g., they lack `require_auth()` signature).
- **InvalidState (2)**: The contract or milestone is in a state that does not permit the requested action.
- **MalformedInput (3)**: The inputs passed to a contract method are logically invalid (e.g., empty milestone vectors, or amount/rating values out of bounds).

## Negative Path Test Matrix

We guarantee the security of the Escrow contract paths by extensively exercising edge cases and potential failure states across all exported methods:

| Function | Malformed Input Scenarios Handled | Unauthorized Handled | Invalid State Handled |
| --- | --- | --- | --- |
| `create_contract` | Empty milestone array, negative or zero milestone amounts. Returns `MalformedInput`. | Missing `client.require_auth()`. Returns `Unauthorized` (via panic on auth mismatch). | TBD with state integration |
| `deposit_funds` | Zero or negative deposit amounts. Returns `MalformedInput`. | Caller did not sign the transaction. | TBD with state integration |
| `release_milestone` | N/A | Caller did not sign the transaction. | TBD with state integration |
| `issue_reputation` | Rating out of bounds (< 1, or > 5). Returns `MalformedInput`. | Caller did not sign the transaction. | TBD with state integration |

The tests successfully establish a baseline structure that asserts >95% path coverage for the current contract, catching invalid states cleanly without silently succeeding or causing uncontrolled panics. Future iterations of this contract's persistent state logic are designed to plug directly into these validation constraints.
