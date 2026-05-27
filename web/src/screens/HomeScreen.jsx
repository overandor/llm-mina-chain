import React from 'react'
import { ArrowRight, MessageSquare, Smartphone, Search } from 'lucide-react'

export default function HomeScreen() {
  return (
    <div className="space-y-8">
      <div className="glass-card proof-glow p-8">
        <h1 className="text-4xl font-bold text-white mb-4">MEMBRA Instant Proof Chain</h1>
        <p className="text-xl text-violet-400 font-mono mb-6">Sending is work. Receiving may sponsor gas.</p>
        <p className="text-gray-400">A Rust-first micro proof-chain for LLM-assisted intent transactions.</p>
      </div>

      <div className="grid md:grid-cols-3 gap-6">
        <button className="glass-card p-6 hover:bg-graphite-700/50">
          <MessageSquare className="text-violet-400 mb-4" size={32} />
          <h3 className="text-lg font-semibold">Create Intent</h3>
          <p className="text-gray-400 text-sm mt-2">Parse natural language to transactions</p>
        </button>

        <button className="glass-card p-6 hover:bg-graphite-700/50">
          <Smartphone className="text-electric-blue mb-4" size={32} />
          <h3 className="text-lg font-semibold">Browser Mining</h3>
          <p className="text-gray-400 text-sm mt-2">Stay on page to become a node</p>
        </button>

        <button className="glass-card p-6 hover:bg-graphite-700/50">
          <Search className="text-green-400 mb-4" size={32} />
          <h3 className="text-lg font-semibold">Inspect Proof</h3>
          <p className="text-gray-400 text-sm mt-2">Explore transaction provenance</p>
        </button>
      </div>

      <div className="glass-card p-6">
        <h2 className="text-lg font-semibold mb-4">System Status</h2>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
          <div><span className="status-verified">✓</span> Release build</div>
          <div><span className="status-warning">⚠</span> Tests: 15/18</div>
          <div><span className="status-verified">✓</span> Storage</div>
          <div><span className="status-verified">✓</span> Metrics</div>
          <div><span className="status-placeholder">○</span> Crypto (placeholder)</div>
          <div><span className="status-disabled">⊘</span> P2P (disabled)</div>
          <div><span className="status-disabled">⊘</span> Consensus</div>
          <div><span className="status-disabled">⊘</span> zk-SNARKs</div>
        </div>
      </div>
    </div>
  )
}
