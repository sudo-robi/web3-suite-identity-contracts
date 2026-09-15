[![Stellar](https://img.shields.io/badge/Built%20on-Stellar-08B5E5?style=for-the-badge&logo=stellar&logoColor=white)](https://stellar.org)
[![Soroban](https://img.shields.io/badge/Smart%20Contracts-Soroban-08B5E5?style=for-the-badge)](https://soroban.stellar.org)
[![Rust](https://img.shields.io/badge/Language-Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![Tests](https://img.shields.io/badge/Tests-Passing-brightgreen?style=for-the-badge)](#testing)
[![CI](https://img.shields.io/badge/CI-GitHub%20Actions-blue?style=for-the-badge&logo=githubactions&logoColor=white)](#)

# Web3 Suite — Identity Contracts

Soroban smart contracts for decentralized identity primitives on the Stellar network. This workspace provides three composable contracts that together enable a complete on-chain identity layer: **DID Registry**, **Verifiable Credentials**, and **KYC Verification**.

---

## Table of Contents

- [Architecture](#architecture)
- [Contracts](#contracts)
  - [DID Registry](#did-registry)
  - [Verifiable Credentials](#verifiable-credentials)
  - [KYC Verification](#kyc-verification)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Build](#build)
  - [Test](#test)
  - [Deploy](#deploy)
- [Project Structure](#project-structure)
- [Error Codes](#error-codes)
- [Security Considerations](#security-considerations)
- [Contributing](#contributing)
- [License](#license)

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
│  │  deactivate_did()│  │  get_credential() │  │  is_verified()   │  │
│  │  transfer_did()  │  │  issuer/subject  │  │  register_verify │  │
│  │  is_active()     │  │  credential lists│  │                  │  │
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

---

## Contracts

### DID Registry

The DID Registry contract manages Decentralized Identifiers on-chain. Each DID is a unique 32-byte identifier derived from a SHA-256 hash of the owner address, a counter, and the initial document.

| Function | Description | Access |
|----------|-------------|--------|
| `create_did(owner, document)` | Create a new DID | Owner (auth) |
| `resolve_did(did_id)` | Resolve DID to its record | Public |
| `update_did(did_id, caller, new_document)` | Update DID document | Owner (auth) |
| `deactivate_did(did_id, caller)` | Deactivate a DID | Owner (auth) |
| `transfer_did(did_id, caller, new_owner)` | Transfer ownership | Owner (auth) |
| `is_active(did_id)` | Check if DID is active | Public |

**Storage:** `persistent::Map<BytesN<32>, DIDRecord>`

**Events:** `DIDCreated`, `DIDUpdated`, `DIDDeactivated`, `DIDTransferred`

---

### Verifiable Credentials

The Credentials contract enables issuance, verification, and revocation of credentials linked to DIDs. Credentials carry typed claims, optional expiry, and cryptographic signatures.

| Function | Description | Access |
|----------|-------------|--------|
| `issue_credential(issuer, subject, type, claims, expires, sig)` | Issue a credential | Issuer (auth) |
| `verify_credential(credential_id)` | Check if credential is valid | Public |
| `revoke_credential(credential_id, caller)` | Revoke a credential | Issuer (auth) |
| `get_credential(credential_id)` | Get full credential record | Public |
| `get_issuer_credentials(issuer)` | List credentials by issuer | Public |
| `get_subject_credentials(subject)` | List credentials by subject | Public |

**Storage:** `persistent::Map<BytesN<32>, CredentialRecord>` + issuer/subject index maps

**Events:** `CredentialIssued`, `CredentialRevoked`

---

### KYC Verification

The KYC contract manages identity verification levels with a verifier registry and tiered approval system.

| Level | Description |
|-------|-------------|
| `Unverified` | No verification (0) |
| `Basic` | Basic identity check (1) |
| `Enhanced` | Enhanced due diligence (2) |
| `Institutional` | Full institutional KYC (3) |

| Function | Description | Access |
|----------|-------------|--------|
| `submit_kyc(applicant, did_id, level, data_hash)` | Submit KYC application | Applicant (auth) |
| `approve_kyc(applicant, verifier, data_hash)` | Approve KYC | Verifier (auth) |
| `reject_kyc(applicant, verifier, reason)` | Reject KYC | Verifier (auth) |
| `verify_kyc(applicant)` | Get KYC record | Public |
| `register_verifier(admin, verifier)` | Register a verifier | Admin (auth) |
| `is_verified(applicant, required_level)` | Check verification level | Public |

**Storage:** `persistent::Map<Address, KYCRecord>` + verifier registry

**Events:** `KYCSubmitted`, `KYCApproved`, `KYCRejected`, `VerifierRegistered`

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup)
- [Stellar CLI](https://developers.stellar.org/docs/tools/stellar-cli)

```bash
# Install Soroban CLI
cargo install --locked soroban-cli

# Or install Stellar CLI
cargo install --locked stellar-cli
```

### Build

```bash
# Build all contracts
cargo build

# Build for production (optimized)
cargo build --release

# Build individual contracts
cargo build -p web3-suite-identity-did
cargo build -p web3-suite-identity-credentials
cargo build -p web3-suite-identity-kyc
```

### Test

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific contract tests
cargo test -p web3-suite-identity-did
cargo test -p web3-suite-identity-credentials
cargo test -p web3-suite-identity-kyc
```

### Deploy

```bash
# Deploy DID Registry
stellar contract deploy \
  --wasm target/release/web3_suite_identity_did.wasm \
  --source your-keypair \
  --network testnet

# Deploy Verifiable Credentials
stellar contract deploy \
  --wasm target/release/web3_suite_identity_credentials.wasm \
  --source your-keypair \
  --network testnet

# Deploy KYC Verification
stellar contract deploy \
  --wasm target/release/web3_suite_identity_kyc.wasm \
  --source your-keypair \
  --network testnet
```

---

## Project Structure

```
web3-suite-identity-contracts/
├── Cargo.toml                      # Workspace configuration
├── contracts/
│   ├── did/                        # DID Registry contract
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── credentials/                # Verifiable Credentials contract
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── kyc/                        # KYC Verification contract
│       ├── Cargo.toml
│       └── src/lib.rs
├── tests/
│   └── integration.rs              # Cross-contract integration tests
├── LICENSE
├── CONTRIBUTING.md
└── README.md
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
| 6 | `InvalidCredential` | Missing required fields |

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

## Security Considerations

- All state-changing functions require authentication via `require_auth()`
- Only credential issuers can revoke their own credentials
- Only registered verifiers can approve/reject KYC applications
- Only the DID owner can update, deactivate, or transfer their DID
- KYC records auto-expire after 1 year
- Credential expiry is enforced at verification time

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Commit your changes (`git commit -am 'Add my feature'`)
4. Push to the branch (`git push origin feat/my-feature`)
5. Open a Pull Request

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

---

## License

This project is licensed under the MIT License — see [LICENSE](LICENSE) for details.

---

Built with ❤️ for the Stellar ecosystem.
