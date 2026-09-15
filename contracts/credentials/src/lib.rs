#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Bytes, BytesN, Env, Symbol, Vec};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[contracttype]
pub struct CredentialRecord {
    pub issuer: Address,
    pub subject: BytesN<32>,
    pub credential_type: Bytes,
    pub claims: Bytes,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
    pub revoked: bool,
    pub signature: Bytes,
}

#[contracttype]
pub enum DataKey {
    Credential(BytesN<32>),
    IssuerCredentials(Address),
    SubjectCredentials(BytesN<32>),
    Counter,
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialError {
    NotFound = 1,
    NotIssuer = 2,
    AlreadyRevoked = 3,
    Expired = 4,
    Revoked = 5,
    InvalidCredential = 6,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct VerifiableCredentials;

#[contractimpl]
impl VerifiableCredentials {
    /// Issue a new verifiable credential. Returns the credential ID.
    pub fn issue_credential(
        env: Env,
        issuer: Address,
        subject: BytesN<32>,
        credential_type: Bytes,
        claims: Bytes,
        expires_at: Option<u64>,
        signature: Bytes,
    ) -> Result<BytesN<32>, CredentialError> {
        if credential_type.is_empty() || claims.is_empty() {
            return Err(CredentialError::InvalidCredential);
        }

        issuer.require_auth();

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Counter)
            .unwrap_or(0);

        // Generate credential ID
        let mut preimage = Bytes::new(&env);
        preimage.extend_from_slice(&issuer.to_buffer());
        preimage.extend_from_slice(&subject);
        preimage.extend_from_slice(&counter.to_be_bytes());
        preimage.extend_from_slice(&credential_type);
        let credential_id = env.crypto().sha256(&preimage).into();

        let now = env.ledger().timestamp();

        // Validate expiry
        if let Some(exp) = expires_at {
            if exp <= now {
                return Err(CredentialError::InvalidCredential);
            }
        }

        let record = CredentialRecord {
            issuer: issuer.clone(),
            subject,
            credential_type,
            claims,
            issued_at: now,
            expires_at,
            revoked: false,
            signature,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Credential(credential_id), &record);

        // Add to issuer's list
        let mut issuer_creds: Vec<BytesN<32>> = env
            .storage()
            .persistent()
            .get(&DataKey::IssuerCredentials(issuer.clone()))
            .unwrap_or(Vec::new(&env));
        issuer_creds.push_back(credential_id);
        env.storage().persistent().set(
            &DataKey::IssuerCredentials(issuer),
            &issuer_creds,
        );

        // Add to subject's list
        let mut subject_creds: Vec<BytesN<32>> = env
            .storage()
            .persistent()
            .get(&DataKey::SubjectCredentials(record.subject.clone()))
            .unwrap_or(Vec::new(&env));
        subject_creds.push_back(credential_id);
        env.storage().persistent().set(
            &DataKey::SubjectCredentials(record.subject),
            &subject_creds,
        );

        // Increment counter
        env.storage()
            .instance()
            .set(&DataKey::Counter, &(counter + 1));

        // Emit event
        env.events().publish(
            Symbol::new(&env, "CredentialIssued"),
            (credential_id, record.issuer, record.issued_at),
        );

        Ok(credential_id)
    }

    /// Verify that a credential is valid (exists, not revoked, not expired).
    pub fn verify_credential(env: Env, credential_id: BytesN<32>) -> bool {
        let record: CredentialRecord = match env
            .storage()
            .persistent()
            .get(&DataKey::Credential(credential_id))
        {
            Some(r) => r,
            None => return false,
        };

        if record.revoked {
            return false;
        }

        if let Some(exp) = record.expires_at {
            if env.ledger().timestamp() >= exp {
                return false;
            }
        }

        true
    }

    /// Revoke a credential. Only the issuer may call this.
    pub fn revoke_credential(
        env: Env,
        credential_id: BytesN<32>,
        caller: Address,
    ) -> Result<(), CredentialError> {
        caller.require_auth();

        let mut record: CredentialRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Credential(credential_id))
            .ok_or(CredentialError::NotFound)?;

        if record.issuer != caller {
            return Err(CredentialError::NotIssuer);
        }

        if record.revoked {
            return Err(CredentialError::AlreadyRevoked);
        }

        record.revoked = true;
        env.storage()
            .persistent()
            .set(&DataKey::Credential(credential_id), &record);

        env.events().publish(
            Symbol::new(&env, "CredentialRevoked"),
            (credential_id, env.ledger().timestamp()),
        );

        Ok(())
    }

    /// Get the full credential record.
    pub fn get_credential(env: Env, credential_id: BytesN<32>) -> Result<CredentialRecord, CredentialError> {
        env.storage()
            .persistent()
            .get(&DataKey::Credential(credential_id))
            .ok_or(CredentialError::NotFound)
    }

    /// List all credential IDs issued by a given issuer.
    pub fn get_issuer_credentials(env: Env, issuer: Address) -> Vec<BytesN<32>> {
        env.storage()
            .persistent()
            .get(&DataKey::IssuerCredentials(issuer))
            .unwrap_or(Vec::new(&env))
    }

    /// List all credential IDs issued to a given subject.
    pub fn get_subject_credentials(env: Env, subject: BytesN<32>) -> Vec<BytesN<32>> {
        env.storage()
            .persistent()
            .get(&DataKey::SubjectCredentials(subject))
            .unwrap_or(Vec::new(&env))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup() -> (Env, Address, BytesN<32>) {
        let env = Env::default();
        let issuer = Address::generate(&env);
        let subject = env.crypto().sha256(&Bytes::from_slice(&env, b"subject-did"));
        (env, issuer, subject)
    }

    #[test]
    fn test_issue_and_verify() {
        let (env, issuer, subject) = setup();
        let contract = VerifiableCredentialsClient::new(
            &env,
            &env.register_contract(None, VerifiableCredentials),
        );

        let cred_type = Bytes::from_slice(&env, b"ProofOfIdentity");
        let claims = Bytes::from_slice(&env, b"{\"name\":\"Alice\"}");
        let sig = Bytes::from_slice(&env, b"sig123");

        let cred_id = contract.issue_credential(
            &issuer,
            &subject,
            &cred_type,
            &claims,
            &None,
            &sig,
        );

        assert!(contract.verify_credential(&cred_id));
        let record = contract.get_credential(&cred_id);
        assert_eq!(record.issuer, issuer);
        assert!(!record.revoked);
    }

    #[test]
    fn test_revoke_credential() {
        let (env, issuer, subject) = setup();
        let contract = VerifiableCredentialsClient::new(
            &env,
            &env.register_contract(None, VerifiableCredentials),
        );

        let cred_id = contract.issue_credential(
            &issuer,
            &subject,
            &Bytes::from_slice(&env, b"type"),
            &Bytes::from_slice(&env, b"claims"),
            &None,
            &Bytes::from_slice(&env, b"sig"),
        );

        assert!(contract.verify_credential(&cred_id));
        contract.revoke_credential(&cred_id, &issuer);
        assert!(!contract.verify_credential(&cred_id));
    }

    #[test]
    fn test_issuer_credentials_list() {
        let (env, issuer, subject) = setup();
        let contract = VerifiableCredentialsClient::new(
            &env,
            &env.register_contract(None, VerifiableCredentials),
        );

        let _id1 = contract.issue_credential(
            &issuer,
            &subject,
            &Bytes::from_slice(&env, b"type1"),
            &Bytes::from_slice(&env, b"c1"),
            &None,
            &Bytes::from_slice(&env, b"s1"),
        );

        let _id2 = contract.issue_credential(
            &issuer,
            &subject,
            &Bytes::from_slice(&env, b"type2"),
            &Bytes::from_slice(&env, b"c2"),
            &None,
            &Bytes::from_slice(&env, b"s2"),
        );

        let creds = contract.get_issuer_credentials(&issuer);
        assert_eq!(creds.len(), 2);
    }

    #[test]
    fn test_not_issuer_fails_revoke() {
        let (env, issuer, subject) = setup();
        let other = Address::generate(&env);
        let contract = VerifiableCredentialsClient::new(
            &env,
            &env.register_contract(None, VerifiableCredentials),
        );

        let cred_id = contract.issue_credential(
            &issuer,
            &subject,
            &Bytes::from_slice(&env, b"type"),
            &Bytes::from_slice(&env, b"claims"),
            &None,
            &Bytes::from_slice(&env, b"sig"),
        );

        let result = contract.try_revoke_credential(&cred_id, &other);
        assert_eq!(result, Err(Err(CredentialError::NotIssuer)));
    }

    #[test]
    fn test_invalid_credential_empty_type() {
        let (env, issuer, subject) = setup();
        let contract = VerifiableCredentialsClient::new(
            &env,
            &env.register_contract(None, VerifiableCredentials),
        );

        let result = contract.try_issue_credential(
            &issuer,
            &subject,
            &Bytes::new(&env),
            &Bytes::from_slice(&env, b"claims"),
            &None,
            &Bytes::from_slice(&env, b"sig"),
        );
        assert_eq!(result, Err(Err(CredentialError::InvalidCredential)));
    }

    #[test]
    fn test_nonexistent_credential() {
        let (env, _, _) = setup();
        let contract = VerifiableCredentialsClient::new(
            &env,
            &env.register_contract(None, VerifiableCredentials),
        );

        let fake_id = env.crypto().sha256(&Bytes::from_slice(&env, b"nope"));
        assert!(!contract.verify_credential(&fake_id.into()));
    }
}
