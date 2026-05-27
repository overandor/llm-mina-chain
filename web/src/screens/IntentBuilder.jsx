import React, { useState } from 'react'
import { Send, Check, AlertTriangle } from 'lucide-react'

export default function IntentBuilder() {
  const [intent, setIntent] = useState('')
  const [parsed, setParsed] = useState(null)

  const handleParse = () => {
    if (!intent.trim()) return
    
    // Simple parser for demo
    const words = intent.toLowerCase().split(' ')
    const amount = words.find(w => !isNaN(w)) || '0'
    const receiver = words.find((w, i) => words[i-1] === 'to') || 'unknown'
    const gasless = intent.toLowerCase().includes('gasless')
    
    setParsed({
      sender: '0x' + Math.random().toString(16).substr(2, 40),
      receiver: receiver,
      amount: amount,
      gasMode: gasless ? 'gasless' : 'receiver-paid',
      gasPayer: gasless ? 'none' : receiver,
      nonce: Math.floor(Math.random() * 1000000),
      confidence: 0.92,
      warnings: gasless ? [] : ['Receiver gas policy requires verification']
    })
  }

  return (
    <div className="space-y-6">
      <div className="glass-card p-6">
        <h2 className="text-2xl font-bold text-white mb-2">Intent Transaction Builder</h2>
        <p className="text-gray-400">Type natural language to create structured transactions</p>
      </div>

      <div className="glass-card p-6">
        <label className="block text-sm font-medium text-gray-300 mb-2">
          Natural Language Intent
        </label>
        <textarea
          value={intent}
          onChange={(e) => setIntent(e.target.value)}
          placeholder="Send 25 credits to Alice, gasless"
          className="w-full h-32 bg-graphite-900 border border-graphite-700 rounded-lg p-4 text-white placeholder-gray-500 focus:outline-none focus:border-violet-500"
        />
        <button
          onClick={handleParse}
          className="mt-4 flex items-center space-x-2 bg-violet-600 hover:bg-violet-700 text-white px-6 py-2 rounded-lg transition-colors"
        >
          <Send size={18} />
          <span>Parse Intent</span>
        </button>
      </div>

      {parsed && (
        <div className="glass-card p-6">
          <h3 className="text-lg font-semibold text-white mb-4">Parsed Transaction</h3>
          <div className="space-y-3 font-mono text-sm">
            <div className="flex justify-between">
              <span className="text-gray-400">Sender:</span>
              <span className="text-violet-400">{parsed.sender}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Receiver:</span>
              <span className="text-violet-400">{parsed.receiver}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Amount:</span>
              <span className="text-white">{parsed.amount} credits</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Gas Mode:</span>
              <span className={parsed.gasMode === 'gasless' ? 'status-verified' : 'status-warning'}>
                {parsed.gasMode}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Gas Payer:</span>
              <span className="text-white">{parsed.gasPayer}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Nonce:</span>
              <span className="text-white">{parsed.nonce}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Confidence:</span>
              <span className="status-verified">{(parsed.confidence * 100).toFixed(0)}%</span>
            </div>
          </div>

          {parsed.warnings.length > 0 && (
            <div className="mt-4 p-3 bg-orange-500/10 border border-orange-500/30 rounded-lg">
              <div className="flex items-center space-x-2 text-orange-400 text-sm">
                <AlertTriangle size={16} />
                <span>Warnings:</span>
              </div>
              <ul className="mt-2 space-y-1 text-sm text-orange-300">
                {parsed.warnings.map((w, i) => (
                  <li key={i}>• {w}</li>
                ))}
              </ul>
            </div>
          )}

          <div className="mt-4 p-3 bg-graphite-900 rounded-lg">
            <div className="text-xs text-gray-500 mb-1">Transaction Hash</div>
            <div className="font-mono text-sm text-violet-400">
              0x{Math.random().toString(16).substr(2, 64)}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
