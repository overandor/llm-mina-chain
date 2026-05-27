#include "core.h"
#include <sstream>
#include <iomanip>
#include <random>

namespace llm_mina_chain {

// Utility functions
std::string sha256(const std::string& data) {
    unsigned char hash[SHA256_DIGEST_LENGTH];
    SHA256_CTX sha256;
    SHA256_Init(&sha256);
    SHA256_Update(&sha256, data.c_str(), data.size());
    SHA256_Final(hash, &sha256);
    
    std::stringstream ss;
    for (int i = 0; i < SHA256_DIGEST_LENGTH; i++) {
        ss << std::hex << std::setw(2) << std::setfill('0') << (int)hash[i];
    }
    return ss.str();
}

std::string generate_tx_id(const std::string& sender, const std::string& receiver,
                          uint64_t amount, uint64_t nonce, int64_t timestamp) {
    std::string data = sender + receiver + std::to_string(amount) + 
                      std::to_string(nonce) + std::to_string(timestamp);
    std::string full_hash = sha256(data);
    return full_hash.substr(0, 16);
}

int64_t current_timestamp() {
    auto now = std::chrono::system_clock::now();
    auto duration = now.time_since_epoch();
    return std::chrono::duration_cast<std::chrono::seconds>(duration).count();
}

// Transaction implementation
Transaction::Transaction() 
    : amount(0), nonce(0), timestamp(0) {}

Transaction::Transaction(const std::string& sender, const std::string& receiver,
                        uint64_t amount, uint64_t nonce,
                        std::optional<uint64_t> gas_limit,
                        std::optional<uint64_t> gas_price)
    : sender(sender), receiver(receiver), amount(amount), nonce(nonce),
      gas_limit(gas_limit), gas_price(gas_price), tx_type("transfer"),
      timestamp(current_timestamp()) {
    tx_id = generate_tx_id(sender, receiver, amount, nonce, timestamp);
}

std::string Transaction::hash() const {
    std::stringstream ss;
    ss << tx_id << sender << receiver << amount << nonce;
    if (gas_limit) ss << *gas_limit;
    if (gas_price) ss << *gas_price;
    ss << tx_type << timestamp;
    return sha256(ss.str());
}

bool Transaction::is_gasless() const {
    return !gas_limit.has_value() || !gas_price.has_value();
}

uint64_t Transaction::calculate_gas_cost() const {
    if (is_gasless()) return 0;
    return gas_limit.value_or(0) * gas_price.value_or(0);
}

// State implementation
State::State() {}

uint64_t State::get_balance(const std::string& address) const {
    auto it = balances.find(address);
    return it != balances.end() ? it->second : 0;
}

void State::set_balance(const std::string& address, uint64_t amount) {
    balances[address] = amount;
}

uint64_t State::get_nonce(const std::string& address) const {
    auto it = nonces.find(address);
    return it != nonces.end() ? it->second : 0;
}

void State::increment_nonce(const std::string& address) {
    nonces[address] = get_nonce(address) + 1;
}

bool State::apply_transaction(const Transaction& tx) {
    // Check nonce
    if (tx.nonce != get_nonce(tx.sender)) {
        return false;
    }
    
    // Check sender has enough to send the amount
    if (get_balance(tx.sender) < tx.amount) {
        return false;
    }
    
    // Check receiver can afford gas (if gas is specified)
    uint64_t gas_cost = tx.calculate_gas_cost();
    if (gas_cost > 0 && get_balance(tx.receiver) < gas_cost) {
        return false;
    }
    
    // Execute atomically
    uint64_t sender_balance = get_balance(tx.sender);
    uint64_t receiver_balance = get_balance(tx.receiver);
    
    // Sender sends amount (mining work done by sender)
    balances[tx.sender] = sender_balance - tx.amount;
    
    // Receiver receives amount minus gas (if gas is specified)
    uint64_t net_received = (gas_cost > 0) 
        ? receiver_balance + tx.amount - gas_cost 
        : receiver_balance + tx.amount;
    balances[tx.receiver] = net_received;
    
    // Increment sender nonce (sender does the work)
    increment_nonce(tx.sender);
    
    return true;
}

std::string State::hash() const {
    std::stringstream ss;
    for (const auto& [addr, bal] : balances) {
        ss << addr << bal;
    }
    for (const auto& [addr, nonce] : nonces) {
        ss << addr << nonce;
    }
    return sha256(ss.str());
}

// Block implementation
Block::Block() : height(0), timestamp(0) {}

Block::Block(uint64_t height, const std::vector<Transaction>& transactions,
             const std::string& previous_hash, const std::string& state_hash)
    : height(height), timestamp(current_timestamp()), transactions(transactions),
      previous_hash(previous_hash), state_hash(state_hash) {
    block_hash = compute_hash();
}

std::string Block::compute_hash() const {
    std::stringstream ss;
    ss << height << timestamp;
    for (const auto& tx : transactions) {
        ss << tx.hash();
    }
    ss << previous_hash << state_hash;
    if (proof) ss << *proof;
    return sha256(ss.str());
}

void Block::set_proof(const std::string& proof) {
    this->proof = proof;
    block_hash = compute_hash();
}

// Blockchain implementation
Blockchain::Blockchain() : gas_price(1), min_gas_price(0), rng_(std::random_device{}()) {
    create_genesis_block();
}

void Blockchain::create_genesis_block() {
    // Initialize with some accounts
    state.set_balance("genesis", 1000000);
    state.set_balance("alice", 1000);
    state.set_balance("bob", 1000);
    
    Block genesis(0, std::vector<Transaction>(), 
                  std::string(64, '0'), state.hash());
    genesis.set_proof("genesis_proof");
    chain.push_back(genesis);
}

Block Blockchain::get_latest_block() const {
    return chain.back();
}

std::optional<Block> Blockchain::get_block(uint64_t height) const {
    if (height < chain.size()) {
        return chain[height];
    }
    return std::nullopt;
}

bool Blockchain::add_transaction(const Transaction& tx) {
    if (!validate_transaction(tx)) {
        return false;
    }
    transaction_pool.push_back(tx);
    return true;
}

bool Blockchain::validate_transaction(const Transaction& tx) const {
    // Check nonce
    if (tx.nonce != state.get_nonce(tx.sender)) {
        return false;
    }
    
    // Check sender has enough to send the amount
    if (state.get_balance(tx.sender) < tx.amount) {
        return false;
    }
    
    // Check receiver can afford gas (if gas is specified)
    uint64_t gas_cost = tx.calculate_gas_cost();
    if (gas_cost > 0 && state.get_balance(tx.receiver) < gas_cost) {
        return false;
    }
    
    // If gas is specified, check gas price
    if (!tx.is_gasless() && tx.gas_price.has_value()) {
        if (tx.gas_price.value() < min_gas_price) {
            return false;
        }
    }
    
    return true;
}

std::optional<Block> Blockchain::create_block(const std::vector<Transaction>& transactions) {
    // Create a copy of state for testing
    State test_state;
    test_state.balances = state.balances;
    test_state.nonces = state.nonces;
    test_state.contracts = state.contracts;
    
    // Try to apply all transactions atomically
    std::vector<Transaction> valid_txs;
    for (const auto& tx : transactions) {
        if (test_state.apply_transaction(tx)) {
            valid_txs.push_back(tx);
        } else {
            // If any transaction fails, rollback all
            return std::nullopt;
        }
    }
    
    // All transactions valid - create block
    Block new_block(get_latest_block().height + 1, valid_txs,
                    get_latest_block().block_hash, test_state.hash());
    new_block.set_proof(generate_proof(test_state));
    
    // Update actual state
    state = test_state;
    chain.push_back(new_block);
    
    // Remove from pool
    std::vector<Transaction> new_pool;
    for (const auto& tx : transaction_pool) {
        bool found = false;
        for (const auto& v : valid_txs) {
            if (tx.tx_id == v.tx_id) {
                found = true;
                break;
            }
        }
        if (!found) {
            new_pool.push_back(tx);
        }
    }
    transaction_pool = new_pool;
    
    return new_block;
}

std::string Blockchain::generate_proof(const State& state) const {
    // In a real implementation, this would generate a zk-SNARK proof
    // For this micro version, we use a hash as a placeholder
    std::random_device rd;
    std::mt19937 gen(rd());
    std::uniform_int_distribution<uint64_t> dist(0, UINT64_MAX);
    uint64_t random = dist(gen);
    
    std::stringstream ss;
    ss << state.hash() << (get_latest_block().height + 1) 
       << current_timestamp() << random;
    return sha256(ss.str());
}

void Blockchain::set_gas_price(uint64_t price) {
    gas_price = std::max(price, min_gas_price);
}

uint64_t Blockchain::get_gas_price() const {
    return gas_price;
}

} // namespace llm_mina_chain
