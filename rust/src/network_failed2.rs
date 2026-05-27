//! Simplified P2P networking layer using libp2p (updated for current API)

use libp2p::{
    gossipsub::{self, MessageId, IdentTopic},
    identity::Keypair,
    mdns,
    noise,
    swarm::{Swarm, SwarmEvent},
    tcp, yamux, PeerId, Transport,
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
fn message_id(message: &gossipsub::Message) -> MessageId {
    let mut hasher = DefaultHasher::new();
    message.data.hash(&mut hasher);
    MessageId::from(hasher.finish().to_string())
}

/// P2P network node (simplified without NetworkBehaviour derive)
pub struct P2PNode {
    swarm: Swarm<libp2p::swarm::behaviour::Behaviour<gossipsub::Behaviour, mdns::tokio::Behaviour>>,
    block_topic: IdentTopic,
    transaction_topic: IdentTopic,
    message_sender: mpsc::UnboundedSender<NetworkMessage>,
}

impl P2PNode {
    /// Create a new P2P node
    pub async fn new(
        keypair: Keypair,
        message_sender: mpsc::UnboundedSender<NetworkMessage>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create gossipsub configuration
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(message_id)
            .build()
            .map_err(|e| format!("Gossipsub config error: {}", e))?;

        // Create gossipsub behavior
        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )
        .map_err(|e| format!("Gossipsub error: {}", e))?;

        // Create mDNS behavior for local peer discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), keypair.public().to_peer_id())
            .map_err(|e| format!("mDNS error: {}", e))?;

        // Create combined behavior
        let behaviour = libp2p::swarm::behaviour::Behaviour::new(gossipsub, mdns);

        // Create transport with noise encryption and yamux multiplexing
        let transport = tcp::tokio::Transport::new(tcp::Config::default())
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::Config::new(&keypair).unwrap())
            .multiplex(yamux::Config::default())
            .timeout(Duration::from_secs(20))
            .boxed();

        // Create swarm
        let swarm = libp2p::swarm::SwarmBuilder::with_tokio_executor(transport, behaviour, keypair.public().to_peer_id())
            .build();

        // Create topics using IdentTopic
        let block_topic = IdentTopic::new("llm-mina-blocks");
        let transaction_topic = IdentTopic::new("llm-mina-transactions");

        Ok(P2PNode {
            swarm,
            block_topic,
            transaction_topic,
            message_sender,
        })
    }

    /// Start listening on the given address
    pub async fn listen(&mut self, addr: String) -> Result<(), Box<dyn std::error::Error>> {
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
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(event) => {
                    // Handle gossipsub and mDNS events
                    match event {
                        libp2p::swarm::behaviour::Event::Gossipsub(gossipsub::Event::Message {
                            propagation_source: _,
                            message_id: _,
                            message,
                        }) => {
                            if let Ok(network_message) = serde_json::from_slice::<NetworkMessage>(&message.data) {
                                match network_message {
                                    NetworkMessage::Block(block) => {
                                        let _ = self.message_sender.send(NetworkMessage::Block(block));
                                    }
                                    NetworkMessage::Transaction(tx) => {
                                        let _ = self.message_sender.send(NetworkMessage::Transaction(tx));
                                    }
                                    _ => {}
                                }
                            }
                        }
                        libp2p::swarm::behaviour::Event::Mdns(mdns::Event::Discovered(list)) => {
                            for (peer_id, _addr) in list {
                                self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                            }
                        }
                        libp2p::swarm::behaviour::Event::Mdns(mdns::Event::Expired(list)) => {
                            for (peer_id, _addr) in list {
                                self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                            }
                        }
                        _ => {}
                    }
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {}", address);
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
