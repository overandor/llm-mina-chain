import React, { useState } from 'react'
import { 
  Home, 
  MessageSquare, 
  Activity, 
  Smartphone, 
  Search, 
  Settings, 
  Code,
  Menu,
  X
} from 'lucide-react'
import HomeScreen from './screens/HomeScreen'
import IntentBuilder from './screens/IntentBuilder'
import AtomicSettlement from './screens/AtomicSettlement'
import BrowserNode from './screens/BrowserNode'
import ProvenanceExplorer from './screens/ProvenanceExplorer'
import BuildConsole from './screens/BuildConsole'
import DevProvenance from './screens/DevProvenance'

const screens = [
  { id: 'home', name: 'Home', icon: Home },
  { id: 'intent', name: 'Intent Builder', icon: MessageSquare },
  { id: 'settlement', name: 'Atomic Settlement', icon: Activity },
  { id: 'node', name: 'Browser Node', icon: Smartphone },
  { id: 'explorer', name: 'Explorer', icon: Search },
  { id: 'build', name: 'Build Status', icon: Settings },
  { id: 'dev', name: 'Dev Provenance', icon: Code },
]

function App() {
  const [currentScreen, setCurrentScreen] = useState('home')
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)

  const ScreenComponent = screens.find(s => s.id === currentScreen)?.component || HomeScreen

  return (
    <div className="min-h-screen bg-graphite-950">
      {/* Navigation */}
      <nav className="fixed top-0 left-0 right-0 z-50 bg-graphite-900/80 backdrop-blur-md border-b border-graphite-700">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center">
              <h1 className="text-xl font-bold text-violet-400">MEMBRA Instant Proof Chain</h1>
            </div>
            
            {/* Desktop Navigation */}
            <div className="hidden md:flex items-center space-x-1">
              {screens.map((screen) => {
                const Icon = screen.icon
                return (
                  <button
                    key={screen.id}
                    onClick={() => setCurrentScreen(screen.id)}
                    className={`flex items-center space-x-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                      currentScreen === screen.id
                        ? 'bg-violet-600/20 text-violet-400'
                        : 'text-gray-400 hover:text-gray-200 hover:bg-graphite-800'
                    }`}
                  >
                    <Icon size={16} />
                    <span>{screen.name}</span>
                  </button>
                )
              })}
            </div>

            {/* Mobile menu button */}
            <button
              onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
              className="md:hidden p-2 rounded-lg text-gray-400 hover:text-gray-200 hover:bg-graphite-800"
            >
              {mobileMenuOpen ? <X size={24} /> : <Menu size={24} />}
            </button>
          </div>
        </div>

        {/* Mobile Navigation */}
        {mobileMenuOpen && (
          <div className="md:hidden border-t border-graphite-700">
            <div className="px-2 pt-2 pb-3 space-y-1">
              {screens.map((screen) => {
                const Icon = screen.icon
                return (
                  <button
                    key={screen.id}
                    onClick={() => {
                      setCurrentScreen(screen.id)
                      setMobileMenuOpen(false)
                    }}
                    className={`flex items-center space-x-3 w-full px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                      currentScreen === screen.id
                        ? 'bg-violet-600/20 text-violet-400'
                        : 'text-gray-400 hover:text-gray-200 hover:bg-graphite-800'
                    }`}
                  >
                    <Icon size={18} />
                    <span>{screen.name}</span>
                  </button>
                )
              })}
            </div>
          </div>
        )}
      </nav>

      {/* Main Content */}
      <main className="pt-20 pb-8 px-4 sm:px-6 lg:px-8 max-w-7xl mx-auto">
        <ScreenComponent />
      </main>

      {/* Footer */}
      <footer className="fixed bottom-0 left-0 right-0 bg-graphite-900/80 backdrop-blur-md border-t border-graphite-700 py-3 px-4">
        <div className="max-w-7xl mx-auto flex items-center justify-between text-xs text-gray-500">
          <span>Experimental prototype • Not for real funds</span>
          <span>Release build: passing • Tests: 15/18</span>
        </div>
      </footer>
    </div>
  )
}

export default App
