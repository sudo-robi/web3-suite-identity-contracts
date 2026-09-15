#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Bytes, BytesN, Env, Symbol, Vec};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KYCLevel {
    Unverified = 0,
    Basic = 1,
    Enhanced = 2,
    Institutional = 3,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KYCStatus {
    Pending = 0,
    Approved = 1,
    Rejected = 2,
    Expired = 3,
}

#[contracttype]
pub struct KYCRecord {
    pub did_id: BytesN<32>,
    pub level: KYCLevel,
    pub verifier: Address,
    pub verified_at: u64,
    pub expires_at: u64,
    pub data_hash: Bytes,
    pub status: KYCStatus,
}

#[contracttype]
pub enum DataKey {
    KYCRecord(Address),
    Verifier(Address),
    Admin,
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KYCError {
    NotFound = 1,
    NotVerifier = 2,
    NotAdmin = 3,
    AlreadyVerified = 4,
    NotPending = 5,
    InvalidLevel = 6,
    Expired = 7,
    LevelInsufficient = 8,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct KYCVerification;

#[contractimpl]
impl KYCVerification {
    /// Submit a KYC application. Sets status to Pending.
    pub fn submit_kyc(
        env: Env,
        applicant: Address,
        did_id: BytesN<32>,
        level: KYCLevel,
        data_hash: Bytes,
    ) -> Result<(), KYCError> {
        applicant.require_auth();

        // Must not already have an active pending or approved application
        if let Some(existing) = env
            .storage()
            .persistent()
            .get::<DataKey, KYCRecord>(&DataKey::KYCRecord(applicant.clone()))
        {
            if existing.status == KYCStatus::Pending || existing.status == KYCStatus::Approved {
                return Err(KYCError::AlreadyVerified);
            }
        }

        if level == KYCLevel::Unverified {
            return Err(KYCError::InvalidLevel);
        }

        let now = env.ledger().timestamp();
        let record = KYCRecord {
            did_id,
            level,
            verifier: applicant.clone(),
            verified_at: 0,
            expires_at: 0,
            data_hash,
            status: KYCStatus::Pending,
        };

        env.storage()
            .persistent()
            .set(&DataKey::KYCRecord(applicant), &record);

        env.events().publish(
            Symbol::new(&env, "KYCSubmitted"),
            (now,),
        );

        Ok(())
    }

    /// Approve a KYC application. Only registered verifiers may call this.
    pub fn approve_kyc(
        env: Env,
        applicant: Address,
        verifier: Address,
        data_hash: Bytes,
    ) -> Result<(), KYCError> {
        verifier.require_auth();

        // Check verifier is registered
        if !env
            .storage()
            .persistent()
            .has(&DataKey::Verifier(verifier.clone()))
        {
            return Err(KYCError::NotVerifier);
        }

        let mut record: KYCRecord = env
            .storage()
            .persistent()
            .get(&DataKey::KYCRecord(applicant.clone()))
            .ok_or(KYCError::NotFound)?;

        if record.status != KYCStatus::Pending {
            return Err(KYCError::NotPending);
        }

        let now = env.ledger().timestamp();
        record.status = KYCStatus::Approved;
        record.verifier = verifier.clone();
        record.verified_at = now;
        record.expires_at = now + 365 * 24 * 60 * 60; // 1 year validity
        record.data_hash = data_hash;

        env.storage()
            .persistent()
            .set(&DataKey::KYCRecord(applicant), &record);

        env.events().publish(
            Symbol::new(&env, "KYCApproved"),
            (verifier, now),
        );

        Ok(())
    }

    /// Reject a KYC application. Only registered verifiers may call this.
    pub fn reject_kyc(
        env: Env,
        applicant: Address,
        verifier: Address,
        _reason: Bytes,
    ) -> Result<(), KYCError> {
        verifier.require_auth();

        if !env
            .storage()
            .persistent()
            .has(&DataKey::Verifier(verifier.clone()))
        {
            return Err(KYCError::NotVerifier);
        }

        let mut record: KYCRecord = env
            .storage()
            .persistent()
            .get(&DataKey::KYCRecord(applicant.clone()))
            .ok_or(KYCError::NotFound)?;

        if record.status != KYCStatus::Pending {
            return Err(KYCError::NotPending);
        }

        record.status = KYCStatus::Rejected;
        record.verifier = verifier;

        env.storage()
            .persistent()
            .set(&DataKey::KYCRecord(applicant), &record);

        env.events().publish(
            Symbol::new(&env, "KYCRejected"),
            (env.ledger().timestamp(),),
        );

        Ok(())
    }

    /// Get the current KYC record for an applicant.
    pub fn verify_kyc(env: Env, applicant: Address) -> Result<KYCRecord, KYCError> {
        let record: KYCRecord = env
            .storage()
            .persistent()
            .get(&DataKey::KYCRecord(applicant))
            .ok_or(KYCError::NotFound)?;

        // Auto-expire if past expires_at
        if record.status == KYCStatus::Approved
            && env.ledger().timestamp() >= record.expires_at
        {
            return Err(KYCError::Expired);
        }

        Ok(record)
    }

    /// Register a KYC verifier. Only the admin may call this.
    pub fn register_verifier(env: Env, admin: Address, verifier: Address) -> Result<(), KYCError> {
        admin.require_auth();

        let stored_admin: Option<Address> = env.storage().instance().get(&DataKey::Admin);
        match stored_admin {
            Some(a) if a == admin => {}
            None => {
                // First caller becomes admin
                env.storage().instance().set(&DataKey::Admin, &admin);
            }
            _ => return Err(KYCError::NotAdmin),
        }

        env.storage()
            .persistent()
            .set(&DataKey::Verifier(verifier.clone()), &true);

        env.events().publish(
            Symbol::new(&env, "VerifierRegistered"),
            (verifier, env.ledger().timestamp()),
        );

        Ok(())
    }

    /// Check if an applicant meets a minimum KYC level requirement.
    pub fn is_verified(env: Env, applicant: Address, required_level: KYCLevel) -> bool {
        let record: KYCRecord = match env
            .storage()
            .persistent()
            .get(&DataKey::KYCRecord(applicant))
        {
            Some(r) => r,
            None => return false,
        };

        if record.status != KYCStatus::Approved {
            return false;
        }

        if env.ledger().timestamp() >= record.expires_at {
            return false;
        }

        // Check level meets requirement
        (record.level as u32) >= (required_level as u32)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup() -> (Env, Address, Address, BytesN<32>) {
        let env = Env::default();
        let admin = Address::generate(&env);
        let applicant = Address::generate(&env);
        let did_id = env.crypto().sha256(&Bytes::from_slice(&env, b"did123"));
        (env, admin, applicant, did_id)
    }

    #[test]
    fn test_full_kyc_flow() {
        let (env, admin, applicant, did_id) = setup();
        let contract = KYCVerificationClient::new(
            &env,
            &env.register_contract(None, KYCVerification),
        );

        // Register admin as verifier (first caller)
        contract.register_verifier(&admin, &admin);

        // Submit
        contract.submit_kyc(
            &applicant,
            &did_id,
            &KYCLevel::Enhanced,
            &Bytes::from_slice(&env, b"hash123"),
        );

        let record = contract.verify_kyc(&applicant);
        assert_eq!(record.status, KYCStatus::Pending);

        // Approve
        contract.approve_kyc(
            &applicant,
            &admin,
            &Bytes::from_slice(&env, b"hash123"),
        );

        assert!(contract.is_verified(&applicant, &KYCLevel::Basic));
        assert!(contract.is_verified(&applicant, &KYCLevel::Enhanced));
        assert!(!contract.is_verified(&applicant, &KYCLevel::Institutional));
    }

    #[test]
    fn test_reject_kyc() {
        let (env, admin, applicant, did_id) = setup();
        let contract = KYCVerificationClient::new(
            &env,
            &env.register_contract(None, KYCVerification),
        );

        contract.register_verifier(&admin, &admin);
        contract.submit_kyc(
            &applicant,
            &did_id,
            &KYCLevel::Basic,
            &Bytes::from_slice(&env, b"hash"),
        );

        contract.reject_kyc(
            &applicant,
            &admin,
            &Bytes::from_slice(&env, b"insufficient docs"),
        );

        assert!(!contract.is_verified(&applicant, &KYCLevel::Basic));
    }

    #[test]
    fn test_unregistered_verifier_fails() {
        let (env, admin, applicant, did_id) = setup();
        let fake_verifier = Address::generate(&env);
        let contract = KYCVerificationClient::new(
            &env,
            &env.register_contract(None, KYCVerification),
        );

        contract.register_verifier(&admin, &admin);
        contract.submit_kyc(
            &applicant,
            &did_id,
            &KYCLevel::Basic,
            &Bytes::from_slice(&env, b"hash"),
        );

        let result = contract.try_approve_kyc(
            &applicant,
            &fake_verifier,
            &Bytes::from_slice(&env, b"hash"),
        );
        assert_eq!(result, Err(Err(KYCError::NotVerifier)));
    }

    #[test]
    fn test_invalid_level() {
        let (env, _, applicant, did_id) = setup();
        let contract = KYCVerificationClient::new(
            &env,
            &env.register_contract(None, KYCVerification),
        );

        let result = contract.try_submit_kyc(
            &applicant,
            &did_id,
            &KYCLevel::Unverified,
            &Bytes::from_slice(&env, b"hash"),
        );
        assert_eq!(result, Err(Err(KYCError::InvalidLevel)));
    }
}
