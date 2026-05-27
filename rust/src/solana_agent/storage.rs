//! Persistent storage backend for Solana blockchain indexing using RocksDB
//! Provides indexed access to accounts, transactions, and program state

use rocksdb::{Options, DB as RocksDB, WriteBatch, WriteOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("RocksDB error: {0}")]
    RocksDB(#[from] rocksdb::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Key not found: {0}")]
    NotFound(String),
}

/// Storage backend for Solana blockchain data
pub struct SolanaStorage {
    db: Arc<RocksDB>,
}

impl SolanaStorage {
    /// Open or create a RocksDB database at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_max_open_files(500);
        opts.set_use_fsync(false);
        opts.set_bytes_per_sync(1 << 20); // 1MB

        let db = Arc::new(RocksDB::open(&opts, path)?);
        Ok(Self { db })
    }

    /// Store account data
    pub fn put_account(&self, pubkey: &str, account: &AccountData) -> Result<(), StorageError> {
        let key = format!("account:{}", pubkey);
        let value = bincode::serialize(account)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        self.db.put(key.as_bytes(), value)?;
        Ok(())
    }

    /// Retrieve account data
    pub fn get_account(&self, pubkey: &str) -> Result<Option<AccountData>, StorageError> {
        let key = format!("account:{}", pubkey);
        match self.db.get(key.as_bytes())? {
            Some(value) => {
                let account = bincode::deserialize(&value)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(account))
            }
            None => Ok(None),
        }
    }

    /// Store transaction data
    pub fn put_transaction(&self, signature: &str, tx: &TransactionData) -> Result<(), StorageError> {
        let key = format!("tx:{}", signature);
        let value = bincode::serialize(tx)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        self.db.put(key.as_bytes(), value)?;
        Ok(())
    }

    /// Retrieve transaction data
    pub fn get_transaction(&self, signature: &str) -> Result<Option<TransactionData>, StorageError> {
        let key = format!("tx:{}", signature);
        match self.db.get(key.as_bytes())? {
            Some(value) => {
                let tx = bincode::deserialize(&value)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(tx))
            }
            None => Ok(None),
        }
    }

    /// Index account by owner for program account lookups
    pub fn index_account_by_owner(&self, owner: &str, account_pubkey: &str) -> Result<(), StorageError> {
        let key = format!("owner:{}:{}", owner, account_pubkey);
        self.db.put(key.as_bytes(), b"")?;
        Ok(())
    }

    /// Get all accounts owned by a program
    pub fn get_accounts_by_owner(&self, owner: &str) -> Result<Vec<String>, StorageError> {
        let prefix = format!("owner:{}:", owner);
        let mut accounts = Vec::new();
        let iter = self.db.prefix_iterator(prefix.as_bytes());
        for item in iter {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            if let Some(account_pubkey) = key_str.strip_prefix(&prefix) {
                accounts.push(account_pubkey.to_string());
            }
        }
        Ok(accounts)
    }

    /// Store slot data for historical queries
    pub fn put_slot(&self, slot: u64, slot_data: &SlotData) -> Result<(), StorageError> {
        let key = format!("slot:{}", slot);
        let value = bincode::serialize(slot_data)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        self.db.put(key.as_bytes(), value)?;
        Ok(())
    }

    /// Retrieve slot data
    pub fn get_slot(&self, slot: u64) -> Result<Option<SlotData>, StorageError> {
        let key = format!("slot:{}", slot);
        match self.db.get(key.as_bytes())? {
            Some(value) => {
                let slot_data = bincode::deserialize(&value)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(slot_data))
            }
            None => Ok(None),
        }
    }

    /// Get latest slot
    pub fn get_latest_slot(&self) -> Result<Option<u64>, StorageError> {
        let iter = self.db.prefix_iterator(b"slot:");
        let mut latest_slot: Option<u64> = None;
        for item in iter {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            if let Some(slot_str) = key_str.strip_prefix("slot:") {
                if let Ok(slot) = slot_str.parse::<u64>() {
                    latest_slot = Some(latest_slot.map_or(slot, |s| s.max(slot)));
                }
            }
        }
        Ok(latest_slot)
    }

    /// Batch write multiple items atomically
    pub fn batch_write(&self, operations: Vec<StorageOperation>) -> Result<(), StorageError> {
        let mut batch = WriteBatch::default();
        let write_opts = WriteOptions::default();

        for op in operations {
            match op {
                StorageOperation::PutAccount(pubkey, account) => {
                    let key = format!("account:{}", pubkey);
                    let value = bincode::serialize(&account)
                        .map_err(|e| StorageError::Serialization(e.to_string()))?;
                    batch.put(key.as_bytes(), value);
                }
                StorageOperation::PutTransaction(signature, tx) => {
                    let key = format!("tx:{}", signature);
                    let value = bincode::serialize(&tx)
                        .map_err(|e| StorageError::Serialization(e.to_string()))?;
                    batch.put(key.as_bytes(), value);
                }
                StorageOperation::PutSlot(slot, slot_data) => {
                    let key = format!("slot:{}", slot);
                    let value = bincode::serialize(&slot_data)
                        .map_err(|e| StorageError::Serialization(e.to_string()))?;
                    batch.put(key.as_bytes(), value);
                }
                StorageOperation::IndexAccountByOwner(owner, account_pubkey) => {
                    let key = format!("owner:{}:{}", owner, account_pubkey);
                    batch.put(key.as_bytes(), b"");
                }
            }
        }

        self.db.write_opt(batch, &write_opts)?;
        Ok(())
    }

    /// Compact the database to reclaim space
    pub fn compact(&self) -> Result<(), StorageError> {
        self.db.compact_range(None::<&[u8]>, None::<&[u8]>);
        Ok(())
    }
}

/// Storage operation for batch writes
pub enum StorageOperation {
    PutAccount(String, AccountData),
    PutTransaction(String, TransactionData),
    PutSlot(u64, SlotData),
    IndexAccountByOwner(String, String),
}

/// Indexed account data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub pubkey: String,
    pub owner: String,
    pub lamports: u64,
    pub data: Vec<u8>,
    pub executable: bool,
    pub rent_epoch: u64,
    pub slot: u64,
    pub timestamp: i64,
}

/// Indexed transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub fee: u64,
    pub status: TransactionStatus,
    pub account_keys: Vec<String>,
    pub log_messages: Vec<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Success,
    Failed,
}

/// Indexed slot data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotData {
    pub slot: u64,
    pub parent_slot: u64,
    pub block_height: Option<u64>,
    pub block_time: Option<i64>,
    pub transactions_count: u64,
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_account_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SolanaStorage::open(temp_dir.path()).unwrap();

        let account = AccountData {
            pubkey: "test_pubkey".to_string(),
            owner: "test_owner".to_string(),
            lamports: 1000000,
            data: vec![1, 2, 3],
            executable: false,
            rent_epoch: 0,
            slot: 100,
            timestamp: 1234567890,
        };

        storage.put_account("test_pubkey", &account).unwrap();
        let retrieved = storage.get_account("test_pubkey").unwrap().unwrap();
        assert_eq!(retrieved.pubkey, account.pubkey);
        assert_eq!(retrieved.lamports, account.lamports);
    }

    #[test]
    fn test_transaction_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SolanaStorage::open(temp_dir.path()).unwrap();

        let tx = TransactionData {
            signature: "test_signature".to_string(),
            slot: 100,
            block_time: Some(1234567890),
            fee: 5000,
            status: TransactionStatus::Success,
            account_keys: vec!["account1".to_string(), "account2".to_string()],
            log_messages: vec!["log1".to_string()],
            timestamp: 1234567890,
        };

        storage.put_transaction("test_signature", &tx).unwrap();
        let retrieved = storage.get_transaction("test_signature").unwrap().unwrap();
        assert_eq!(retrieved.signature, tx.signature);
        assert_eq!(retrieved.slot, tx.slot);
    }

    #[test]
    fn test_owner_indexing() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SolanaStorage::open(temp_dir.path()).unwrap();

        storage.index_account_by_owner("owner1", "account1").unwrap();
        storage.index_account_by_owner("owner1", "account2").unwrap();
        storage.index_account_by_owner("owner2", "account3").unwrap();

        let owner1_accounts = storage.get_accounts_by_owner("owner1").unwrap();
        assert_eq!(owner1_accounts.len(), 2);
        assert!(owner1_accounts.contains(&"account1".to_string()));
        assert!(owner1_accounts.contains(&"account2".to_string()));
    }

    #[test]
    fn test_slot_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SolanaStorage::open(temp_dir.path()).unwrap();

        let slot_data = SlotData {
            slot: 100,
            parent_slot: 99,
            block_height: Some(50),
            block_time: Some(1234567890),
            transactions_count: 10,
            timestamp: 1234567890,
        };

        storage.put_slot(100, &slot_data).unwrap();
        let retrieved = storage.get_slot(100).unwrap().unwrap();
        assert_eq!(retrieved.slot, 100);
        assert_eq!(retrieved.parent_slot, 99);

        let latest = storage.get_latest_slot().unwrap().unwrap();
        assert_eq!(latest, 100);
    }

    #[test]
    fn test_batch_write() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SolanaStorage::open(temp_dir.path()).unwrap();

        let account = AccountData {
            pubkey: "test_pubkey".to_string(),
            owner: "test_owner".to_string(),
            lamports: 1000000,
            data: vec![],
            executable: false,
            rent_epoch: 0,
            slot: 100,
            timestamp: 1234567890,
        };

        let operations = vec![
            StorageOperation::PutAccount("test_pubkey".to_string(), account),
        ];

        storage.batch_write(operations).unwrap();
        let retrieved = storage.get_account("test_pubkey").unwrap().unwrap();
        assert_eq!(retrieved.pubkey, "test_pubkey");
    }
}
