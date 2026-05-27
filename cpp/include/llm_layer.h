#ifndef LLM_LAYER_H
#define LLM_LAYER_H

#include <string>
#include <vector>
#include <optional>
#include <set>
#include <regex>
#include "core.h"

namespace llm_mina_chain {

// Parsed transaction from natural language
struct ParsedTransaction {
    std::optional<std::string> sender;
    std::string receiver;
    uint64_t amount;
    std::optional<uint64_t> gas_limit;
    std::optional<uint64_t> gas_price;
    std::string tx_type;
    std::string data;  // JSON string
    double confidence;
    std::string explanation;
    
    ParsedTransaction() : amount(0), confidence(0.0) {}
};

// Gas suggestion
struct GasSuggestion {
    uint64_t gas_limit;
    uint64_t gas_price;
    
    GasSuggestion() : gas_limit(21000), gas_price(1) {}
    GasSuggestion(uint64_t limit, uint64_t price) : gas_limit(limit), gas_price(price) {}
};

// LLM transaction parser
class LLMTransactionParser {
public:
    LLMTransactionParser();
    
    // Parse natural language into transaction
    ParsedTransaction parse(const std::string& text, 
                            const std::optional<std::string>& default_sender = std::nullopt) const;
    
    // Validate transaction semantics
    std::pair<bool, std::string> validate_semantics(const Transaction& tx, 
                                                     const State& state) const;
    
    // Suggest gas parameters
    GasSuggestion suggest_gas(const Transaction& tx) const;
    
    // Generate natural language explanation
    std::string explain(const Transaction& tx) const;
    
private:
    std::set<std::string> accounts_;
    std::vector<std::regex> transfer_patterns_;
    std::vector<std::regex> gas_patterns_;
};

} // namespace llm_mina_chain

#endif // LLM_LAYER_H
