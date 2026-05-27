#include "llm_layer.h"
#include <sstream>
#include <algorithm>

namespace llm_mina_chain {

LLMTransactionParser::LLMTransactionParser() {
    accounts_ = {"alice", "bob", "charlie", "genesis"};
    
    // Transfer patterns
    transfer_patterns_ = {
        std::regex(R"(transfer\s+(\d+)\s+from\s+(\w+)\s+to\s+(\w+))", 
                   std::regex_constants::icase),
        std::regex(R"(send\s+(\d+)\s+from\s+(\w+)\s+to\s+(\w+))", 
                   std::regex_constants::icase),
        std::regex(R"((\w+)\s+sends?\s+(\d+)\s+to\s+(\w+))", 
                   std::regex_constants::icase),
        std::regex(R"(pay\s+(\w+)\s+(\d+))", 
                   std::regex_constants::icase),
        std::regex(R"(give\s+(\w+)\s+(\d+))", 
                   std::regex_constants::icase),
    };
    
    // Gas patterns
    gas_patterns_ = {
        std::regex(R"(gas\s+limit\s+(\d+))", std::regex_constants::icase),
        std::regex(R"(gas\s+price\s+(\d+))", std::regex_constants::icase),
        std::regex(R"(with\s+(\d+)\s+gas)", std::regex_constants::icase),
    };
}

ParsedTransaction LLMTransactionParser::parse(const std::string& text,
                                               const std::optional<std::string>& default_sender) const {
    std::string text_lower = text;
    std::transform(text_lower.begin(), text_lower.end(), text_lower.begin(), ::tolower);
    
    ParsedTransaction parsed;
    parsed.sender = default_sender;
    parsed.tx_type = "transfer";
    parsed.confidence = 0.0;
    
    // Try to match transfer patterns
    for (const auto& pattern : transfer_patterns_) {
        std::smatch match;
        if (std::regex_search(text_lower, match, pattern)) {
            if (match.size() >= 4) {
                // Pattern: transfer X from A to B
                parsed.amount = std::stoull(match[1].str());
                std::string sender_candidate = match[2].str();
                parsed.sender = (accounts_.find(sender_candidate) != accounts_.end()) 
                    ? sender_candidate : default_sender;
                parsed.receiver = match[3].str();
            } else if (match.size() >= 3) {
                // Pattern: A sends X to B or pay B X
                std::string first = match[1].str();
                if (accounts_.find(first) != accounts_.end()) {
                    parsed.sender = first;
                    parsed.amount = std::stoull(match[2].str());
                } else {
                    parsed.receiver = first;
                    parsed.amount = std::stoull(match[2].str());
                    parsed.sender = default_sender;
                }
            }
            
            parsed.confidence = 0.8;
            std::stringstream ss;
            ss << "Transfer " << parsed.amount << " from " 
               << (parsed.sender.value_or("unknown")) << " to " << parsed.receiver;
            parsed.explanation = ss.str();
            break;
        }
    }
    
    // Check for gasless
    if (text_lower.find("gasless") != std::string::npos || 
        text_lower.find("no gas") != std::string::npos) {
        parsed.gas_limit = std::nullopt;
        parsed.gas_price = std::nullopt;
        parsed.explanation += " (gasless transaction)";
    } else {
        // Try to match gas patterns
        for (const auto& pattern : gas_patterns_) {
            std::smatch match;
            if (std::regex_search(text_lower, match, pattern)) {
                if (match.size() >= 2) {
                    if (text_lower.find("gas limit") != std::string::npos) {
                        parsed.gas_limit = std::stoull(match[1].str());
                    } else if (text_lower.find("gas price") != std::string::npos) {
                        parsed.gas_price = std::stoull(match[1].str());
                    }
                }
            }
        }
    }
    
    // Validate receiver
    if (!parsed.receiver.empty() && accounts_.find(parsed.receiver) == accounts_.end()) {
        parsed.confidence = std::max(0.0, parsed.confidence - 0.3);
        parsed.explanation += " (warning: receiver '" + parsed.receiver + "' not known)";
    }
    
    return parsed;
}

std::pair<bool, std::string> LLMTransactionParser::validate_semantics(
    const Transaction& tx, const State& state) const {
    
    std::vector<std::string> issues;
    
    // Check amount
    if (tx.amount == 0) {
        issues.push_back("Amount must be positive");
    }
    
    if (tx.amount > 1000000) {
        issues.push_back("Amount suspiciously large");
    }
    
    // Check gas parameters
    if (tx.gas_limit.has_value() && tx.gas_limit.value() < 21000) {
        issues.push_back("Gas limit too low");
    }
    
    if (tx.gas_price.has_value() && tx.gas_price.value() > 1000) {
        issues.push_back("Gas price suspiciously high");
    }
    
    // Check sender/receiver are different
    if (tx.sender == tx.receiver) {
        issues.push_back("Sender and receiver cannot be the same");
    }
    
    // Check context (balance, nonce)
    // Sender must have enough to send the amount (mining work)
    uint64_t sender_balance = state.get_balance(tx.sender);
    if (sender_balance < tx.amount) {
        std::stringstream ss;
        ss << "Insufficient sender balance: " << sender_balance << " < " << tx.amount;
        issues.push_back(ss.str());
    }
    
    // Receiver must afford gas (if gas is specified)
    uint64_t gas_cost = tx.calculate_gas_cost();
    if (gas_cost > 0) {
        uint64_t receiver_balance = state.get_balance(tx.receiver);
        if (receiver_balance < gas_cost) {
            std::stringstream ss;
            ss << "Insufficient receiver balance for gas: " << receiver_balance << " < " << gas_cost;
            issues.push_back(ss.str());
        }
    }
    
    uint64_t sender_nonce = state.get_nonce(tx.sender);
    if (tx.nonce != sender_nonce) {
        std::stringstream ss;
        ss << "Invalid nonce: expected " << sender_nonce << ", got " << tx.nonce;
        issues.push_back(ss.str());
    }
    
    if (issues.empty()) {
        return {true, "Transaction semantics valid"};
    } else {
        std::stringstream ss;
        for (size_t i = 0; i < issues.size(); ++i) {
            if (i > 0) ss << "; ";
            ss << issues[i];
        }
        return {false, ss.str()};
    }
}

GasSuggestion LLMTransactionParser::suggest_gas(const Transaction& tx) const {
    uint64_t base_gas = (tx.tx_type == "contract_call") ? 100000 : 21000;
    return GasSuggestion(base_gas, 1);
}

std::string LLMTransactionParser::explain(const Transaction& tx) const {
    std::stringstream ss;
    
    std::string gas_info;
    if (tx.is_gasless()) {
        gas_info = " (gasless)";
    } else {
        std::stringstream gas_ss;
        gas_ss << " (gas: " << tx.gas_limit.value_or(0) << " * " 
               << tx.gas_price.value_or(0) << " = " << tx.calculate_gas_cost() << ")";
        gas_info = gas_ss.str();
    }
    
    std::string short_id = tx.tx_id.substr(0, std::min(size_t(8), tx.tx_id.length()));
    ss << "Transaction " << short_id << ": " << tx.sender << " sends " << tx.amount 
       << " to " << tx.receiver << gas_info;
    
    if (!tx.data.empty()) {
        ss << " with data: " << tx.data;
    }
    
    return ss.str();
}

} // namespace llm_mina_chain
