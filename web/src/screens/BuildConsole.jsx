import React from 'react'
import { CheckCircle, XCircle, Clock, Package, Cpu } from 'lucide-react'

function BuildConsole() {
  const buildStatus = {
    rust: { status: 'passing', time: '1m 0s', warnings: 4 },
    cpp: { status: 'passing', time: '2.3s', warnings: 3 },
    web: { status: 'building', time: 'pending', warnings: 0 },
    tests: { status: 'partial', time: '5.2s', passed: 15, total: 18 }
  }

  const getStatusIcon = (status) => {
    switch (status) {
      case 'passing':
        return <CheckCircle className="text-green-400" size={20} />
      case 'partial':
        return <Clock className="text-yellow-400" size={20} />
      case 'building':
        return <Clock className="text-blue-400 animate-spin" size={20} />
      default:
        return <XCircle className="text-red-400" size={20} />
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white mb-2">Build Console</h2>
        <p className="text-gray-400">Real-time build status and compilation output</p>
      </div>

      {/* Build Status Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Rust Build */}
        <div className="bg-graphite-800 rounded-lg p-4 border border-graphite-700">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center space-x-2">
              <Package className="text-orange-400" size={20} />
              <span className="text-white font-medium">Rust Implementation</span>
            </div>
            {getStatusIcon(buildStatus.rust.status)}
          </div>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-400">Status</span>
              <span className="text-green-400">Passing</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Build Time</span>
              <span className="text-white">{buildStatus.rust.time}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Warnings</span>
              <span className="text-yellow-400">{buildStatus.rust.warnings}</span>
            </div>
          </div>
        </div>

        {/* C++ Build */}
        <div className="bg-graphite-800 rounded-lg p-4 border border-graphite-700">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center space-x-2">
              <Cpu className="text-blue-400" size={20} />
              <span className="text-white font-medium">C++ Implementation</span>
            </div>
            {getStatusIcon(buildStatus.cpp.status)}
          </div>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-400">Status</span>
              <span className="text-green-400">Passing</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Build Time</span>
              <span className="text-white">{buildStatus.cpp.time}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Warnings</span>
              <span className="text-yellow-400">{buildStatus.cpp.warnings}</span>
            </div>
          </div>
        </div>

        {/* Web Build */}
        <div className="bg-graphite-800 rounded-lg p-4 border border-graphite-700">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center space-x-2">
              <Package className="text-violet-400" size={20} />
              <span className="text-white font-medium">Web Interface</span>
            </div>
            {getStatusIcon(buildStatus.web.status)}
          </div>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-400">Status</span>
              <span className="text-blue-400">Building...</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Build Time</span>
              <span className="text-white">{buildStatus.web.time}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Warnings</span>
              <span className="text-white">{buildStatus.web.warnings}</span>
            </div>
          </div>
        </div>

        {/* Tests */}
        <div className="bg-graphite-800 rounded-lg p-4 border border-graphite-700">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center space-x-2">
              <CheckCircle className="text-green-400" size={20} />
              <span className="text-white font-medium">Test Suite</span>
            </div>
            {getStatusIcon(buildStatus.tests.status)}
          </div>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-400">Status</span>
              <span className="text-yellow-400">Partial (15/18)</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Test Time</span>
              <span className="text-white">{buildStatus.tests.time}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Passed</span>
              <span className="text-green-400">{buildStatus.tests.passed}/{buildStatus.tests.total}</span>
            </div>
          </div>
        </div>
      </div>

      {/* Build Log */}
      <div className="bg-graphite-800 rounded-lg p-4 border border-graphite-700">
        <h3 className="text-white font-medium mb-3">Latest Build Output</h3>
        <div className="bg-graphite-900 rounded p-3 font-mono text-xs text-gray-300 space-y-1 max-h-64 overflow-y-auto">
          <div className="text-green-400">✓ cargo build --release</div>
          <div className="text-gray-400">   Compiling llm-mina-chain v0.1.0</div>
          <div className="text-yellow-400">   warning: unused imports</div>
          <div className="text-green-400">   Finished release profile</div>
          <div className="text-green-400">✓ cmake .. && make</div>
          <div className="text-gray-400">   Building CXX object</div>
          <div className="text-yellow-400">   warning: deprecated OpenSSL functions</div>
          <div className="text-green-400">   Built target llm-mina-node</div>
          <div className="text-blue-400">→ npm run build</div>
          <div className="text-gray-400">   vite build...</div>
        </div>
      </div>
    </div>
  )
}

export default BuildConsole
