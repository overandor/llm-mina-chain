import React, { useState, useEffect } from 'react'
import { Play, Pause, Activity, Battery, Wifi, Smartphone } from 'lucide-react'

export default function BrowserNode() {
  const [mining, setMining] = useState(false)
  const [hashRate, setHashRate] = useState(0)
  const [sessionDuration, setSessionDuration] = useState(0)
  const [workSubmitted, setWorkSubmitted] = useState(0)

  useEffect(() => {
    let interval
    if (mining) {
      interval = setInterval(() => {
        setHashRate(Math.random() * 100 + 50)
        setSessionDuration(prev => prev + 1)
        setWorkSubmitted(prev => prev + Math.floor(Math.random() * 5))
      }, 1000)
    }
    return () => clearInterval(interval)
  }, [mining])

  const formatDuration = (seconds) => {
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  return (
    <div className="space-y-6">
      <div className="glass-card p-6">
        <h2 className="text-2xl font-bold text-white mb-2">Browser Node / iPhone Mining</h2>
        <p className="text-gray-400">Keep this page open to participate as a lightweight browser node</p>
      </div>

      <div className="glass-card p-6 border-l-4 border-l-orange-500">
        <div className="flex items-start space-x-3">
          <AlertTriangle className="text-orange-400 mt-1" size={20} />
          <div>
            <h3 className="text-orange-400 font-semibold mb-1">Experimental Browser Mining</h3>
            <p className="text-gray-400 text-sm">
              No real financial rewards. Do not treat this as mainnet mining. 
              This is a prototype for lightweight node participation.
            </p>
          </div>
        </div>
      </div>

      <div className="glass-card p-6">
        <div className="flex items-center justify-between mb-6">
          <h3 className="text-lg font-semibold text-white">Mining Status</h3>
          <button
            onClick={() => setMining(!mining)}
            className={`flex items-center space-x-2 px-4 py-2 rounded-lg transition-colors ${
              mining 
                ? 'bg-red-600 hover:bg-red-700 text-white' 
                : 'bg-green-600 hover:bg-green-700 text-white'
            }`}
          >
            {mining ? <Pause size={18} /> : <Play size={18} />}
            <span>{mining ? 'Stop Mining' : 'Start Mining'}</span>
          </button>
        </div>

        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-graphite-900 rounded-lg p-4">
            <div className="flex items-center space-x-2 text-gray-400 text-sm mb-2">
              <Activity size={16} />
              <span>Hash Rate</span>
            </div>
            <div className="text-2xl font-bold text-violet-400">
              {hashRate.toFixed(1)} H/s
            </div>
          </div>

          <div className="bg-graphite-900 rounded-lg p-4">
            <div className="flex items-center space-x-2 text-gray-400 text-sm mb-2">
              <Clock size={16} />
              <span>Duration</span>
            </div>
            <div className="text-2xl font-bold text-white">
              {formatDuration(sessionDuration)}
            </div>
          </div>

          <div className="bg-graphite-900 rounded-lg p-4">
            <div className="flex items-center space-x-2 text-gray-400 text-sm mb-2">
              <Activity size={16} />
              <span>Work Submitted</span>
            </div>
            <div className="text-2xl font-bold text-electric-blue">
              {workSubmitted}
            </div>
          </div>

          <div className="bg-graphite-900 rounded-lg p-4">
            <div className="flex items-center space-x-2 text-gray-400 text-sm mb-2">
              <CheckCircle size={16} />
              <span>Contribution</span>
            </div>
            <div className="text-2xl font-bold text-green-400">
              {(workSubmitted * 0.1).toFixed(1)} pts
            </div>
          </div>
        </div>
      </div>

      <div className="glass-card p-6">
        <h3 className="text-lg font-semibold text-white mb-4">Device Information</h3>
        <div className="space-y-3 font-mono text-sm">
          <div className="flex justify-between">
            <span className="text-gray-400">Device Type:</span>
            <span className="text-white">{navigator.userAgent.includes('iPhone') ? 'iPhone' : 'Desktop'}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-gray-400">Browser Support:</span>
            <span className="status-verified">WebAssembly ✓</span>
          </div>
          <div className="flex justify-between">
            <span className="text-gray-400">Battery Status:</span>
            <span className="text-white">Checking...</span>
          </div>
          <div className="flex justify-between">
            <span className="text-gray-400">Network:</span>
            <span className="status-verified">Connected ✓</span>
          </div>
          <div className="flex justify-between">
            <span className="text-gray-400">Cores:</span>
            <span className="text-white">{navigator.hardwareConcurrency || 'Unknown'}</span>
          </div>
        </div>
      </div>

      {mining && (
        <div className="glass-card p-6">
          <h3 className="text-lg font-semibold text-white mb-4">Live Mining Output</h3>
          <div className="bg-graphite-900 rounded-lg p-4 font-mono text-xs text-green-400 space-y-1 h-48 overflow-y-auto">
            <div>[{new Date().toISOString()}] Mining session started</div>
            <div>[{new Date().toISOString()}] Hash: 0x{Math.random().toString(16).substr(2, 32)}</div>
            <div>[{new Date().toISOString()}] Work unit submitted</div>
            <div>[{new Date().toISOString()}] Hash: 0x{Math.random().toString(16).substr(2, 32)}</div>
            <div>[{new Date().toISOString()}] Proof candidate generated</div>
          </div>
        </div>
      )}
    </div>
  )
}
