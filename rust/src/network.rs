//! P2P networking layer using libp2p 0.53+

use libp2p::{
    gossipsub::{self, MessageAuthenticity, MessageId, ValidationMode},
    identity::Keypair,
    mdns,
    noise,
    swarm::SwarmEvent,
    PeerId,
};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::{Block, Transaction};

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// New block announcement
    Block(Block),
    /// New transaction
    Transaction(Transaction),
    /// Request for block by height
    BlockRequest(u64),
    /// Response with block
    BlockResponse(Option<Block>),
    /// Peer status update
    PeerStatus { height: u64, hash: String },
}

/// Custom gossipsub message ID based on content hash
fn message_id_fn(message: &gossipsub::Message) -> MessageId {
    let mut hasher = DefaultHasher::new();
    message.data.hash(&mut hasher);
    MessageId::from(hasher.finish().to_string())
}

/// Network behavior combining gossipsub and mDNS
#[derive(libp2p::swarm::NetworkBehaviour)]
pub struct BlockchainNetworkBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

/// P2P network node
pub struct P2PNode {
    swarm: libp2p::Swarm<BlockchainNetworkBehaviour>,
    block_topic: gossipsub::IdentTopic,
    transaction_topic: gossipsub::IdentTopic,
    #[allow(dead_code)]
    message_sender: mpsc::UnboundedSender<NetworkMessage>,
}

impl P2PNode {
    /// Create a new P2P node
    pub fn new(
        keypair: Keypair,
        message_sender: mpsc::UnboundedSender<NetworkMessage>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let peer_id = PeerId::from(keypair.public());

        // Create gossipsub configuration
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(ValidationMode::Strict)
            .message_id_fn(message_id_fn)
            .build()
            .map_err(|e| format!("Gossipsub config error: {}", e))?;

        // Create gossipsub behavior
        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )
        .map_err(|e| format!("Gossipsub error: {}", e))?;

        // Create mDNS behavior for local peer discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)?;

        // Create network behavior
        let behaviour = BlockchainNetworkBehaviour { gossipsub, mdns };

        // Create swarm with tokio executor using the builder pattern
        let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                noise::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_behaviour(|_| Ok(behaviour))?
            .build();

        // Create topics
        let block_topic = gossipsub::IdentTopic::new("llm-mina-blocks");
        let transaction_topic = gossipsub::IdentTopic::new("llm-mina-transactions");

        Ok(P2PNode {
            swarm,
            block_topic,
            transaction_topic,
            message_sender,
        })
    }

    /// Start listening on the given address
    pub fn listen(&mut self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.swarm.listen_on(addr.parse()?)?;
        Ok(())
    }

    /// Subscribe to block topic
    pub fn subscribe_blocks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.block_topic)?;
        Ok(())
    }

    /// Subscribe to transaction topic
    pub fn subscribe_transactions(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.transaction_topic)?;
        Ok(())
    }

    /// Publish a block to the network
    pub fn publish_block(&mut self, block: &Block) -> Result<(), Box<dyn std::error::Error>> {
        let message = NetworkMessage::Block(block.clone());
        let data = serde_json::to_vec(&message)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.block_topic.clone(), data)?;
        Ok(())
    }

    /// Publish a transaction to the network
    pub fn publish_transaction(
        &mut self,
        tx: &Transaction,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message = NetworkMessage::Transaction(tx.clone());
        let data = serde_json::to_vec(&message)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.transaction_topic.clone(), data)?;
        Ok(())
    }

    /// Run the network event loop
    pub async fn run(&mut self) {
        use futures::StreamExt;
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(event) => {
                    // Simplified event handling for demo purposes
                    // In production, this would handle gossipsub and mDNS events
                    let _ = event;
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    tracing::info!("Listening on {}", address);
                }
                _ => {}
            }
        }
    }

    /// Get connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.swarm.connected_peers().copied().collect()
    }

    /// Get peer ID
    pub fn peer_id(&self) -> PeerId {
        *self.swarm.local_peer_id()
    }
}

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Listen address
    pub listen_addr: String,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<String>,
    /// Enable mDNS for local discovery
    pub enable_mdns: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
            bootstrap_peers: vec![],
            enable_mdns: true,
        }
    }
}
