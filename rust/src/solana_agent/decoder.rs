//! Account decoding for Solana programs
//! Supports SPL Token, Metaplex, Anchor, and common program account types

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("Invalid account data length: expected {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
    #[error("Unknown account discriminator: {0}")]
    UnknownDiscriminator(u64),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Base64 decode error: {0}")]
    Base64(String),
}

/// Decoded account data with program-specific structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "program", rename_all = "lowercase")]
pub enum DecodedAccount {
    /// SPL Token Mint
    TokenMint(TokenMintData),
    /// SPL Token Account
    TokenAccount(TokenAccountData),
    /// Metaplex Metadata
    MetaplexMetadata(MetaplexMetadata),
    /// Metaplex Master Edition
    MetaplexMasterEdition(MetaplexMasterEdition),
    /// System Program
    System(SystemAccount),
    /// Generic/Unknown account
    Generic(GenericAccount),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMintData {
    pub mint_authority: Option<String>,
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAccountData {
    pub mint: String,
    pub owner: String,
    pub amount: u64,
    pub delegate: Option<String>,
    pub state: TokenAccountState,
    pub is_native: Option<u64>,
    pub delegated_amount: u64,
    pub close_authority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenAccountState {
    Uninitialized,
    Initialized,
    Frozen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaplexMetadata {
    pub key: u8,
    pub update_authority: String,
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub creators: Vec<Creator>,
    pub primary_sale_happened: bool,
    pub is_mutable: bool,
    pub edition_nonce: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creator {
    pub address: String,
    pub verified: bool,
    pub share: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaplexMasterEdition {
    pub key: u8,
    pub edition: u64,
    pub mint: String,
    pub print_supply: Option<PrintSupply>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintSupply {
    pub current: u64,
    pub max: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemAccount {
    pub lamports: u64,
    pub owner: String,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericAccount {
    pub owner: String,
    pub data: Vec<u8>,
    pub executable: bool,
}

/// Known program IDs
pub mod program_ids {
    pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
    pub const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
    pub const METAPLEX_METADATA: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
    pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
}

/// Decode account data based on owner program
pub fn decode_account(owner: &str, data: &[u8]) -> Result<DecodedAccount, DecodeError> {
    match owner {
        program_ids::TOKEN_PROGRAM | program_ids::TOKEN_2022_PROGRAM => {
            decode_token_account(data)
        }
        program_ids::METAPLEX_METADATA => {
            decode_metaplex_account(data)
        }
        program_ids::SYSTEM_PROGRAM => {
            Ok(DecodedAccount::System(SystemAccount {
                lamports: 0, // Not in data
                owner: owner.to_string(),
                executable: false,
            }))
        }
        _ => Ok(DecodedAccount::Generic(GenericAccount {
            owner: owner.to_string(),
            data: data.to_vec(),
            executable: false,
        })),
    }
}

/// Decode SPL Token account (mint or token account)
fn decode_token_account(data: &[u8]) -> Result<DecodedAccount, DecodeError> {
    if data.len() < 8 {
        return Err(DecodeError::InvalidLength {
            expected: 8,
            actual: data.len(),
        });
    }

    // First 8 bytes is the discriminator
    let _discriminator = u64::from_le_bytes(data[0..8].try_into().unwrap());

    // Token mint discriminator: 0x...
    // Token account discriminator: 0x...
    // For now, use length to distinguish
    if data.len() == 82 {
        // Token mint
        decode_token_mint(data)
    } else if data.len() == 165 {
        // Token account
        decode_token_account_data(data)
    } else {
        Ok(DecodedAccount::Generic(GenericAccount {
            owner: program_ids::TOKEN_PROGRAM.to_string(),
            data: data.to_vec(),
            executable: false,
        }))
    }
}

fn decode_token_mint(data: &[u8]) -> Result<DecodedAccount, DecodeError> {
    if data.len() < 82 {
        return Err(DecodeError::InvalidLength {
            expected: 82,
            actual: data.len(),
        });
    }

    let mint_authority = if data[36] != 0 {
        Some(bs58::encode(&data[4..36]).into_string())
    } else {
        None
    };

    let supply = u64::from_le_bytes(data[36..44].try_into().unwrap());
    let decimals = data[44];
    let is_initialized = data[45] != 0;

    let freeze_authority = if data[77] != 0 {
        Some(bs58::encode(&data[45..77]).into_string())
    } else {
        None
    };

    Ok(DecodedAccount::TokenMint(TokenMintData {
        mint_authority,
        supply,
        decimals,
        is_initialized,
        freeze_authority,
    }))
}

fn decode_token_account_data(data: &[u8]) -> Result<DecodedAccount, DecodeError> {
    if data.len() < 165 {
        return Err(DecodeError::InvalidLength {
            expected: 165,
            actual: data.len(),
        });
    }

    let mint = bs58::encode(&data[0..32]).into_string();
    let owner = bs58::encode(&data[32..64]).into_string();
    let amount = u64::from_le_bytes(data[64..72].try_into().unwrap());

    let delegate = if data[104] != 0 {
        Some(bs58::encode(&data[72..104]).into_string())
    } else {
        None
    };

    let state_byte = data[104];
    let state = match state_byte {
        0 => TokenAccountState::Uninitialized,
        1 => TokenAccountState::Initialized,
        2 => TokenAccountState::Frozen,
        _ => TokenAccountState::Uninitialized,
    };

    let is_native = if data[136] != 0 {
        Some(u64::from_le_bytes(data[104..112].try_into().unwrap()))
    } else {
        None
    };

    let delegated_amount = u64::from_le_bytes(data[112..120].try_into().unwrap());

    let close_authority = if data[161] != 0 {
        Some(bs58::encode(&data[120..152]).into_string())
    } else {
        None
    };

    Ok(DecodedAccount::TokenAccount(TokenAccountData {
        mint,
        owner,
        amount,
        delegate,
        state,
        is_native,
        delegated_amount,
        close_authority,
    }))
}

/// Decode Metaplex metadata account
fn decode_metaplex_account(data: &[u8]) -> Result<DecodedAccount, DecodeError> {
    if data.len() < 1 {
        return Err(DecodeError::InvalidLength {
            expected: 1,
            actual: data.len(),
        });
    }

    let key = data[0];

    // Metaplex metadata key is 4
    if key == 4 && data.len() >= 1 + 32 + 32 + 4 + 4 + 4 + 2 + 1 + 1 + 1 {
        let update_authority = bs58::encode(&data[1..33]).into_string();
        let mint = bs58::encode(&data[33..65]).into_string();

        let name_len = u32::from_le_bytes(data[65..69].try_into().unwrap()) as usize;
        let name_start = 69;
        let name_end = name_start + name_len;
        let name = String::from_utf8_lossy(&data[name_start..name_end]).to_string();

        let symbol_len = u32::from_le_bytes(data[name_end..name_end + 4].try_into().unwrap()) as usize;
        let symbol_start = name_end + 4;
        let symbol_end = symbol_start + symbol_len;
        let symbol = String::from_utf8_lossy(&data[symbol_start..symbol_end]).to_string();

        let uri_len = u32::from_le_bytes(data[symbol_end..symbol_end + 4].try_into().unwrap()) as usize;
        let uri_start = symbol_end + 4;
        let uri_end = uri_start + uri_len;
        let uri = String::from_utf8_lossy(&data[uri_start..uri_end]).to_string();

        let seller_fee_basis_points = u16::from_le_bytes(data[uri_end..uri_end + 2].try_into().unwrap());

        // Simplified creator parsing - would need full implementation
        let creators = vec![];

        let primary_sale_happened = data[uri_end + 2] != 0;
        let is_mutable = data[uri_end + 3] != 0;

        Ok(DecodedAccount::MetaplexMetadata(MetaplexMetadata {
            key,
            update_authority,
            mint,
            name,
            symbol,
            uri,
            seller_fee_basis_points,
            creators,
            primary_sale_happened,
            is_mutable,
            edition_nonce: None,
        }))
    } else {
        Ok(DecodedAccount::Generic(GenericAccount {
            owner: program_ids::METAPLEX_METADATA.to_string(),
            data: data.to_vec(),
            executable: false,
        }))
    }
}

/// Convert decoded account to JSON value for API responses
pub fn decoded_to_json(decoded: &DecodedAccount) -> Value {
    json!(decoded)
}

/// Try to decode account from RPC response
pub fn decode_from_rpc_response(account_data: &Value) -> Result<DecodedAccount, DecodeError> {
    let owner = account_data["owner"]
        .as_str()
        .ok_or_else(|| DecodeError::Parse("Missing owner field".to_string()))?;

    let data_base64 = account_data["data"]
        .as_str()
        .or_else(|| account_data["data"][0].as_str())
        .ok_or_else(|| DecodeError::Parse("Missing data field".to_string()))?;

    let data = base64::decode(data_base64)
        .map_err(|e| DecodeError::Base64(e.to_string()))?;

    decode_account(owner, &data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_generic_account() {
        let data = vec![1, 2, 3, 4];
        let decoded = decode_account("UnknownProgram111111111111111111111111111", &data).unwrap();
        match decoded {
            DecodedAccount::Generic(g) => {
                assert_eq!(g.owner, "UnknownProgram111111111111111111111111111");
                assert_eq!(g.data, vec![1, 2, 3, 4]);
            }
            _ => panic!("Expected Generic account"),
        }
    }

    #[test]
    fn test_system_account() {
        let data = vec![];
        let decoded = decode_account(program_ids::SYSTEM_PROGRAM, &data).unwrap();
        match decoded {
            DecodedAccount::System(s) => {
                assert_eq!(s.owner, program_ids::SYSTEM_PROGRAM);
            }
            _ => panic!("Expected System account"),
        }
    }

    #[test]
    fn test_token_mint_insufficient_data() {
        let data = vec![0; 10];
        let result = decode_token_mint(&data);
        assert!(result.is_err());
    }
}
