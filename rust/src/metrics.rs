//! Monitoring and metrics using Prometheus

use prometheus::{Histogram, IntCounter, IntGauge, Registry, Encoder};
use std::sync::Arc;
use std::time::Instant;

/// Blockchain metrics
pub struct BlockchainMetrics {
    // Block metrics
    pub blocks_produced: IntCounter,
    pub blocks_received: IntCounter,
    pub block_height: IntGauge,
    pub block_production_duration: Histogram,
    
    // Transaction metrics
    pub transactions_submitted: IntCounter,
    pub transactions_processed: IntCounter,
    pub transactions_pool_size: IntGauge,
    pub transaction_validation_duration: Histogram,
    
    // Network metrics
    pub peers_connected: IntGauge,
    pub messages_sent: IntCounter,
    pub messages_received: IntCounter,
    pub network_latency: Histogram,
    
    // Consensus metrics
    pub consensus_rounds: IntCounter,
    pub consensus_duration: Histogram,
    pub view_changes: IntCounter,
    
    // Storage metrics
    pub storage_reads: IntCounter,
    pub storage_writes: IntCounter,
    pub storage_read_duration: Histogram,
    pub storage_write_duration: Histogram,
    
    // System metrics
    pub memory_usage: IntGauge,
    pub uptime: IntGauge,
}

impl BlockchainMetrics {
    pub fn new(registry: &Registry) -> Self {
        let blocks_produced = IntCounter::new(
            "blockchain_blocks_produced_total",
            "Total number of blocks produced",
        ).unwrap();
        registry.register(Box::new(blocks_produced.clone())).ok();

        let block_height = IntGauge::new(
            "blockchain_block_height",
            "Current blockchain height",
        ).unwrap();
        registry.register(Box::new(block_height.clone())).ok();

        BlockchainMetrics {
            blocks_produced,
            blocks_received: IntCounter::new(
                "blockchain_blocks_received_total",
                "Total number of blocks received from peers",
            ).unwrap(),
            block_height,
            block_production_duration: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "blockchain_block_production_duration_seconds",
                    "Time spent producing a block",
                )
                .buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0]),
            )
            .unwrap(),
            
            transactions_submitted: IntCounter::new(
                "blockchain_transactions_submitted_total",
                "Total number of transactions submitted",
            ).unwrap(),
            transactions_processed: IntCounter::new(
                "blockchain_transactions_processed_total",
                "Total number of transactions processed",
            ).unwrap(),
            transactions_pool_size: IntGauge::new(
                "blockchain_transaction_pool_size",
                "Current transaction pool size",
            ).unwrap(),
            transaction_validation_duration: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "blockchain_transaction_validation_duration_seconds",
                    "Time spent validating a transaction",
                )
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1]),
            )
            .unwrap(),
            
            peers_connected: IntGauge::new(
                "blockchain_peers_connected",
                "Number of connected peers",
            ).unwrap(),
            messages_sent: IntCounter::new(
                "blockchain_messages_sent_total",
                "Total number of messages sent",
            ).unwrap(),
            messages_received: IntCounter::new(
                "blockchain_messages_received_total",
                "Total number of messages received",
            ).unwrap(),
            network_latency: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "blockchain_network_latency_seconds",
                    "Network latency to peers",
                )
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
            )
            .unwrap(),
            
            consensus_rounds: IntCounter::new(
                "blockchain_consensus_rounds_total",
                "Total number of consensus rounds",
            ).unwrap(),
            consensus_duration: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "blockchain_consensus_duration_seconds",
                    "Time spent in consensus",
                )
                .buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0]),
            )
            .unwrap(),
            view_changes: IntCounter::new(
                "blockchain_consensus_view_changes_total",
                "Total number of view changes",
            ).unwrap(),
            
            storage_reads: IntCounter::new(
                "blockchain_storage_reads_total",
                "Total number of storage reads",
            ).unwrap(),
            storage_writes: IntCounter::new(
                "blockchain_storage_writes_total",
                "Total number of storage writes",
            ).unwrap(),
            storage_read_duration: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "blockchain_storage_read_duration_seconds",
                    "Time spent reading from storage",
                )
                .buckets(vec![0.0001, 0.001, 0.01, 0.1]),
            )
            .unwrap(),
            storage_write_duration: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "blockchain_storage_write_duration_seconds",
                    "Time spent writing to storage",
                )
                .buckets(vec![0.0001, 0.001, 0.01, 0.1]),
            )
            .unwrap(),
            
            memory_usage: IntGauge::new(
                "blockchain_memory_usage_bytes",
                "Memory usage in bytes",
            ).unwrap(),
            uptime: IntGauge::new(
                "blockchain_uptime_seconds",
                "Node uptime in seconds",
            ).unwrap(),
        }
    }
}

/// Metrics server for exposing Prometheus metrics
pub struct MetricsServer {
    registry: Arc<Registry>,
    metrics: BlockchainMetrics,
    start_time: Instant,
}

impl MetricsServer {
    pub fn new(registry: Arc<Registry>) -> Self {
        let metrics = BlockchainMetrics::new(&registry);
        MetricsServer {
            registry,
            metrics,
            start_time: Instant::now(),
        }
    }
    
    /// Get the metrics
    pub fn metrics(&self) -> &BlockchainMetrics {
        &self.metrics
    }
    
    /// Update uptime metric
    pub fn update_uptime(&self) {
        self.metrics
            .uptime
            .set(self.start_time.elapsed().as_secs() as i64);
    }
    
    /// Export metrics in Prometheus format
    pub fn export(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
    
    /// Serve metrics via HTTP
    pub async fn serve_metrics(&self, addr: String) -> Result<(), Box<dyn std::error::Error>> {
        use hyper::{Body, Request, Response, Server};
        use hyper::service::{make_service_fn, service_fn};
        use std::convert::Infallible;
        
        let registry = self.registry.clone();
        
        let make_svc = make_service_fn(move |_conn| {
            let registry = registry.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |_req: Request<Body>| {
                    let registry = registry.clone();
                    async move {
                        let encoder = prometheus::TextEncoder::new();
                        let metric_families = registry.gather();
                        let mut buffer = Vec::new();
                        encoder.encode(&metric_families, &mut buffer).unwrap();
                        Ok::<_, Infallible>(Response::new(Body::from(buffer)))
                    }
                }))
            }
        });
        
        let addr = addr.parse()?;
        Server::bind(&addr).serve(make_svc).await?;
        
        Ok(())
    }
}

/// Timer for measuring duration
pub struct Timer {
    start: Instant,
    histogram: Histogram,
}

impl Timer {
    pub fn new(histogram: Histogram) -> Self {
        Timer {
            start: Instant::now(),
            histogram,
        }
    }
    
    pub fn observe(self) {
        let duration = self.start.elapsed().as_secs_f64();
        self.histogram.observe(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_export() {
        let registry = Registry::new();
        let metrics = BlockchainMetrics::new(&registry);
        
        metrics.blocks_produced.inc();
        metrics.block_height.set(10);
        
        let encoder = prometheus::TextEncoder::new();
        let metric_families = registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("blockchain_blocks_produced_total"));
        assert!(output.contains("blockchain_block_height"));
    }
}
