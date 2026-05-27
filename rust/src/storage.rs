//! Persistent storage layer using RocksDB

use rocksdb::{DB, Options, WriteBatch, WriteOptions};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

use crate::{Block, Transaction, State};

/// Storage errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("RocksDB error: {0}")]
    RocksDB(#[from] rocksdb::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Deserialization error: {0}")]
    Deserialization(String),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
}

/// Storage keys
const BLOCK_PREFIX: &[u8] = b"block/";
const TRANSACTION_PREFIX: &[u8] = b"tx/";
const STATE_KEY: &[u8] = b"state";
const LATEST_HEIGHT_KEY: &[u8] = b"latest_height";

/// Persistent storage for blockchain data
pub struct BlockchainStorage {
    db: Arc<DB>,
}

impl BlockchainStorage {
    /// Open or create storage at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        
        let db = DB::open(&opts, path)?;
        
        Ok(BlockchainStorage {
            db: Arc::new(db),
        })
    }
    
    /// Store a block
    #[tracing::instrument(skip(self, block), fields(height = block.height, tx_count = block.transactions.len()))]
    pub fn put_block(&self, block: &Block) -> Result<(), StorageError> {
        let key = format!("{}{}", std::str::from_utf8(BLOCK_PREFIX).unwrap(), block.height);
        let value = serde_json::to_vec(block)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        
        self.db.put(key.as_bytes(), value)?;
        
        // Update latest height
        self.db.put(LATEST_HEIGHT_KEY, block.height.to_be_bytes())?;
        
        Ok(())
    }
    
    /// Get a block by height
    #[tracing::instrument(skip(self), fields(height = height))]
    pub fn get_block(&self, height: u64) -> Result<Option<Block>, StorageError> {
        let key = format!("{}{}", std::str::from_utf8(BLOCK_PREFIX).unwrap(), height);
        
        match self.db.get(key.as_bytes())? {
            Some(value) => {
                let block = serde_json::from_slice(&value)
                    .map_err(|e| StorageError::Deserialization(e.to_string()))?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }
    
    /// Store a transaction
    #[tracing::instrument(skip(self, tx), fields(tx_id = %tx.tx_id))]
    pub fn put_transaction(&self, tx: &Transaction) -> Result<(), StorageError> {
        let key = format!("{}{}", std::str::from_utf8(TRANSACTION_PREFIX).unwrap(), tx.tx_id);
        let value = serde_json::to_vec(tx)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        
        self.db.put(key.as_bytes(), value)?;
        Ok(())
    }
    
    /// Get a transaction by ID
    #[tracing::instrument(skip(self), fields(tx_id = tx_id))]
    pub fn get_transaction(&self, tx_id: &str) -> Result<Option<Transaction>, StorageError> {
        let key = format!("{}{}", std::str::from_utf8(TRANSACTION_PREFIX).unwrap(), tx_id);
        
        match self.db.get(key.as_bytes())? {
            Some(value) => {
                let tx = serde_json::from_slice(&value)
                    .map_err(|e| StorageError::Deserialization(e.to_string()))?;
                Ok(Some(tx))
            }
            None => Ok(None),
        }
    }
    
    /// Store the current state
    #[tracing::instrument(skip(self, state), fields(balance_count = state.balances.len()))]
    pub fn put_state(&self, state: &State) -> Result<(), StorageError> {
        let value = serde_json::to_vec(state)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        
        self.db.put(STATE_KEY, value)?;
        Ok(())
    }
    
    /// Get the current state
    #[tracing::instrument(skip(self))]
    pub fn get_state(&self) -> Result<Option<State>, StorageError> {
        match self.db.get(STATE_KEY)? {
            Some(value) => {
                let state = serde_json::from_slice(&value)
                    .map_err(|e| StorageError::Deserialization(e.to_string()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }
    
    /// Get the latest block height
    pub fn get_latest_height(&self) -> Result<Option<u64>, StorageError> {
        match self.db.get(LATEST_HEIGHT_KEY)? {
            Some(value) => {
                let height = u64::from_be_bytes([
                    value[0], value[1], value[2], value[3],
                    value[4], value[5], value[6], value[7],
                ]);
                Ok(Some(height))
            }
            None => Ok(None),
        }
    }
    
    /// Batch write multiple blocks
    #[tracing::instrument(skip(self, blocks), fields(batch_len = blocks.len()))]
    pub fn put_blocks_batch(&self, blocks: &[Block]) -> Result<(), StorageError> {
        let mut batch = WriteBatch::default();
        
        for block in blocks {
            let key = format!("{}{}", std::str::from_utf8(BLOCK_PREFIX).unwrap(), block.height);
            let value = serde_json::to_vec(block)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            batch.put(key.as_bytes(), value);
        }
        
        // Update latest height
        if let Some(last_block) = blocks.last() {
            batch.put(LATEST_HEIGHT_KEY, last_block.height.to_be_bytes());
        }
        
        let write_opts = WriteOptions::default();
        self.db.write_opt(batch, &write_opts)?;
        
        Ok(())
    }
    
    /// Delete a block (for reorgs)
    pub fn delete_block(&self, height: u64) -> Result<(), StorageError> {
        let key = format!("{}{}", std::str::from_utf8(BLOCK_PREFIX).unwrap(), height);
        self.db.delete(key.as_bytes())?;
        Ok(())
    }
    
    /// Create a snapshot for consistent reads
    pub fn snapshot(&self) -> Result<rocksdb::Snapshot<'_>, StorageError> {
        Ok(self.db.snapshot())
    }
    
    /// Compact the database
    pub fn compact(&self) -> Result<(), StorageError> {
        self.db.compact_range(None::<&str>, None::<&str>);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_block_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = BlockchainStorage::open(temp_dir.path()).unwrap();
        
        let block = Block::new(
            1,
            vec![],
            "prev_hash".to_string(),
            "state_hash".to_string(),
        );
        
        storage.put_block(&block).unwrap();
        let retrieved = storage.get_block(1).unwrap();
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().height, 1);
    }
    
    #[test]
    fn test_transaction_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = BlockchainStorage::open(temp_dir.path()).unwrap();
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0,
            Some(21000),
            Some(1),
        );
        
        storage.put_transaction(&tx).unwrap();
        let retrieved = storage.get_transaction(&tx.tx_id).unwrap();
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().tx_id, tx.tx_id);
    }
    
    #[test]
    fn test_state_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = BlockchainStorage::open(temp_dir.path()).unwrap();
        
        let mut state = State::new();
        state.set_balance("alice".to_string(), 1000);
        
        storage.put_state(&state).unwrap();
        let retrieved = storage.get_state().unwrap();
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().get_balance("alice"), 1000);
    }
}
