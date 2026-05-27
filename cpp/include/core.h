#ifndef CORE_H
#define CORE_H

#include <string>
#include <vector>
#include <map>
#include <optional>
#include <cstdint>
#include <chrono>
#include <random>
#include <openssl/sha.h>

namespace llm_mina_chain {

// Transaction type enum
enum class TransactionType {
    TRANSFER,
    CONTRACT_CALL,
    LLM_GENERATED
};

// Atomic transaction with optional gas
struct Transaction {
    std::string tx_id;
    std::string sender;
    std::string receiver;
    uint64_t amount;
    uint64_t nonce;
    std::optional<uint64_t> gas_limit;
    std::optional<uint64_t> gas_price;
    std::string tx_type;
    std::string data;  // JSON string
    std::string signature;
    int64_t timestamp;
    
    Transaction();
    Transaction(const std::string& sender, const std::string& receiver, 
                uint64_t amount, uint64_t nonce,
                std::optional<uint64_t> gas_limit = std::nullopt,
                std::optional<uint64_t> gas_price = std::nullopt);
    
    std::string hash() const;
    bool is_gasless() const;
    uint64_t calculate_gas_cost() const;
};

// Blockchain state (balances, nonces, contracts)
struct State {
    std::map<std::string, uint64_t> balances;
    std::map<std::string, uint64_t> nonces;
    std::map<std::string, std::string> contracts;  // JSON strings
    
    State();
    
    uint64_t get_balance(const std::string& address) const;
    void set_balance(const std::string& address, uint64_t amount);
    uint64_t get_nonce(const std::string& address) const;
    void increment_nonce(const std::string& address);
    
    // Atomically apply transaction to state
    bool apply_transaction(const Transaction& tx);
    
    std::string hash() const;
};

// Block with recursive proof (Mina-like)
struct Block {
    uint64_t height;
    int64_t timestamp;
    std::vector<Transaction> transactions;
    std::string previous_hash;
    std::string state_hash;
    std::optional<std::string> proof;
    std::string block_hash;
    
    Block();
    Block(uint64_t height, const std::vector<Transaction>& transactions,
          const std::string& previous_hash, const std::string& state_hash);
    
    std::string compute_hash() const;
    void set_proof(const std::string& proof);
};

// Recursive blockchain with atomic transactions
class Blockchain {
public:
    Blockchain();
    
    Block get_latest_block() const;
    std::optional<Block> get_block(uint64_t height) const;
    
    // Add transaction to pool with immediate validation
    bool add_transaction(const Transaction& tx);
    
    // Create new block with atomic transaction execution
    std::optional<Block> create_block(const std::vector<Transaction>& transactions);
    
    void set_gas_price(uint64_t price);
    uint64_t get_gas_price() const;
    
    // Public members for simplicity
    std::vector<Block> chain;
    State state;
    std::vector<Transaction> transaction_pool;
    uint64_t gas_price;
    uint64_t min_gas_price;
    
private:
    void create_genesis_block();
    bool validate_transaction(const Transaction& tx) const;
    std::string generate_proof(const State& state) const;
    
    std::mt19937 rng_;
};

// Utility functions
std::string sha256(const std::string& data);
std::string generate_tx_id(const std::string& sender, const std::string& receiver,
                          uint64_t amount, uint64_t nonce, int64_t timestamp);
int64_t current_timestamp();

} // namespace llm_mina_chain

#endif // CORE_H
