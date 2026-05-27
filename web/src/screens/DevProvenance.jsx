import React from 'react'
import { GitBranch, Code, FileText, Terminal, Shield } from 'lucide-react'

function DevProvenance() {
  const commits = [
    { hash: 'a1b2c3d', message: 'Add Rust blockchain core implementation', author: 'dev', time: '2h ago' },
    { hash: 'e4f5g6h', message: 'Implement C++ core with OpenSSL', author: 'dev', time: '4h ago' },
    { hash: 'i7j8k9l', message: 'Add web interface scaffolding', author: 'dev', time: '6h ago' },
  ]

  const files = [
    { name: 'rust/src/lib.rs', lines: 452, status: 'complete' },
    { name: 'cpp/src/core.cpp', lines: 295, status: 'complete' },
    { name: 'web/src/App.jsx', lines: 124, status: 'complete' },
    { name: 'rust/src/crypto.rs', lines: 200, status: 'partial' },
  ]

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white mb-2">Development Provenance</h2>
        <p className="text-gray-400">Track code changes, file status, and development history</p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="bg-graphite-800 rounded-lg p-4 border border-graphite-700">
          <div className="flex items-center space-x-2 mb-2">
            <GitBranch className="text-violet-400" size={20} />
            <span className="text-gray-400 text-sm">Commits</span>
          </div>
          <div className="text-2xl font-bold text-white">127</div>
        </div>
        <div className="bg-graphite-800 rounded-lg p-4 border border-graphite-700">
          <div className="flex items-center space-x-2 mb-2">
            <Code className="text-green-400" size={20} />
            <span className="text-gray-400 text-sm">Files</span>
          </div>
          <div className="text-2xl font-bold text-white">42</div>
        </div>
        <div className="bg-graphite-800 rounded-lg p-4 border border-graphite-700">
          <div className="flex items-center space-x-2 mb-2">
            <FileText className="text-blue-400" size={20} />
            <span className="text-gray-400 text-sm">Lines of Code</span>
          </div>
          <div className="text-2xl font-bold text-white">8.5K</div>
        </div>
        <div className="bg-graphite-800 rounded-lg p-4 border border-graphite-700">
          <div className="flex items-center space-x-2 mb-2">
            <Shield className="text-yellow-400" size={20} />
            <span className="text-gray-400 text-sm">Test Coverage</span>
          </div>
          <div className="text-2xl font-bold text-white">83%</div>
        </div>
      </div>

      {/* Recent Commits */}
      <div className="bg-graphite-800 rounded-lg p-4 border border-graphite-700">
        <h3 className="text-white font-medium mb-3 flex items-center">
          <GitBranch size={18} className="mr-2 text-violet-400" />
          Recent Commits
        </h3>
        <div className="space-y-3">
          {commits.map((commit) => (
            <div key={commit.hash} className="flex items-start space-x-3 p-3 bg-graphite-900 rounded">
              <div className="font-mono text-violet-400 text-sm">{commit.hash}</div>
              <div className="flex-1">
                <div className="text-white text-sm">{commit.message}</div>
                <div className="text-gray-500 text-xs mt-1">
                  {commit.author} • {commit.time}
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* File Status */}
      <div className="bg-graphite-800 rounded-lg p-4 border border-graphite-700">
        <h3 className="text-white font-medium mb-3 flex items-center">
          <Code size={18} className="mr-2 text-green-400" />
          File Status
        </h3>
        <div className="space-y-2">
          {files.map((file) => (
            <div key={file.name} className="flex items-center justify-between p-3 bg-graphite-900 rounded">
              <div className="flex items-center space-x-3">
                <Terminal size={16} className="text-gray-400" />
                <div>
                  <div className="text-white text-sm font-mono">{file.name}</div>
                  <div className="text-gray-500 text-xs">{file.lines} lines</div>
                </div>
              </div>
              <span className={`text-xs px-2 py-1 rounded ${
                file.status === 'complete' 
                  ? 'bg-green-600/20 text-green-400' 
                  : 'bg-yellow-600/20 text-yellow-400'
              }`}>
                {file.status}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}

export default DevProvenance
