import React, { useState } from 'react'
import { CheckCircle, Clock, XCircle, AlertTriangle } from 'lucide-react'

const stages = [
  { id: 1, name: 'Intent Received', status: 'verified' },
  { id: 2, name: 'Transaction Parsed', status: 'verified' },
  { id: 3, name: 'Transaction Signed', status: 'verified' },
  { id: 4, name: 'Sender Work Computed', status: 'verified' },
  { id: 5, name: 'Receiver Gas Policy Checked', status: 'verified' },
  { id: 6, name: 'State Updated Atomically', status: 'verified' },
  { id: 7, name: 'Block Candidate Produced', status: 'verified' },
  { id: 8, name: 'Storage Written', status: 'verified' },
  { id: 9, name: 'Metrics Emitted', status: 'verified' },
  { id: 10, name: 'Proof Receipt Generated', status: 'verified' },
]

export default function AtomicSettlement() {
  const [selectedStage, setSelectedStage] = useState(null)

  const getStatusIcon = (status) => {
    switch (status) {
      case 'verified': return <CheckCircle className="text-green-400" size={20} />
      case 'pending': return <Clock className="text-yellow-400" size={20} />
      case 'error': return <XCircle className="text-red-400" size={20} />
      case 'warning': return <AlertTriangle className="text-orange-400" size={20} />
      default: return <Clock className="text-gray-400" size={20} />
    }
  }

  const getStatusColor = (status) => {
    switch (status) {
      case 'verified': return 'border-green-500/30 bg-green-500/10'
      case 'pending': return 'border-yellow-500/30 bg-yellow-500/10'
      case 'error': return 'border-red-500/30 bg-red-500/10'
      case 'warning': return 'border-orange-500/30 bg-orange-500/10'
      default: return 'border-gray-500/30 bg-gray-500/10'
    }
  }

  return (
    <div className="space-y-6">
      <div className="glass-card p-6">
        <h2 className="text-2xl font-bold text-white mb-2">Atomic Settlement View</h2>
        <p className="text-gray-400">Transaction lifecycle proof timeline</p>
      </div>

      <div className="glass-card p-6">
        <div className="space-y-4">
          {stages.map((stage, index) => (
            <div
              key={stage.id}
              onClick={() => setSelectedStage(stage)}
              className={`p-4 rounded-lg border cursor-pointer transition-all ${getStatusColor(stage.status)} ${
                selectedStage?.id === stage.id ? 'ring-2 ring-violet-500' : ''
              }`}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-3">
                  <div className="flex items-center justify-center w-8 h-8 rounded-full bg-graphite-900 text-xs font-mono">
                    {index + 1}
                  </div>
                  <span className="font-medium text-white">{stage.name}</span>
                </div>
                {getStatusIcon(stage.status)}
              </div>
            </div>
          ))}
        </div>
      </div>

      {selectedStage && (
        <div className="glass-card p-6">
          <h3 className="text-lg font-semibold text-white mb-4">
            Stage Details: {selectedStage.name}
          </h3>
          <div className="space-y-3 font-mono text-sm">
            <div className="flex justify-between">
              <span className="text-gray-400">Status:</span>
              <span className={`status-${selectedStage.status}`}>{selectedStage.status}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Timestamp:</span>
              <span className="text-white">{new Date().toISOString()}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Hash:</span>
              <span className="text-violet-400">0x{Math.random().toString(16).substr(2, 32)}</span>
            </div>
          </div>
          
          {selectedStage.id === 3 && (
            <div className="mt-4 p-3 bg-yellow-500/10 border border-yellow-500/30 rounded-lg">
              <div className="text-yellow-400 text-sm">
                <strong>Note:</strong> Signature mode: placeholder prototype
              </div>
            </div>
          )}
          
          {selectedStage.id === 6 && (
            <div className="mt-4 p-3 bg-gray-500/10 border border-gray-500/30 rounded-lg">
              <div className="text-gray-400 text-sm">
                <strong>Note:</strong> Consensus mode: disabled in current build
              </div>
            </div>
          )}
        </div>
      )}

      <div className="glass-card p-6">
        <h3 className="text-lg font-semibold text-white mb-4">Proof Receipt</h3>
        <div className="bg-graphite-900 rounded-lg p-4 font-mono text-xs text-gray-300 space-y-2">
          <div>Transaction Hash: 0x{Math.random().toString(16).substr(2, 64)}</div>
          <div>Block Hash: 0x{Math.random().toString(16).substr(2, 64)}</div>
          <div>State Root Before: 0x{Math.random().toString(16).substr(2, 64)}</div>
          <div>State Root After: 0x{Math.random().toString(16).substr(2, 64)}</div>
          <div>Gas Mode: gasless</div>
          <div>Build Version: v0.1.0-prototype</div>
          <div>Receipt Signature: status-placeholder</div>
        </div>
      </div>
    </div>
  )
}
