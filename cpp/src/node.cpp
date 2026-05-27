#include "core.h"
#include "llm_layer.h"
#include <iostream>
#include <sstream>
#include <vector>
#include <string>
#include <thread>
#include <chrono>
#include <mutex>

using namespace llm_mina_chain;

std::mutex blockchain_mutex;
bool mining_active = true;

void mining_thread(Blockchain* blockchain) {
    while (mining_active) {
        std::this_thread::sleep_for(std::chrono::seconds(5));
        
        std::lock_guard<std::mutex> lock(blockchain_mutex);
        if (!blockchain->transaction_pool.empty()) {
            std::vector<Transaction> txs = blockchain->transaction_pool;
            auto block = blockchain->create_block(txs);
            if (block) {
                std::cout << "⛏️  Mined block #" << block->height 
                          << " with " << block->transactions.size() << " transactions" << std::endl;
            } else {
                std::cout << "❌ Failed to create block - invalid transactions" << std::endl;
            }
        }
    }
}

void print_help() {
    std::cout << "📖 Available Commands:" << std::endl;
    std::cout << "   help              - Show this help" << std::endl;
    std::cout << "   state             - Show current blockchain state" << std::endl;
    std::cout << "   block [height]    - Show specific block or latest" << std::endl;
    std::cout << "   chain             - Show entire blockchain" << std::endl;
    std::cout << "   transfer <s> <r> <a>  - Create transfer transaction" << std::endl;
    std::cout << "   gasless <s> <r> <a>   - Create gasless transaction" << std::endl;
    std::cout << "   llm <text>        - Parse natural language to transaction" << std::endl;
    std::cout << "   mine              - Mine next block" << std::endl;
    std::cout << "   pool              - Show transaction pool" << std::endl;
    std::cout << "   gas [price]       - Set or get gas price" << std::endl;
    std::cout << "   exit              - Exit the node" << std::endl;
}

void print_state(const Blockchain& bc) {
    std::cout << "   Balances:" << std::endl;
    for (const auto& [addr, balance] : bc.state.balances) {
        std::cout << "     " << addr << ": " << balance << std::endl;
    }
    std::cout << "   Nonces:" << std::endl;
    for (const auto& [addr, nonce] : bc.state.nonces) {
        std::cout << "     " << addr << ": " << nonce << std::endl;
    }
    std::cout << "   Gas Price: " << bc.gas_price << std::endl;
}

void print_block(const Block& block) {
    std::cout << "📦 Block #" << block.height << std::endl;
    std::cout << "   Hash: " << block.block_hash.substr(0, 64) << std::endl;
    std::cout << "   Previous: " << block.previous_hash.substr(0, 64) << std::endl;
    std::cout << "   State Hash: " << block.state_hash.substr(0, 64) << std::endl;
    std::cout << "   Proof: " << (block.proof ? block.proof.value() : "none") << std::endl;
    std::cout << "   Transactions: " << block.transactions.size() << std::endl;
    for (const auto& tx : block.transactions) {
        std::cout << "     " << tx.sender << " -> " << tx.receiver 
                  << " (" << tx.amount << ") [" << tx.tx_id.substr(0, 8) << "]" << std::endl;
    }
}

void print_chain(const Blockchain& bc) {
    std::cout << "🔗 Blockchain (" << bc.chain.size() << " blocks)" << std::endl;
    for (const auto& block : bc.chain) {
        std::cout << "   Block #" << block.height << ": " << block.transactions.size() 
                  << " transactions, hash: " << block.block_hash.substr(0, 16) << std::endl;
    }
}

void handle_command(const std::string& input, Blockchain* blockchain, const LLMTransactionParser& parser) {
    std::istringstream iss(input);
    std::vector<std::string> parts((std::istream_iterator<std::string>(iss)),
                                   std::istream_iterator<std::string>());
    
    if (parts.empty()) return;
    
    std::string cmd = parts[0];
    
    if (cmd == "help") {
        print_help();
    } else if (cmd == "state") {
        std::lock_guard<std::mutex> lock(blockchain_mutex);
        print_state(*blockchain);
    } else if (cmd == "block") {
        std::lock_guard<std::mutex> lock(blockchain_mutex);
        if (parts.size() >= 2) {
            try {
                uint64_t height = std::stoull(parts[1]);
                auto block = blockchain->get_block(height);
                if (block) {
                    print_block(*block);
                } else {
                    std::cout << "❌ Block #" << height << " not found" << std::endl;
                }
            } catch (...) {
                std::cout << "❌ Invalid block height" << std::endl;
            }
        } else {
            print_block(blockchain->get_latest_block());
        }
    } else if (cmd == "chain") {
        std::lock_guard<std::mutex> lock(blockchain_mutex);
        print_chain(*blockchain);
    } else if (cmd == "transfer") {
        if (parts.size() >= 4) {
            std::string sender = parts[1];
            std::string receiver = parts[2];
            uint64_t amount = std::stoull(parts[3]);
            
            std::lock_guard<std::mutex> lock(blockchain_mutex);
            uint64_t nonce = blockchain->state.get_nonce(sender);
            
            Transaction tx(sender, receiver, amount, nonce, 21000, 1);
            
            if (blockchain->add_transaction(tx)) {
                std::cout << "✅ Transaction added to pool: " << tx.tx_id << std::endl;
                std::cout << "   " << sender << " -> " << receiver << " (" << amount << ")" << std::endl;
            } else {
                std::cout << "❌ Transaction validation failed" << std::endl;
            }
        } else {
            std::cout << "Usage: transfer <sender> <receiver> <amount>" << std::endl;
        }
    } else if (cmd == "gasless") {
        if (parts.size() >= 4) {
            std::string sender = parts[1];
            std::string receiver = parts[2];
            uint64_t amount = std::stoull(parts[3]);
            
            std::lock_guard<std::mutex> lock(blockchain_mutex);
            uint64_t nonce = blockchain->state.get_nonce(sender);
            
            Transaction tx(sender, receiver, amount, nonce, std::nullopt, std::nullopt);
            
            if (blockchain->add_transaction(tx)) {
                std::cout << "✅ Gasless transaction added to pool: " << tx.tx_id << std::endl;
                std::cout << "   " << sender << " -> " << receiver << " (" << amount << ")" << std::endl;
            } else {
                std::cout << "❌ Transaction validation failed" << std::endl;
            }
        } else {
            std::cout << "Usage: gasless <sender> <receiver> <amount>" << std::endl;
        }
    } else if (cmd == "llm") {
        if (parts.size() >= 2) {
            std::string text;
            for (size_t i = 1; i < parts.size(); ++i) {
                if (i > 1) text += " ";
                text += parts[i];
            }
            
            std::lock_guard<std::mutex> lock(blockchain_mutex);
            ParsedTransaction parsed = parser.parse(text, "alice");
            
            std::cout << "🤖 Parsed transaction:" << std::endl;
            std::cout << "   Confidence: " << parsed.confidence << std::endl;
            std::cout << "   Explanation: " << parsed.explanation << std::endl;
            std::cout << "   Sender: " << (parsed.sender ? parsed.sender.value() : "none") << std::endl;
            std::cout << "   Receiver: " << parsed.receiver << std::endl;
            std::cout << "   Amount: " << parsed.amount << std::endl;
            std::cout << "   Gas: " << (parsed.gas_limit ? std::to_string(*parsed.gas_limit) : "none") << std::endl;
            
            if (parsed.confidence > 0.5) {
                uint64_t nonce = blockchain->state.get_nonce(parsed.sender.value_or("alice"));
                Transaction tx(parsed.sender.value_or("alice"), parsed.receiver, 
                             parsed.amount, nonce, parsed.gas_limit, parsed.gas_price);
                
                if (blockchain->add_transaction(tx)) {
                    std::cout << "✅ Transaction added to pool: " << tx.tx_id << std::endl;
                } else {
                    std::cout << "❌ Transaction validation failed" << std::endl;
                }
            }
        } else {
            std::cout << "Usage: llm <natural language command>" << std::endl;
            std::cout << "Example: llm transfer 100 from alice to bob" << std::endl;
            std::cout << "Example: llm send 50 to bob gasless" << std::endl;
        }
    } else if (cmd == "mine") {
        std::lock_guard<std::mutex> lock(blockchain_mutex);
        std::vector<Transaction> txs = blockchain->transaction_pool;
        
        if (txs.empty()) {
            std::cout << "⚠️  No transactions in pool" << std::endl;
            return;
        }
        
        auto block = blockchain->create_block(txs);
        if (block) {
            std::cout << "⛏️  Mined block #" << block->height 
                      << " with " << block->transactions.size() << " transactions" << std::endl;
            print_block(*block);
        } else {
            std::cout << "❌ Failed to create block - invalid transactions" << std::endl;
        }
    } else if (cmd == "pool") {
        std::lock_guard<std::mutex> lock(blockchain_mutex);
        std::cout << "📦 Transaction Pool (" << blockchain->transaction_pool.size() << " transactions)" << std::endl;
        for (const auto& tx : blockchain->transaction_pool) {
            std::cout << "   " << tx.sender << " -> " << tx.receiver 
                      << " (" << tx.amount << ") [" << tx.tx_id.substr(0, 8) << "]" << std::endl;
        }
    } else if (cmd == "gas") {
        if (parts.size() >= 2) {
            try {
                uint64_t price = std::stoull(parts[1]);
                std::lock_guard<std::mutex> lock(blockchain_mutex);
                blockchain->set_gas_price(price);
                std::cout << "⛽ Gas price set to " << price << std::endl;
            } catch (...) {
                std::cout << "❌ Invalid gas price" << std::endl;
            }
        } else {
            std::lock_guard<std::mutex> lock(blockchain_mutex);
            std::cout << "⛽ Current gas price: " << blockchain->get_gas_price() << std::endl;
        }
    } else if (cmd == "exit" || cmd == "quit") {
        std::cout << "👋 Goodbye!" << std::endl;
        mining_active = false;
        std::exit(0);
    } else {
        std::cout << "❓ Unknown command. Type 'help' for available commands." << std::endl;
    }
}

int main() {
    std::cout << "🔗 LLM-Mina-Chain Node v0.1.0" << std::endl;
    std::cout << "================================" << std::endl << std::endl;
    
    // Initialize blockchain
    Blockchain blockchain;
    LLMTransactionParser parser;
    
    std::cout << "✅ Blockchain initialized" << std::endl;
    std::cout << "📊 Current state:" << std::endl;
    print_state(blockchain);
    
    // Start mining thread
    std::thread miner(mining_thread, &blockchain);
    miner.detach();
    
    // Main loop
    std::string input;
    while (std::getline(std::cin, input)) {
        input.erase(0, input.find_first_not_of(" \t\n\r"));
        input.erase(input.find_last_not_of(" \t\n\r") + 1);
        
        if (input.empty()) continue;
        
        handle_command(input, &blockchain, parser);
    }
    
    mining_active = false;
    return 0;
}
