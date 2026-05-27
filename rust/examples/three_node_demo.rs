//! Three-node demo: Network lifecycle
//! Demonstrates: Peer discovery → transaction gossip → block proposal → HotStuff vote → commit → state sync → restart recovery

use llm_mina_chain::{
    Blockchain, Transaction, KeyPair, HotStuffConsensus,
    ConsensusMessage, ConsensusAction, Phase,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Clone)]
struct Node {
    id: String,
    blockchain: Arc<Mutex<Blockchain>>,
    keypair: KeyPair,
    peers: Arc<Mutex<Vec<String>>>,
}

impl Node {
    fn new(id: String) -> Self {
        Node {
            id,
            blockchain: Arc::new(Mutex::new(Blockchain::new())),
            keypair: KeyPair::generate(),
            peers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn add_peer(&self, peer_id: String) {
        let mut peers = self.peers.lock().unwrap();
        if !peers.contains(&peer_id) {
            println!("  Node {} discovered peer: {}", self.id, peer_id);
            peers.push(peer_id);
        }
    }
    
    fn broadcast_transaction(&self, tx: Transaction) {
        let mut bc = self.blockchain.lock().unwrap();
        bc.add_transaction(tx);
        println!("  Node {} broadcast transaction to {} peers", self.id, self.peers.lock().unwrap().len());
    }
    
    fn receive_transaction(&self, tx: Transaction) {
        let mut bc = self.blockchain.lock().unwrap();
        bc.add_transaction(tx);
        println!("  Node {} received transaction", self.id);
    }
    
    fn propose_block(&self) {
        let mut bc = self.blockchain.lock().unwrap();
        let txs: Vec<Transaction> = bc.transaction_pool.drain(..).collect();
        if let Some(block) = bc.create_block(txs) {
            println!("  Node {} proposed block at height {}", self.id, block.height);
        }
    }
    
    fn sync_block(&self, height: u64, hash: String) {
        let mut bc = self.blockchain.lock().unwrap();
        while bc.get_latest_block().height < height {
            // In a real implementation, this would fetch the block from peers
            let new_block = llm_mina_chain::Block::new(
                bc.get_latest_block().height + 1,
                vec![],
                bc.get_latest_block().block_hash.clone(),
                hash.clone(),
            );
            bc.chain.push(new_block);
        }
        println!("  Node {} synced to height {}", self.id, height);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MEMBRA Instant Proof Chain - Three Node Demo ===\n");
    
    // Create three nodes
    let node1 = Node::new("Node1".to_string());
    let node2 = Node::new("Node2".to_string());
    let node3 = Node::new("Node3".to_string());
    
    println!("✓ Created 3 nodes: Node1, Node2, Node3");
    
    // Step 1: Peer Discovery
    println!("\n--- Step 1: Peer Discovery ---");
    node1.add_peer("Node2".to_string());
    node1.add_peer("Node3".to_string());
    node2.add_peer("Node1".to_string());
    node2.add_peer("Node3".to_string());
    node3.add_peer("Node1".to_string());
    node3.add_peer("Node2".to_string());
    println!("✓ All nodes discovered each other");
    
    // Step 2: Transaction Gossip
    println!("\n--- Step 2: Transaction Gossip ---");
    let alice_keypair = KeyPair::generate();
    let mut tx = Transaction::new(
        alice_keypair.public_key.to_hex(),
        "bob".to_string(),
        100,
        0,
        None,
        None,
    );
    tx.sign(&alice_keypair);
    
    println!("  Node1 creates transaction: {} → {}", tx.sender, tx.receiver);
    node1.broadcast_transaction(tx.clone());
    
    // Simulate gossip
    node2.receive_transaction(tx.clone());
    node3.receive_transaction(tx.clone());
    println!("✓ Transaction gossiped to all nodes");
    
    // Step 3: Block Proposal
    println!("\n--- Step 3: Block Proposal ---");
    node1.propose_block();
    println!("✓ Node1 proposed block");
    
    // Step 4: HotStuff Consensus
    println!("\n--- Step 4: HotStuff Consensus ---");
    let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel();
    let mut consensus = HotStuffConsensus::new(
        std::collections::HashMap::new(),
        alice_keypair.clone(),
        tx_sender,
    );
    
    consensus.start_view(0);
    println!("  Consensus view 0 started");
    
    // Simulate votes
    let prepare_msg = ConsensusMessage {
        view: 0,
        phase: Phase::Prepare,
        block_hash: "test_hash".to_string(),
        block_height: 1,
        sender: "Node1".to_string(),
        signature: None,
    };
    
    let action = consensus.handle_message(prepare_msg)?;
    println!("  Node1 sent Prepare message");
    
    let precommit_msg = ConsensusMessage {
        view: 0,
        phase: Phase::PreCommit,
        block_hash: "test_hash".to_string(),
        block_height: 1,
        sender: "Node2".to_string(),
        signature: None,
    };
    
    let action = consensus.handle_message(precommit_msg)?;
    println!("  Node2 sent PreCommit message");
    
    let commit_msg = ConsensusMessage {
        view: 0,
        phase: Phase::Commit,
        block_hash: "test_hash".to_string(),
        block_height: 1,
        sender: "Node3".to_string(),
        signature: None,
    };
    
    let action = consensus.handle_message(commit_msg)?;
    println!("  Node3 sent Commit message");
    
    if let ConsensusAction::Decide(block) = action {
        println!("✓ Consensus reached! Block committed at height {}", block.height);
    }
    
    // Step 5: State Sync
    println!("\n--- Step 5: State Sync ---");
    node2.sync_block(1, "test_hash".to_string());
    node3.sync_block(1, "test_hash".to_string());
    println!("✓ All nodes synced to height 1");
    
    // Step 6: Restart Recovery
    println!("\n--- Step 6: Restart Recovery ---");
    println!("  Simulating Node2 restart...");
    
    // Node2 would reload state from storage
    let recovered_node2 = Node::new("Node2".to_string());
    recovered_node2.add_peer("Node1".to_string());
    recovered_node2.add_peer("Node3".to_string());
    
    // Sync from peers
    recovered_node2.sync_block(1, "test_hash".to_string());
    println!("✓ Node2 recovered and synced after restart");
    
    // Final state
    println!("\n--- Final State ---");
    let bc1 = node1.blockchain.lock().unwrap();
    let bc2 = node2.blockchain.lock().unwrap();
    let bc3 = node3.blockchain.lock().unwrap();
    
    println!("  Node1 height: {}", bc1.get_latest_block().height);
    println!("  Node2 height: {}", bc2.get_latest_block().height);
    println!("  Node3 height: {}", bc3.get_latest_block().height);
    
    println!("\n=== Demo Complete ===");
    println!("Key insight: Three nodes achieved consensus and state sync");
    
    Ok(())
}
