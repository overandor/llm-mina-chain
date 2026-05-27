import React, { useState } from 'react'
import { Search, ExternalLink, Clock, Hash } from 'lucide-react'

function ProvenanceExplorer() {
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedBlock, setSelectedBlock] = useState(null)

  // Mock data
  const blocks = [
    { height: 12345, hash: '0x8f3d...2a1b', timestamp: '2024-01-15 14:32:00', txs: 15, miner: '0x7a2b...9c3d' },
    { height: 12344, hash: '0x2e7c...4f9a', timestamp: '2024-01-15 14:31:45', txs: 8, miner: '0x1d8e...6b2f' },
    { height: 12343, hash: '0x9a4f...1c8d', timestamp: '2024-01-15 14:31:30', txs: 23, miner: '0x5e3b...7a2c' },
  ]

  const filteredBlocks = blocks.filter(block =>
    block.hash.toLowerCase().includes(searchQuery.toLowerCase()) ||
    block.height.toString().includes(searchQuery)
  )

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white mb-2">Provenance Explorer</h2>
        <p className="text-gray-400">Explore blockchain history and transaction provenance</p>
      </div>

      {/* Search */}
      <div className="bg-graphite-800 rounded-lg p-4">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400" size={20} />
          <input
            type="text"
            placeholder="Search by block height or hash..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-graphite-900 border border-graphite-700 rounded-lg pl-10 pr-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-violet-500"
          />
        </div>
      </div>

      {/* Block List */}
      <div className="space-y-3">
        {filteredBlocks.map((block) => (
          <div
            key={block.height}
            onClick={() => setSelectedBlock(block)}
            className="bg-graphite-800 rounded-lg p-4 cursor-pointer hover:bg-graphite-700 transition-colors border border-graphite-700 hover:border-violet-500"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-3">
                <Hash className="text-violet-400" size={20} />
                <div>
                  <div className="text-white font-medium">Block #{block.height}</div>
                  <div className="text-gray-400 text-sm">{block.hash}</div>
                </div>
              </div>
              <div className="text-right">
                <div className="text-gray-400 text-sm flex items-center justify-end">
                  <Clock size={14} className="mr-1" />
                  {block.timestamp}
                </div>
                <div className="text-violet-400 text-sm">{block.txs} transactions</div>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Block Detail Modal */}
      {selectedBlock && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={() => setSelectedBlock(null)}>
          <div className="bg-graphite-900 rounded-lg p-6 max-w-2xl w-full mx-4 border border-graphite-700" onClick={e => e.stopPropagation()}>
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-xl font-bold text-white">Block #{selectedBlock.height}</h3>
              <button onClick={() => setSelectedBlock(null)} className="text-gray-400 hover:text-white">
                ✕
              </button>
            </div>
            <div className="space-y-3">
              <div className="flex justify-between">
                <span className="text-gray-400">Hash</span>
                <span className="text-white font-mono">{selectedBlock.hash}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Timestamp</span>
                <span className="text-white">{selectedBlock.timestamp}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Transactions</span>
                <span className="text-white">{selectedBlock.txs}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Miner</span>
                <span className="text-white font-mono">{selectedBlock.miner}</span>
              </div>
            </div>
            <button className="mt-4 w-full bg-violet-600 hover:bg-violet-700 text-white py-2 rounded-lg flex items-center justify-center">
              <ExternalLink size={16} className="mr-2" />
              View on Block Explorer
            </button>
          </div>
        </div>
      )}
    </div>
  )
}

export default ProvenanceExplorer
