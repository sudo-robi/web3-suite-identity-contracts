#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Bytes, BytesN, Env, Symbol};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[contracttype]
pub struct DIDRecord {
    pub owner: Address,
    pub document: Bytes,
    pub created_at: u64,
    pub updated_at: u64,
    pub active: bool,
}

#[contracttype]
pub enum DataKey {
    DID(BytesN<32>),
    Counter,
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DIDError {
    NotFound = 1,
    NotOwner = 2,
    Inactive = 3,
    AlreadyExists = 4,
    InvalidDocument = 5,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct DIDCreatedEvent {
    pub did_id: BytesN<32>,
    pub owner: Address,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct DIDUpdatedEvent {
    pub did_id: BytesN<32>,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct DIDDeactivatedEvent {
    pub did_id: BytesN<32>,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct DIDTransferredEvent {
    pub did_id: BytesN<32>,
    pub from: Address,
    pub to: Address,
    pub timestamp: u64,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct DIDRegistry;

#[contractimpl]
impl DIDRegistry {
    /// Create a new DID with a unique ID derived from owner + document hash.
    /// Returns the 32-byte DID identifier.
    pub fn create_did(env: Env, owner: Address, document: Bytes) -> Result<BytesN<32>, DIDError> {
        if document.is_empty() {
            return Err(DIDError::InvalidDocument);
        }

        owner.require_auth();

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Counter)
            .unwrap_or(0);

        // Generate unique ID: sha256(owner || counter || document)
        let mut preimage = Bytes::new(&env);
        preimage.extend_from_slice(&owner.to_buffer());
        preimage.extend_from_slice(&counter.to_be_bytes());
        preimage.extend_from_slice(&document);
        let did_id = env.crypto().sha256(&preimage).into();

        // Check not already exists
        if env.storage().persistent().has(&DataKey::DID(did_id)) {
            return Err(DIDError::AlreadyExists);
        }

        let now = env.ledger().timestamp();
        let record = DIDRecord {
            owner: owner.clone(),
            document,
            created_at: now,
            updated_at: now,
            active: true,
        };

        env.storage().persistent().set(&DataKey::DID(did_id), &record);
        env.storage()
            .instance()
            .set(&DataKey::Counter, &(counter + 1));

        // Emit event
        env.events().publish(
            Symbol::new(&env, "DIDCreated"),
            (did_id.clone(), owner, now),
        );

        Ok(did_id)
    }

    /// Resolve a DID to its full record.
    pub fn resolve_did(env: Env, did_id: BytesN<32>) -> Result<DIDRecord, DIDError> {
        env.storage()
            .persistent()
            .get(&DataKey::DID(did_id))
            .ok_or(DIDError::NotFound)
    }

    /// Update a DID's document. Only the owner may call this.
    pub fn update_did(
        env: Env,
        did_id: BytesN<32>,
        caller: Address,
        new_document: Bytes,
    ) -> Result<(), DIDError> {
        if new_document.is_empty() {
            return Err(DIDError::InvalidDocument);
        }

        caller.require_auth();

        let mut record: DIDRecord = env
            .storage()
            .persistent()
            .get(&DataKey::DID(did_id))
            .ok_or(DIDError::NotFound)?;

        if !record.active {
            return Err(DIDError::Inactive);
        }
        if record.owner != caller {
            return Err(DIDError::NotOwner);
        }

        record.document = new_document;
        record.updated_at = env.ledger().timestamp();

        env.storage().persistent().set(&DataKey::DID(did_id), &record);

        env.events().publish(
            Symbol::new(&env, "DIDUpdated"),
            (did_id, record.updated_at),
        );

        Ok(())
    }

    /// Deactivate a DID. Only the owner may call this.
    pub fn deactivate_did(env: Env, did_id: BytesN<32>, caller: Address) -> Result<(), DIDError> {
        caller.require_auth();

        let mut record: DIDRecord = env
            .storage()
            .persistent()
            .get(&DataKey::DID(did_id))
            .ok_or(DIDError::NotFound)?;

        if record.owner != caller {
            return Err(DIDError::NotOwner);
        }

        record.active = false;
        record.updated_at = env.ledger().timestamp();

        env.storage().persistent().set(&DataKey::DID(did_id), &record);

        env.events().publish(
            Symbol::new(&env, "DIDDeactivated"),
            (did_id, record.updated_at),
        );

        Ok(())
    }

    /// Transfer DID ownership to a new address. Only the owner may call this.
    pub fn transfer_did(
        env: Env,
        did_id: BytesN<32>,
        caller: Address,
        new_owner: Address,
    ) -> Result<(), DIDError> {
        caller.require_auth();

        let mut record: DIDRecord = env
            .storage()
            .persistent()
            .get(&DataKey::DID(did_id))
            .ok_or(DIDError::NotFound)?;

        if !record.active {
            return Err(DIDError::Inactive);
        }
        if record.owner != caller {
            return Err(DIDError::NotOwner);
        }

        let old_owner = record.owner.clone();
        record.owner = new_owner.clone();
        record.updated_at = env.ledger().timestamp();

        env.storage().persistent().set(&DataKey::DID(did_id), &record);

        env.events().publish(
            Symbol::new(&env, "DIDTransferred"),
            (did_id, old_owner, new_owner, record.updated_at),
        );

        Ok(())
    }

    /// Check if a DID is currently active.
    pub fn is_active(env: Env, did_id: BytesN<32>) -> bool {
        env.storage()
            .persistent()
            .get::<DataKey, DIDRecord>(&DataKey::DID(did_id))
            .map(|r| r.active)
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        let owner = Address::generate(&env);
        let other = Address::generate(&env);
        (env, owner, other)
    }

    #[test]
    fn test_create_and_resolve() {
        let (env, owner, _) = setup();
        let contract = DIDRegistryClient::new(&env, &env.register_contract(None, DIDRegistry));

        let doc = Bytes::from_slice(&env, b"https://example.com/did/doc1");
        let did_id = contract.create_did(&owner, &doc);

        let record = contract.resolve_did(&did_id);
        assert_eq!(record.owner, owner);
        assert_eq!(record.document, doc);
        assert!(record.active);
    }

    #[test]
    fn test_update_did() {
        let (env, owner, _) = setup();
        let contract = DIDRegistryClient::new(&env, &env.register_contract(None, DIDRegistry));

        let doc = Bytes::from_slice(&env, b"original");
        let did_id = contract.create_did(&owner, &doc);

        let new_doc = Bytes::from_slice(&env, b"updated");
        contract.update_did(&did_id, &owner, &new_doc);

        let record = contract.resolve_did(&did_id);
        assert_eq!(record.document, new_doc);
    }

    #[test]
    fn test_deactivate_did() {
        let (env, owner, _) = setup();
        let contract = DIDRegistryClient::new(&env, &env.register_contract(None, DIDRegistry));

        let doc = Bytes::from_slice(&env, b"doc");
        let did_id = contract.create_did(&owner, &doc);

        assert!(contract.is_active(&did_id));
        contract.deactivate_did(&did_id, &owner);
        assert!(!contract.is_active(&did_id));
    }

    #[test]
    fn test_transfer_did() {
        let (env, owner, other) = setup();
        let contract = DIDRegistryClient::new(&env, &env.register_contract(None, DIDRegistry));

        let doc = Bytes::from_slice(&env, b"doc");
        let did_id = contract.create_did(&owner, &doc);

        contract.transfer_did(&did_id, &owner, &other);
        let record = contract.resolve_did(&did_id);
        assert_eq!(record.owner, other);
    }

    #[test]
    fn test_not_owner_fails() {
        let (env, owner, other) = setup();
        let contract = DIDRegistryClient::new(&env, &env.register_contract(None, DIDRegistry));

        let doc = Bytes::from_slice(&env, b"doc");
        let did_id = contract.create_did(&owner, &doc);

        let result = contract.try_update_did(&did_id, &other, &Bytes::from_slice(&env, b"x"));
        assert_eq!(result, Err(Err(DIDError::NotOwner)));
    }

    #[test]
    fn test_empty_document_fails() {
        let (env, owner, _) = setup();
        let contract = DIDRegistryClient::new(&env, &env.register_contract(None, DIDRegistry));

        let result = contract.try_create_did(&owner, &Bytes::new(&env));
        assert_eq!(result, Err(Err(DIDError::InvalidDocument)));
    }

    #[test]
    fn test_resolve_nonexistent_fails() {
        let (env, _, _) = setup();
        let contract = DIDRegistryClient::new(&env, &env.register_contract(None, DIDRegistry));

        let fake_id = env.crypto().sha256(&Bytes::from_slice(&env, b"nope"));
        let result = contract.try_resolve_did(&fake_id.into());
        assert_eq!(result, Err(Err(DIDError::NotFound)));
    }

    #[test]
    fn test_deactivate_by_non_owner_fails() {
        let (env, owner, other) = setup();
        let contract = DIDRegistryClient::new(&env, &env.register_contract(None, DIDRegistry));

        let doc = Bytes::from_slice(&env, b"doc");
        let did_id = contract.create_did(&owner, &doc);

        let result = contract.try_deactivate_did(&did_id, &other);
        assert_eq!(result, Err(Err(DIDError::NotOwner)));
    }
}
