# Web3 Suite — Identity Contracts

> Soroban smart contracts for decentralized identity primitives on the Stellar network — DID registry, verifiable credentials, and tiered KYC verification.

[![Stellar](https://img.shields.io/badge/Built%20on-Stellar-08B5E5?style=for-the-badge&logo=stellar&logoColor=white)](https://stellar.org)
[![Soroban](https://img.shields.io/badge/Smart%20Contracts-Soroban-08B5E5?style=for-the-badge)](https://soroban.stellar.org)
[![Rust](https://img.shields.io/badge/Language-Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![Issues](https://img.shields.io/github/issues/sudo-robi/web3-suite-identity-contracts)](https://github.com/sudo-robi/web3-suite-identity-contracts/issues)
[![Stars](https://img.shields.io/github/stars/sudo-robi/web3-suite-identity-contracts)](https://github.com/sudo-robi/web3-suite-identity-contracts/stargazers)

---

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Features](#features)
- [Tech Stack](#tech-stack)
- [Project Structure](#project-structure)
- [Smart Contracts](#smart-contracts)
  - [DID Registry](#did-registry)
  - [Verifiable Credentials](#verifiable-credentials)
  - [KYC Verification](#kyc-verification)
- [Error Codes](#error-codes)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
  - [Building](#building)
  - [Testing](#testing)
  - [Deployment](#deployment)
- [Configuration](#configuration)
- [Security Considerations](#security-considerations)
- [Contributing](#contributing)
- [License](#license)

---

## Overview

### The Problem

Decentralized identity on blockchain networks has historically been fragmented. Developers building identity-aware dApps on Stellar had no composable primitive layer — no standardized DID registry, no on-chain verifiable credential issuance, and no integrated KYC verification. Each project had to reimplement identity primitives from scratch, leading to siloed identity systems that couldn't interoperate.

### The Solution

This workspace provides three composable Soroban smart contracts that together form a complete on-chain identity layer for the Stellar network:

1. **DID Registry** — Create, resolve, update, deactivate, and transfer decentralized identifiers
2. **Verifiable Credentials** — Issue, verify, and revoke typed credentials linked to DIDs
3. **KYC Verification** — Tiered identity verification with verifier registries and expiration

The contracts are designed as independent, composable primitives. Any downstream system — wallets, DeFi protocols, DAOs — can query these contracts to establish trust without relying on centralized authorities.

### Target Audience

- **dApp developers** building identity-aware applications on Stellar
- **DeFi protocols** requiring KYC-gated access or credential verification
- **DAOs** managing membership through verifiable credentials
- **Wallet developers** integrating DID resolution and credential storage
- **Enterprise teams** building compliance-ready identity solutions on Stellar

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Stellar Network                              │
│                                                                     │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐  │
│  │   DID Registry   │  │   Credentials    │  │  KYC Verification│  │
│  │                  │  │                  │  │                  │  │
│  │  create_did()    │  │  issue_credential│  │  submit_kyc()    │  │
│  │  resolve_did()   │◄─┤  verify_credential│◄─┤  approve_kyc()   │  │
│  │  update_did()    │  │  revoke_credential│  │  reject_kyc()    │  │
│  │  deactivate_did()│  │  get_credential() │  │  verify_kyc()   │  │
│  │  transfer_did()  │  │  issuer/subject  │  │  register_verify │  │
│  │  is_active()     │  │  credential lists│  │  is_verified()   │  │
│  └────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘  │
│           │                     │                     │             │
│           │    ┌────────────────┴─────────────────────┘             │
│           │    │                                                    │
│  ┌────────▼────▼──────────────────────────────────────────────────┐ │
│  │              Persistent Storage (env.storage)                  │ │
│  │                                                                │ │
│  │  DID Records          Credential Records      KYC Records      │ │
│  │  ├─ did_id → record   ├─ cred_id → record     ├─ addr → record │ │
│  │  └─ counter           ├─ issuer → [cred_ids]  ├─ verifier → T  │ │
│  │                       └─ subject → [cred_ids] └─ admin → addr  │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
                                    ▲
                                    │
                           ┌────────┴────────┐
                           │  Client Apps     │
                           │  (Backend/API)   │
                           └─────────────────┘
```

### Identity Lifecycle

```
  User creates DID ─────► DID Document stored on-chain
         │
         ▼
  Issuer issues credential ─────► Credential linked to DID
         │
         ▼
  User submits KYC ─────► KYC record linked to DID
         │
         ▼
  Verifier approves ─────► KYC Verified ✓
         │
         ▼
  Apps check: is_verified() + verify_credential() + is_active()
```

### Data Flow Between Contracts

```
┌────────────┐      references       ┌────────────────┐
│    DID     │◄──────────────────────│   Credentials   │
│  Registry  │                       │    Contract      │
└─────┬──────┘                       └────────┬───────┘
      │                                       │
      │         links DID to KYC              │
      ▼                                       ▼
┌────────────┐                       ┌────────────────┐
│    KYC     │──────────────────────►│  Credential    │
│Verification│    validates via      │  Verification  │
└────────────┘                       └────────────────┘
```

---

## Features

### DID Registry (6 features)

1. **Create DID** — Generate a unique 32-byte identifier derived from SHA-256(owner + counter + document). The DID document is stored on-chain with ownership metadata.

2. **Resolve DID** — Look up any DID to retrieve its full record including owner, document, timestamps, and active status. Public read access enables trustless verification.

3. **Update DID** — Modify a DID's document. Only the owner can update, and the DID must be active. Updates are timestamped for audit trails.

4. **Deactivate DID** — Permanently deactivate a DID. The owner can deactivate at any time. Deactivated DIDs cannot be updated or transferred.

5. **Transfer DID** — Transfer ownership of a DID to a new Stellar address. The DID must be active and the caller must be the current owner.

6. **Active Check** — Gas-efficient boolean check for whether a DID is currently active, without returning the full record.

### Verifiable Credentials (6 features)

7. **Issue Credential** — Issuers create typed credentials with claims, optional expiry, and cryptographic signatures. Credentials are indexed by both issuer and subject.

8. **Verify Credential** — Validate that a credential exists, has not been revoked, and has not expired. Returns a simple boolean for gas-efficient verification.

9. **Revoke Credential** — Issuers can revoke credentials they created. Revocation is irreversible and immediately invalidates verification.

10. **Get Credential** — Retrieve the full credential record including issuer, subject, type, claims, timestamps, revocation status, and signature.

11. **Issuer Credentials** — List all credential IDs issued by a given address, enabling portfolio views and issuer reputation systems.

12. **Subject Credentials** — List all credential IDs held by a given DID, enabling wallet-style credential browsing.

### KYC Verification (6 features)

13. **Submit KYC** — Applicants submit KYC applications with a DID reference, target level, and data hash. Prevents duplicate active applications.

14. **Approve KYC** — Registered verifiers approve pending applications. Sets a 1-year validity window and records the verifier identity.

15. **Reject KYC** — Registered verifiers reject pending applications with a reason. Rejection is recorded for audit purposes.

16. **Verify KYC** — Retrieve the full KYC record for an applicant, including auto-expiration check for expired approvals.

17. **Register Verifier** — Admins register authorized verifiers. The first caller to `register_verifier` becomes the contract admin.

18. **Level Check** — Gas-efficient check whether an applicant meets a minimum KYC level requirement (Basic, Enhanced, or Institutional).

---

## Tech Stack

| Component | Technology | Version | Purpose |
|-----------|-----------|---------|---------|
| Language | Rust | Latest Stable | Smart contract implementation |
| Framework | Soroban SDK | 21.0.0 | Stellar smart contract development |
| Network | Stellar | Testnet/Mainnet | Blockchain deployment target |
| Storage | `persistent::Map` | — | On-chain persistent key-value storage |
| Events | Soroban Events | — | On-chain event emission for indexing |
| Testing | `soroban-sdk` testutils | — | In-memory contract testing |
| Build | Cargo | — | Rust package manager and build system |
| WASM | `wasm32-unknown-unknown` | — | Compilation target for on-chain execution |

### Build Profile

The release profile is optimized for minimal WASM binary size:

| Setting | Value | Purpose |
|---------|-------|---------|
| `opt-level` | `z` | Optimize for size |
| `overflow-checks` | `true` | Runtime overflow protection |
| `strip` | `symbols` | Strip debug symbols |
| `panic` | `abort` | No unwinding (smaller binary) |
| `lto` | `true` | Link-time optimization |
| `codegen-units` | `1` | Single codegen unit for maximum optimization |

---

## Project Structure

```
web3-suite-identity-contracts/
├── Cargo.toml                          # Workspace configuration
│                                       #   - Defines 3 workspace members
│                                       #   - soroban-sdk = "21.0.0" dependency
│                                       #   - Release profile for WASM optimization
│
├── contracts/
│   ├── did/                            # DID Registry contract
│   │   ├── Cargo.toml                  #   - Package: web3-suite-identity-did
│   │   └── src/
│   │       └── lib.rs                  #   - 357 lines
│   │                                   #   - DIDRegistry contractimpl
│   │                                   #   - DIDRecord struct
│   │                                   #   - DIDError enum (5 variants)
│   │                                   #   - 4 event types
│   │                                   #   - 7 unit tests
│   │
│   ├── credentials/                    # Verifiable Credentials contract
│   │   ├── Cargo.toml                  #   - Package: web3-suite-identity-credentials
│   │   └── src/
│   │       └── lib.rs                  #   - 372 lines
│   │                                   #   - VerifiableCredentials contractimpl
│   │                                   #   - CredentialRecord struct
│   │                                   #   - CredentialError enum (6 variants)
│   │                                   #   - Issuer/subject index maps
│   │                                   #   - 6 unit tests
│   │
│   └── kyc/                            # KYC Verification contract
│       ├── Cargo.toml                  #   - Package: web3-suite-identity-kyc
│       └── src/
│           └── lib.rs                  #   - 389 lines
│                                       #   - KYCVerification contractimpl
│                                       #   - KYCRecord, KYCLevel, KYCStatus
│                                       #   - KYCError enum (8 variants)
│                                       #   - Admin/verifier registry
│                                       #   - 4 unit tests
│
├── tests/
│   └── integration.rs                  # Cross-contract integration tests
│
├── LICENSE                             # MIT License
├── CONTRIBUTING.md                     # Contribution guidelines
└── README.md                           # This file
```

---

## Smart Contracts

### DID Registry

The DID Registry manages Decentralized Identifiers on-chain. Each DID is a unique 32-byte identifier derived from a SHA-256 hash of the owner address, a counter, and the initial document content.

#### Storage Layout

```
persistent::Map<BytesN<32>, DIDRecord>    // did_id → record
instance::DataKey::Counter                 // auto-incrementing counter
```

#### DIDRecord

```rust
pub struct DIDRecord {
    pub owner: Address,        // Stellar address that owns this DID
    pub document: Bytes,       // DID document content (arbitrary bytes)
    pub created_at: u64,       // Ledger timestamp at creation
    pub updated_at: u64,       // Ledger timestamp at last update
    pub active: bool,          // Whether the DID is currently active
}
```

#### Functions

##### `create_did(owner: Address, document: Bytes) -> Result<BytesN<32>, DIDError>`

Create a new DID with a unique ID derived from SHA-256(owner || counter || document).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | `Address` | Yes | Stellar address that will own the DID. Requires auth. |
| `document` | `Bytes` | Yes | DID document content. Must be non-empty. |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(did_id)` | `BytesN<32>` | The 32-byte unique DID identifier |
| `Err(InvalidDocument)` | — | Document is empty |
| `Err(AlreadyExists)` | — | DID with this hash already exists |

**Events emitted:** `DIDCreated(did_id, owner, timestamp)`

```rust
// Example usage
let doc = Bytes::from_slice(&env, b"https://example.com/did/doc1");
let did_id = contract.create_did(&owner, &doc);
// did_id = BytesN<32> containing the SHA-256 hash
```

---

##### `resolve_did(did_id: BytesN<32>) -> Result<DIDRecord, DIDError>`

Resolve a DID to its full record. Public read access.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `did_id` | `BytesN<32>` | Yes | The DID identifier to resolve |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(record)` | `DIDRecord` | Full DID record |
| `Err(NotFound)` | — | DID does not exist |

```rust
let record = contract.resolve_did(&did_id);
assert_eq!(record.owner, owner);
assert_eq!(record.document, doc);
assert!(record.active);
```

---

##### `update_did(did_id: BytesN<32>, caller: Address, new_document: Bytes) -> Result<(), DIDError>`

Update a DID's document. Only the owner may call this. The DID must be active.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `did_id` | `BytesN<32>` | Yes | The DID to update |
| `caller` | `Address` | Yes | Must be the DID owner. Requires auth. |
| `new_document` | `Bytes` | Yes | New document content. Must be non-empty. |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(())` | — | Document updated successfully |
| `Err(NotFound)` | — | DID does not exist |
| `Err(NotOwner)` | — | Caller is not the DID owner |
| `Err(Inactive)` | — | DID has been deactivated |
| `Err(InvalidDocument)` | — | New document is empty |

**Events emitted:** `DIDUpdated(did_id, timestamp)`

---

##### `deactivate_did(did_id: BytesN<32>, caller: Address) -> Result<(), DIDError>`

Deactivate a DID. Only the owner may call this. Deactivation is permanent.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `did_id` | `BytesN<32>` | Yes | The DID to deactivate |
| `caller` | `Address` | Yes | Must be the DID owner. Requires auth. |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(())` | — | DID deactivated |
| `Err(NotFound)` | — | DID does not exist |
| `Err(NotOwner)` | — | Caller is not the DID owner |

**Events emitted:** `DIDDeactivated(did_id, timestamp)`

---

##### `transfer_did(did_id: BytesN<32>, caller: Address, new_owner: Address) -> Result<(), DIDError>`

Transfer DID ownership to a new address. The DID must be active.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `did_id` | `BytesN<32>` | Yes | The DID to transfer |
| `caller` | `Address` | Yes | Must be the current owner. Requires auth. |
| `new_owner` | `Address` | Yes | New owner's Stellar address |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(())` | — | Ownership transferred |
| `Err(NotFound)` | — | DID does not exist |
| `Err(NotOwner)` | — | Caller is not the DID owner |
| `Err(Inactive)` | — | DID has been deactivated |

**Events emitted:** `DIDTransferred(did_id, from, to, timestamp)`

---

##### `is_active(did_id: BytesN<32>) -> bool`

Check if a DID is currently active. Gas-efficient boolean check.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `did_id` | `BytesN<32>` | Yes | The DID to check |

| Return | Type | Description |
|--------|------|-------------|
| `true` | `bool` | DID exists and is active |
| `false` | `bool` | DID does not exist or is inactive |

---

### Verifiable Credentials

The Credentials contract enables issuance, verification, and revocation of credentials linked to DIDs. Credentials carry typed claims, optional expiry, and cryptographic signatures.

#### Storage Layout

```
persistent::Map<BytesN<32>, CredentialRecord>    // cred_id → record
persistent::Map<Address, Vec<BytesN<32>>>         // issuer → [cred_ids]
persistent::Map<BytesN<32>, Vec<BytesN<32>>>      // subject_did → [cred_ids]
instance::DataKey::Counter                         // auto-incrementing counter
```

#### CredentialRecord

```rust
pub struct CredentialRecord {
    pub issuer: Address,             // Stellar address of the issuer
    pub subject: BytesN<32>,         // DID of the credential subject
    pub credential_type: Bytes,      // Type of credential (e.g., "ProofOfIdentity")
    pub claims: Bytes,               // Claims payload (e.g., JSON)
    pub issued_at: u64,              // Ledger timestamp at issuance
    pub expires_at: Option<u64>,     // Optional expiry timestamp
    pub revoked: bool,               // Whether the credential is revoked
    pub signature: Bytes,            // Cryptographic signature
}
```

#### Functions

##### `issue_credential(issuer, subject, credential_type, claims, expires_at, signature) -> Result<BytesN<32>, CredentialError>`

Issue a new verifiable credential. Returns the credential ID.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `issuer` | `Address` | Yes | Issuer's Stellar address. Requires auth. |
| `subject` | `BytesN<32>` | Yes | Subject's DID identifier |
| `credential_type` | `Bytes` | Yes | Credential type. Must be non-empty. |
| `claims` | `Bytes` | Yes | Claims payload. Must be non-empty. |
| `expires_at` | `Option<u64>` | No | Expiry timestamp. Must be in the future if provided. |
| `signature` | `Bytes` | Yes | Cryptographic signature |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(credential_id)` | `BytesN<32>` | Unique credential identifier |
| `Err(InvalidCredential)` | — | Empty type, claims, or past expiry |

**Events emitted:** `CredentialIssued(credential_id, issuer, timestamp)`

```rust
let cred_type = Bytes::from_slice(&env, b"ProofOfIdentity");
let claims = Bytes::from_slice(&env, b"{\"name\":\"Alice\"}");
let sig = Bytes::from_slice(&env, b"sig123");

let cred_id = contract.issue_credential(
    &issuer, &subject, &cred_type, &claims, &None, &sig,
);
assert!(contract.verify_credential(&cred_id));
```

---

##### `verify_credential(credential_id: BytesN<32>) -> bool`

Verify that a credential is valid (exists, not revoked, not expired).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `credential_id` | `BytesN<32>` | Yes | Credential to verify |

| Return | Type | Description |
|--------|------|-------------|
| `true` | `bool` | Credential is valid |
| `false` | `bool` | Credential not found, revoked, or expired |

---

##### `revoke_credential(credential_id: BytesN<32>, caller: Address) -> Result<(), CredentialError>`

Revoke a credential. Only the issuer may call this. Revocation is irreversible.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `credential_id` | `BytesN<32>` | Yes | Credential to revoke |
| `caller` | `Address` | Yes | Must be the credential issuer. Requires auth. |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(())` | — | Credential revoked |
| `Err(NotFound)` | — | Credential does not exist |
| `Err(NotIssuer)` | — | Caller is not the issuer |
| `Err(AlreadyRevoked)` | — | Credential was already revoked |

**Events emitted:** `CredentialRevoked(credential_id, timestamp)`

---

##### `get_credential(credential_id: BytesN<32>) -> Result<CredentialRecord, CredentialError>`

Get the full credential record.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `credential_id` | `BytesN<32>` | Yes | Credential to retrieve |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(record)` | `CredentialRecord` | Full credential record |
| `Err(NotFound)` | — | Credential does not exist |

---

##### `get_issuer_credentials(issuer: Address) -> Vec<BytesN<32>>`

List all credential IDs issued by a given issuer.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `issuer` | `Address` | Yes | Issuer address to query |

| Return | Type | Description |
|--------|------|-------------|
| `Vec<BytesN<32>>` | `Vec` | List of credential IDs (empty if none) |

---

##### `get_subject_credentials(subject: BytesN<32>) -> Vec<BytesN<32>>`

List all credential IDs held by a given subject DID.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `subject` | `BytesN<32>` | Yes | Subject DID to query |

| Return | Type | Description |
|--------|------|-------------|
| `Vec<BytesN<32>>` | `Vec` | List of credential IDs (empty if none) |

---

### KYC Verification

The KYC contract manages identity verification levels with a verifier registry and tiered approval system.

#### Storage Layout

```
persistent::Map<Address, KYCRecord>    // applicant → KYC record
persistent::Map<Address, bool>         // verifier → registered (true)
instance::DataKey::Admin               // contract admin address
```

#### KYCLevel

```rust
pub enum KYCLevel {
    Unverified = 0,     // No verification
    Basic = 1,          // Basic identity check
    Enhanced = 2,       // Enhanced due diligence
    Institutional = 3,  // Full institutional KYC
}
```

#### KYCStatus

```rust
pub enum KYCStatus {
    Pending = 0,    // Application submitted, awaiting review
    Approved = 1,   // Verification approved
    Rejected = 2,   // Verification rejected
    Expired = 3,    // Verification expired
}
```

#### KYCRecord

```rust
pub struct KYCRecord {
    pub did_id: BytesN<32>,      // Applicant's DID
    pub level: KYCLevel,         // Requested/approved level
    pub verifier: Address,       // Address of the verifier
    pub verified_at: u64,        // Timestamp of approval (0 if pending)
    pub expires_at: u64,         // Expiration timestamp (0 if pending)
    pub data_hash: Bytes,        // Hash of supporting documents
    pub status: KYCStatus,       // Current status
}
```

#### Functions

##### `submit_kyc(applicant, did_id, level, data_hash) -> Result<(), KYCError>`

Submit a KYC application. Sets status to Pending.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `applicant` | `Address` | Yes | Applicant's Stellar address. Requires auth. |
| `did_id` | `BytesN<32>` | Yes | Applicant's DID |
| `level` | `KYCLevel` | Yes | Target verification level (Basic, Enhanced, or Institutional) |
| `data_hash` | `Bytes` | Yes | Hash of supporting KYC documents |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(())` | — | Application submitted |
| `Err(AlreadyVerified)` | — | Active pending or approved application exists |
| `Err(InvalidLevel)` | — | Cannot submit for Unverified level |

**Events emitted:** `KYCSubmitted(timestamp)`

---

##### `approve_kyc(applicant, verifier, data_hash) -> Result<(), KYCError>`

Approve a KYC application. Only registered verifiers may call this.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `applicant` | `Address` | Yes | Applicant to approve |
| `verifier` | `Address` | Yes | Registered verifier. Requires auth. |
| `data_hash` | `Bytes` | Yes | Updated document hash |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(())` | — | KYC approved (valid for 1 year) |
| `Err(NotFound)` | — | No KYC application found |
| `Err(NotVerifier)` | — | Caller is not a registered verifier |
| `Err(NotPending)` | — | Application is not in pending status |

**Events emitted:** `KYCApproved(verifier, timestamp)`

---

##### `reject_kyc(applicant, verifier, reason) -> Result<(), KYCError>`

Reject a KYC application. Only registered verifiers may call this.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `applicant` | `Address` | Yes | Applicant to reject |
| `verifier` | `Address` | Yes | Registered verifier. Requires auth. |
| `reason` | `Bytes` | Yes | Rejection reason (for audit trail) |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(())` | — | KYC rejected |
| `Err(NotFound)` | — | No KYC application found |
| `Err(NotVerifier)` | — | Caller is not a registered verifier |
| `Err(NotPending)` | — | Application is not in pending status |

**Events emitted:** `KYCRejected(timestamp)`

---

##### `verify_kyc(applicant: Address) -> Result<KYCRecord, KYCError>`

Get the current KYC record for an applicant. Auto-expires approved records past their expiration.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `applicant` | `Address` | Yes | Applicant to query |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(record)` | `KYCRecord` | Full KYC record |
| `Err(NotFound)` | — | No KYC record found |
| `Err(Expired)` | — | KYC was approved but has expired |

---

##### `register_verifier(admin, verifier) -> Result<(), KYCError>`

Register a KYC verifier. Only the admin may call this. The first caller becomes the admin.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `admin` | `Address` | Yes | Admin address. Requires auth. |
| `verifier` | `Address` | Yes | Verifier to register |

| Return | Type | Description |
|--------|------|-------------|
| `Ok(())` | — | Verifier registered |
| `Err(NotAdmin)` | — | Caller is not the admin |

**Events emitted:** `VerifierRegistered(verifier, timestamp)`

```rust
// First caller becomes admin
contract.register_verifier(&admin, &admin);
// Now admin can register other verifiers
contract.register_verifier(&admin, &other_verifier);
```

---

##### `is_verified(applicant: Address, required_level: KYCLevel) -> bool`

Check if an applicant meets a minimum KYC level requirement.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `applicant` | `Address` | Yes | Applicant to check |
| `required_level` | `KYCLevel` | Yes | Minimum required level |

| Return | Type | Description |
|--------|------|-------------|
| `true` | `bool` | Applicant meets or exceeds the required level |
| `false` | `bool` | Not verified, wrong level, or expired |

```rust
assert!(contract.is_verified(&applicant, &KYCLevel::Basic));    // true
assert!(contract.is_verified(&applicant, &KYCLevel::Enhanced));  // true
assert!(!contract.is_verified(&applicant, &KYCLevel::Institutional)); // false
```

---

## Error Codes

### DID Errors

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotFound` | DID does not exist |
| 2 | `NotOwner` | Caller is not the DID owner |
| 3 | `Inactive` | DID has been deactivated |
| 4 | `AlreadyExists` | DID with this ID already exists |
| 5 | `InvalidDocument` | Document is empty |

### Credential Errors

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotFound` | Credential does not exist |
| 2 | `NotIssuer` | Caller is not the credential issuer |
| 3 | `AlreadyRevoked` | Credential already revoked |
| 4 | `Expired` | Credential has expired |
| 5 | `Revoked` | Credential has been revoked |
| 6 | `InvalidCredential` | Missing required fields or invalid expiry |

### KYC Errors

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotFound` | KYC record does not exist |
| 2 | `NotVerifier` | Caller is not a registered verifier |
| 3 | `NotAdmin` | Caller is not the admin |
| 4 | `AlreadyVerified` | Active KYC already exists |
| 5 | `NotPending` | KYC is not in pending status |
| 6 | `InvalidLevel` | Unverified level not allowed |
| 7 | `Expired` | KYC verification has expired |
| 8 | `LevelInsufficient` | KYC level below requirement |

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup) or [Stellar CLI](https://developers.stellar.org/docs/tools/stellar-cli)
- A Stellar account with testnet tokens (for deployment)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Soroban CLI
cargo install --locked soroban-cli

# Or install Stellar CLI
cargo install --locked stellar-cli

# Verify installation
soroban --version
# or
stellar --version
```

### Installation

```bash
# Clone the repository
git clone https://github.com/sudo-robi/web3-suite-identity-contracts.git
cd web3-suite-identity-contracts
```

### Building

```bash
# Build all contracts
cargo build

# Build for production (optimized WASM)
cargo build --release

# Build individual contracts
cargo build -p web3-suite-identity-did
cargo build -p web3-suite-identity-credentials
cargo build -p web3-suite-identity-kyc
```

The compiled WASM files will be in `target/release/`:
- `web3_suite_identity_did.wasm`
- `web3_suite_identity_credentials.wasm`
- `web3_suite_identity_kyc.wasm`

### Testing

```bash
# Run all tests (unit + integration)
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific contract tests
cargo test -p web3-suite-identity-did
cargo test -p web3-suite-identity-credentials
cargo test -p web3-suite-identity-kyc

# Run integration tests only
cargo test --test integration
```

### Deployment

```bash
# Generate a keypair (if you don't have one)
stellar keys generate my-keypair

# Fund the account on testnet
stellar keys fund my-keypair --network testnet

# Deploy DID Registry
stellar contract deploy \
  --wasm target/release/web3_suite_identity_did.wasm \
  --source my-keypair \
  --network testnet

# Deploy Verifiable Credentials
stellar contract deploy \
  --wasm target/release/web3_suite_identity_credentials.wasm \
  --source my-keypair \
  --network testnet

# Deploy KYC Verification
stellar contract deploy \
  --wasm target/release/web3_suite_identity_kyc.wasm \
  --source my-keypair \
  --network testnet

# Note the contract IDs from deployment output — you'll need them
# for the backend configuration.
```

---

## Configuration

The contracts themselves have no configuration — they are immutable once deployed. Configuration is handled at the backend layer via environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `CONTRACT_ID_DID` | Deployed DID Registry contract ID | `CBYK...3DQR` |
| `CONTRACT_ID_CREDENTIALS` | Deployed Credentials contract ID | `DCXL...7MNP` |
| `CONTRACT_ID_KYC` | Deployed KYC contract ID | `EDRW...9STU` |
| `SOROBAN_RPC_URL` | Soroban RPC endpoint | `https://soroban-testnet.stellar.org` |
| `STELLAR_NETWORK` | Network passphrase | `testnet` |

---

## Security Considerations

- **Authentication**: All state-changing functions require authentication via `require_auth()`. The Soroban runtime enforces cryptographic signature verification.

- **Authorization**: Only the DID owner can update, deactivate, or transfer their DID. Only credential issuers can revoke their credentials. Only registered verifiers can approve/reject KYC applications.

- **Input Validation**: All functions validate inputs — empty documents are rejected, expiry dates must be in the future, and level enums are validated.

- **Expiration**: KYC records auto-expire after 1 year. Credential expiry is enforced at verification time. Expired records return appropriate errors.

- **Immutability**: Smart contracts are immutable once deployed. State changes are recorded on-chain and cannot be retroactively modified.

- **Event Emission**: All state-changing operations emit events for off-chain indexing and audit trails.

- **No Reentrancy**: Soroban's execution model prevents reentrancy attacks by design — each contract invocation completes before the next begins.

- **Storage Isolation**: Each contract maintains its own storage namespace. Cross-contract reads are explicit and auditable.

---

## Contributing

### Branch Naming

| Prefix | Use Case |
|--------|----------|
| `feat/` | New features |
| `fix/` | Bug fixes |
| `docs/` | Documentation changes |
| `refactor/` | Code restructuring |
| `test/` | Adding or updating tests |
| `chore/` | Maintenance tasks |

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(did): add batch DID creation
fix(credentials): handle expired credential edge case
test(kyc): add verifier registration integration tests
docs: update deployment guide
```

### Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Write tests for new functionality
4. Ensure all tests pass (`cargo test`)
5. Ensure code compiles without warnings (`cargo build`)
6. Commit your changes (`git commit -m 'feat: add my feature'`)
7. Push to the branch (`git push origin feat/my-feature`)
8. Open a Pull Request with a clear description

### Code Standards

- All public functions must have doc comments
- All error variants must have descriptive names
- Tests must cover happy paths and error cases
- No `unwrap()` in production code — use `ok_or()` with proper error variants

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

---

## License

This project is licensed under the MIT License — see [LICENSE](LICENSE) for details.

---

Built with ❤️ for the Stellar ecosystem.
