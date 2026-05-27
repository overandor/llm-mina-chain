"""
Core blockchain components for LLM-Mina-Chain
"""
import hashlib
import json
import time
from typing import Dict, List, Optional, Any
from dataclasses import dataclass, asdict
from enum import Enum
import secrets


class TransactionType(Enum):
    TRANSFER = "transfer"
    CONTRACT_CALL = "contract_call"
    LLM_GENERATED = "llm_generated"


@dataclass
class Transaction:
    """Atomic transaction with optional gas"""
    tx_id: str
    sender: str
    receiver: str
    amount: int
    nonce: int
    gas_limit: Optional[int] = None
    gas_price: Optional[int] = None
    tx_type: str = "transfer"
    data: Optional[Dict[str, Any]] = None
    signature: Optional[str] = None
    timestamp: float = 0.0
    
    def __post_init__(self):
        if self.timestamp == 0.0:
            self.timestamp = time.time()
    
    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)
    
    def hash(self) -> str:
        """Compute transaction hash"""
        tx_data = {
            "tx_id": self.tx_id,
            "sender": self.sender,
            "receiver": self.receiver,
            "amount": self.amount,
            "nonce": self.nonce,
            "gas_limit": self.gas_limit,
            "gas_price": self.gas_price,
            "tx_type": self.tx_type,
            "data": self.data,
            "timestamp": self.timestamp
        }
        return hashlib.sha256(json.dumps(tx_data, sort_keys=True).encode()).hexdigest()
    
    def is_gasless(self) -> bool:
        """Check if transaction is gasless"""
        return self.gas_limit is None or self.gas_price is None
    
    def calculate_gas_cost(self) -> int:
        """Calculate gas cost (0 if gasless)"""
        if self.is_gasless():
            return 0
        return (self.gas_limit or 0) * (self.gas_price or 0)


@dataclass
class State:
    """Blockchain state (account balances, nonces)"""
    balances: Dict[str, int]
    nonces: Dict[str, int]
    contracts: Dict[str, Dict[str, Any]]
    
    def __init__(self):
        self.balances = {}
        self.nonces = {}
        self.contracts = {}
    
    def get_balance(self, address: str) -> int:
        return self.balances.get(address, 0)
    
    def set_balance(self, address: str, amount: int):
        self.balances[address] = amount
    
    def get_nonce(self, address: str) -> int:
        return self.nonces.get(address, 0)
    
    def increment_nonce(self, address: str):
        self.nonces[address] = self.get_nonce(address) + 1
    
    def apply_transaction(self, tx: Transaction) -> bool:
        """Atomically apply transaction to state"""
        # Check nonce
        if tx.nonce != self.get_nonce(tx.sender):
            return False
        
        # Check balance (including gas cost)
        total_cost = tx.amount + tx.calculate_gas_cost()
        if self.get_balance(tx.sender) < total_cost:
            return False
        
        # Execute atomically
        self.balances[tx.sender] = self.get_balance(tx.sender) - total_cost
        self.balances[tx.receiver] = self.get_balance(tx.receiver) + tx.amount
        self.increment_nonce(tx.sender)
        
        return True
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "balances": self.balances,
            "nonces": self.nonces,
            "contracts": self.contracts
        }
    
    def hash(self) -> str:
        """Compute state hash"""
        return hashlib.sha256(json.dumps(self.to_dict(), sort_keys=True).encode()).hexdigest()


@dataclass
class Block:
    """Block with recursive proof (Mina-like)"""
    height: int
    timestamp: float
    transactions: List[Transaction]
    previous_hash: str
    state_hash: str
    proof: Optional[str] = None
    block_hash: str = ""
    
    def __post_init__(self):
        if not self.block_hash:
            self.block_hash = self.compute_hash()
    
    def compute_hash(self) -> str:
        """Compute block hash"""
        block_data = {
            "height": self.height,
            "timestamp": self.timestamp,
            "transactions": [tx.hash() for tx in self.transactions],
            "previous_hash": self.previous_hash,
            "state_hash": self.state_hash,
            "proof": self.proof
        }
        return hashlib.sha256(json.dumps(block_data, sort_keys=True).encode()).hexdigest()
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "height": self.height,
            "timestamp": self.timestamp,
            "transactions": [tx.to_dict() for tx in self.transactions],
            "previous_hash": self.previous_hash,
            "state_hash": self.state_hash,
            "proof": self.proof,
            "block_hash": self.block_hash
        }


class Blockchain:
    """Recursive blockchain with atomic transactions"""
    
    def __init__(self):
        self.chain: List[Block] = []
        self.state = State()
        self.transaction_pool: List[Transaction] = []
        self.gas_price = 1  # Base gas price
        self.min_gas_price = 0  # Allow gasless transactions
        
        # Genesis block
        self._create_genesis_block()
    
    def _create_genesis_block(self):
        """Create genesis block with initial state"""
        # Initialize with some accounts
        self.state.set_balance("genesis", 1000000)
        self.state.set_balance("alice", 1000)
        self.state.set_balance("bob", 1000)
        
        genesis = Block(
            height=0,
            timestamp=time.time(),
            transactions=[],
            previous_hash="0" * 64,
            state_hash=self.state.hash(),
            proof="genesis_proof"
        )
        genesis.block_hash = genesis.compute_hash()
        self.chain.append(genesis)
    
    def get_latest_block(self) -> Block:
        return self.chain[-1]
    
    def get_block(self, height: int) -> Optional[Block]:
        if 0 <= height < len(self.chain):
            return self.chain[height]
        return None
    
    def add_transaction(self, tx: Transaction) -> bool:
        """Add transaction to pool with immediate validation"""
        # Basic validation
        if not self._validate_transaction(tx):
            return False
        
        self.transaction_pool.append(tx)
        return True
    
    def _validate_transaction(self, tx: Transaction) -> bool:
        """Validate transaction"""
        # Check nonce
        if tx.nonce != self.state.get_nonce(tx.sender):
            return False
        
        # Check balance
        total_cost = tx.amount + tx.calculate_gas_cost()
        if self.state.get_balance(tx.sender) < total_cost:
            return False
        
        # If gas is specified, check gas price
        if not tx.is_gasless() and tx.gas_price < self.min_gas_price:
            return False
        
        return True
    
    def create_block(self, transactions: List[Transaction]) -> Optional[Block]:
        """Create new block with atomic transaction execution"""
        # Create a copy of state for testing
        test_state = State()
        test_state.balances = self.state.balances.copy()
        test_state.nonces = self.state.nonces.copy()
        test_state.contracts = self.state.contracts.copy()
        
        # Try to apply all transactions atomically
        valid_txs = []
        for tx in transactions:
            if test_state.apply_transaction(tx):
                valid_txs.append(tx)
            else:
                # If any transaction fails, rollback all
                return None
        
        # All transactions valid - create block
        new_block = Block(
            height=self.get_latest_block().height + 1,
            timestamp=time.time(),
            transactions=valid_txs,
            previous_hash=self.get_latest_block().block_hash,
            state_hash=test_state.hash(),
            proof=self._generate_proof(test_state)
        )
        
        # Update actual state
        self.state = test_state
        self.chain.append(new_block)
        
        # Remove from pool
        for tx in valid_txs:
            if tx in self.transaction_pool:
                self.transaction_pool.remove(tx)
        
        return new_block
    
    def _generate_proof(self, state: State) -> str:
        """Generate recursive proof (simplified - in real Mina this would be a SNARK)"""
        # In a real implementation, this would generate a zk-SNARK proof
        # For this micro version, we use a hash as a placeholder
        proof_data = {
            "state_hash": state.hash(),
            "block_height": self.get_latest_block().height + 1,
            "timestamp": time.time(),
            "random": secrets.token_hex(16)
        }
        return hashlib.sha256(json.dumps(proof_data, sort_keys=True).encode()).hexdigest()
    
    def get_state(self) -> Dict[str, Any]:
        return self.state.to_dict()
    
    def get_chain(self) -> List[Dict[str, Any]]:
        return [block.to_dict() for block in self.chain]
    
    def set_gas_price(self, price: int):
        """Update gas price"""
        self.gas_price = max(price, self.min_gas_price)
    
    def get_gas_price(self) -> int:
        return self.gas_price
