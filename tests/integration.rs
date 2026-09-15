//! Cross-contract integration tests for the Identity Suite.
//!
//! These tests verify the full flow: DID creation -> Credential issuance -> KYC verification.

use soroban_sdk::{testutils::Address as _, Bytes, BytesN, Env};
use web3_suite_identity_did::DIDRegistry;
use web3_suite_identity_credentials::VerifiableCredentials;
use web3_suite_identity_kyc::KYCVerification;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct TestContext {
    env: Env,
    did_contract: web3_suite_identity_did::DIDRegistryClient<'static>,
    cred_contract: web3_suite_identity_credentials::VerifiableCredentialsClient<'static>,
    kyc_contract: web3_suite_identity_kyc::KYCVerificationClient<'static>,
    owner: soroban_sdk::Address,
    issuer: soroban_sdk::Address,
    admin: soroban_sdk::Address,
}

impl TestContext {
    fn new() -> Self {
        let env = Env::default();
        let owner = soroban_sdk::Address::generate(&env);
        let issuer = soroban_sdk::Address::generate(&env);
        let admin = soroban_sdk::Address::generate(&env);

        let did_id = env.register_contract(None, DIDRegistry);
        let cred_id = env.register_contract(None, VerifiableCredentials);
        let kyc_id = env.register_contract(None, KYCVerification);

        let did_contract = web3_suite_identity_did::DIDRegistryClient::new(&env, &did_id);
        let cred_contract =
            web3_suite_identity_credentials::VerifiableCredentialsClient::new(&env, &cred_id);
        let kyc_contract = web3_suite_identity_kyc::KYCVerificationClient::new(&env, &kyc_id);

        TestContext {
            env,
            did_contract,
            cred_contract,
            kyc_contract,
            owner,
            issuer,
            admin,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_full_identity_flow() {
    let ctx = TestContext::new();

    // Step 1: Create a DID
    let doc = Bytes::from_slice(&ctx.env, b"https://example.com/alice/did");
    let did_id = ctx.did_contract.create_did(&ctx.owner, &doc);
    assert!(ctx.did_contract.is_active(&did_id));

    // Step 2: Issue a credential linked to the DID
    let cred_type = Bytes::from_slice(&ctx.env, b"ProofOfIdentity");
    let claims = Bytes::from_slice(&ctx.env, b"{\"name\":\"Alice\",\"dob\":\"1990-01-01\"}");
    let sig = Bytes::from_slice(&ctx.env, b"issuer-sig-001");

    let cred_id = ctx.cred_contract.issue_credential(
        &ctx.issuer,
        &did_id,
        &cred_type,
        &claims,
        &None,
        &sig,
    );
    assert!(ctx.cred_contract.verify_credential(&cred_id));

    // Step 3: Register KYC verifier and submit KYC
    ctx.kyc_contract.register_verifier(&ctx.admin, &ctx.admin);

    let data_hash = Bytes::from_slice(&ctx.env, b"kyc-data-hash-abc");
    ctx.kyc_contract.submit_kyc(&ctx.owner, &did_id, &web3_suite_identity_kyc::KYCLevel::Enhanced, &data_hash);

    // Step 4: Approve KYC
    ctx.kyc_contract.approve_kyc(&ctx.owner, &ctx.admin, &data_hash);
    assert!(ctx.kyc_contract.is_verified(
        &ctx.owner,
        &web3_suite_identity_kyc::KYCLevel::Basic,
    ));
    assert!(ctx.kyc_contract.is_verified(
        &ctx.owner,
        &web3_suite_identity_kyc::KYCLevel::Enhanced,
    ));
    assert!(!ctx.kyc_contract.is_verified(
        &ctx.owner,
        &web3_suite_identity_kyc::KYCLevel::Institutional,
    ));
}

#[test]
fn test_did_deactivation_revokes_credential usefulness() {
    let ctx = TestContext::new();

    let doc = Bytes::from_slice(&ctx.env, b"doc");
    let did_id = ctx.did_contract.create_did(&ctx.owner, &doc);

    // Issue credential
    let cred_id = ctx.cred_contract.issue_credential(
        &ctx.issuer,
        &did_id,
        &Bytes::from_slice(&ctx.env, b"type"),
        &Bytes::from_slice(&ctx.env, b"claims"),
        &None,
        &Bytes::from_slice(&ctx.env, b"sig"),
    );
    assert!(ctx.cred_contract.verify_credential(&cred_id));

    // Deactivate DID
    ctx.did_contract.deactivate_did(&did_id, &ctx.owner);
    assert!(!ctx.did_contract.is_active(&did_id));

    // Credential still technically valid (independent systems)
    // but DID is inactive — app layer should check both
    assert!(ctx.cred_contract.verify_credential(&cred_id));
}

#[test]
fn test_multiple_credentials_per_did() {
    let ctx = TestContext::new();

    let doc = Bytes::from_slice(&ctx.env, b"doc");
    let did_id = ctx.did_contract.create_did(&ctx.owner, &doc);

    let _c1 = ctx.cred_contract.issue_credential(
        &ctx.issuer,
        &did_id,
        &Bytes::from_slice(&ctx.env, b"type1"),
        &Bytes::from_slice(&ctx.env, b"claims1"),
        &None,
        &Bytes::from_slice(&ctx.env, b"sig1"),
    );

    let _c2 = ctx.cred_contract.issue_credential(
        &ctx.issuer,
        &did_id,
        &Bytes::from_slice(&ctx.env, b"type2"),
        &Bytes::from_slice(&ctx.env, b"claims2"),
        &None,
        &Bytes::from_slice(&ctx.env, b"sig2"),
    );

    let issuer_creds = ctx.cred_contract.get_issuer_credentials(&ctx.issuer);
    assert_eq!(issuer_creds.len(), 2);

    let subject_creds = ctx.cred_contract.get_subject_credentials(&did_id);
    assert_eq!(subject_creds.len(), 2);
}

#[test]
fn test_credential_revocation_invalidation() {
    let ctx = TestContext::new();

    let doc = Bytes::from_slice(&ctx.env, b"doc");
    let did_id = ctx.did_contract.create_did(&ctx.owner, &doc);

    let cred_id = ctx.cred_contract.issue_credential(
        &ctx.issuer,
        &did_id,
        &Bytes::from_slice(&ctx.env, b"type"),
        &Bytes::from_slice(&ctx.env, b"claims"),
        &None,
        &Bytes::from_slice(&ctx.env, b"sig"),
    );

    assert!(ctx.cred_contract.verify_credential(&cred_id));

    ctx.cred_contract.revoke_credential(&cred_id, &ctx.issuer);
    assert!(!ctx.cred_contract.verify_credential(&cred_id));

    // KYC should also fail if credential is revoked (app layer logic)
}
