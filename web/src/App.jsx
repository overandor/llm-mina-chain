import React, { useEffect, useState } from 'react'
import {
  ArrowRight,
  BadgeInfo,
  Database,
  Globe,
  HeartPulse,
  History,
  Search,
  Server,
  Wallet,
} from 'lucide-react'

const PANELS = [
  {
    id: 'query',
    title: 'Query',
    blurb: 'Natural language search',
    icon: Search,
  },
  {
    id: 'wallet',
    title: 'Wallet',
    blurb: 'Balance and token lookups',
    icon: Wallet,
  },
  {
    id: 'transaction',
    title: 'Transaction',
    blurb: 'Inspect signatures',
    icon: History,
  },
  {
    id: 'network',
    title: 'Network',
    blurb: 'Slot, health, version',
    icon: Globe,
  },
]

const QUICK_ACTIONS = [
  'What is the latest slot on Solana?',
  'Check Solana RPC health',
  'Show wallet balance for So11111111111111111111111111111111111111112',
  'Show account info for So11111111111111111111111111111111111111112',
]

const DEFAULT_PROMPT = 'What is the latest slot on Solana?'
const DEFAULT_ENDPOINT = 'http://127.0.0.1:8000'

function normalizeEndpoint(value) {
  return value.trim().replace(/\/+$/, '')
}

function extractAddress(text) {
  const match = text.match(/[1-9A-HJ-NP-Za-km-z]{32,44}/)
  return match?.[0] ?? ''
}

function extractSignature(text) {
  const match = text.match(/[1-9A-HJ-NP-Za-km-z]{60,120}/)
  return match?.[0] ?? ''
}

function summarizeResponse(payload) {
  if (payload?.slot) return `Current slot is ${payload.slot}.`
  if (payload?.pubkey && typeof payload?.lamports === 'number') {
    return `${payload.pubkey} holds ${payload.sol?.toFixed?.(6) ?? payload.sol} SOL.`
  }
  if (payload?.status) return `Network status: ${payload.status}.`
  if (payload?.columns && payload?.rows) {
    return `Returned ${payload.row_count ?? payload.rows.length} row${payload.rows.length === 1 ? '' : 's'} from ${payload.query_type ?? 'query'}.`
  }
  if (payload?.service) return `${payload.service} is online.`
  if (payload?.answer) return payload.answer
  return 'Request completed successfully.'
}

async function requestJson(url, options = {}) {
  const response = await fetch(url, options)
  const data = await response.json().catch(() => ({}))
  if (!response.ok) {
    throw new Error(data.error || `Request failed with status ${response.status}`)
  }
  return data
}

async function runPrompt(endpoint, prompt) {
  const lowered = prompt.toLowerCase().trim()
  const address = extractAddress(prompt)
  const signature = extractSignature(prompt)

  if (lowered.startsWith('select ')) {
    return requestJson(`${endpoint}/query`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query: prompt }),
    })
  }

  if (lowered.includes('health')) {
    return requestJson(`${endpoint}/health`)
  }

  if (lowered.includes('latest slot') || lowered === 'slot' || lowered.includes('current slot')) {
    return requestJson(`${endpoint}/slot`)
  }

  if ((lowered.includes('balance') || lowered.includes('wallet')) && address) {
    return requestJson(`${endpoint}/balance`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pubkey: address }),
    })
  }

  if ((lowered.includes('transaction') || lowered.includes('signature') || lowered.startsWith('tx')) && signature) {
    return requestJson(`${endpoint}/transaction`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ signature }),
    })
  }

  if ((lowered.includes('account') || lowered.includes('wallet')) && address) {
    return requestJson(`${endpoint}/account`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pubkey: address }),
    })
  }

  return requestJson(`${endpoint}/ask`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ question: prompt }),
  })
}

function StatCard({ title, value, detail, icon: Icon, tone = 'mint' }) {
  return (
    <section className={`neo-card neo-stat neo-tone-${tone}`}>
      <div className="neo-stat-head">
        <span className="neo-icon-shell">
          <Icon size={18} />
        </span>
        <span>{title}</span>
      </div>
      <strong>{value}</strong>
      <p>{detail}</p>
    </section>
  )
}

function App() {
  const [activePanel, setActivePanel] = useState('query')
  const [endpoint, setEndpoint] = useState(
    normalizeEndpoint(localStorage.getItem('solana-agent-endpoint') || DEFAULT_ENDPOINT),
  )
  const [wallet, setWallet] = useState('So11111111111111111111111111111111111111112')
  const [prompt, setPrompt] = useState(DEFAULT_PROMPT)
  const [result, setResult] = useState(null)
  const [summary, setSummary] = useState('Use a plain-English prompt or SQL-style query.')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [metrics, setMetrics] = useState({
    health: 'Checking',
    slot: '...',
    version: 'unknown',
    balance: '--',
  })
  const [recentQueries, setRecentQueries] = useState([])

  useEffect(() => {
    localStorage.setItem('solana-agent-endpoint', endpoint)
  }, [endpoint])

  useEffect(() => {
    let cancelled = false

    async function loadOverview() {
      try {
        const base = normalizeEndpoint(endpoint)
        const [health, slot, version] = await Promise.all([
          requestJson(`${base}/health`),
          requestJson(`${base}/slot`),
          requestJson(`${base}/version`),
        ])

        let balance = '--'
        if (wallet) {
          const balanceResponse = await requestJson(`${base}/balance`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ pubkey: wallet }),
          })
          balance = `${Number(balanceResponse.sol).toFixed(4)} SOL`
        }

        if (!cancelled) {
          setMetrics({
            health: health.status || 'ok',
            slot: String(slot.slot ?? 'n/a'),
            version: version['solana-core'] || version.version || 'unknown',
            balance,
          })
        }
      } catch (overviewError) {
        if (!cancelled) {
          setMetrics((previous) => ({
            ...previous,
            health: 'offline',
          }))
        }
      }
    }

    loadOverview()
    return () => {
      cancelled = true
    }
  }, [endpoint, wallet])

  async function handleSubmit(nextPrompt = prompt) {
    const cleanEndpoint = normalizeEndpoint(endpoint)
    const cleanPrompt = nextPrompt.trim()

    if (!cleanEndpoint || !cleanPrompt) return

    setLoading(true)
    setError('')

    try {
      const payload = await runPrompt(cleanEndpoint, cleanPrompt)
      setResult(payload)
      setSummary(summarizeResponse(payload))
      setRecentQueries((previous) => [
        { prompt: cleanPrompt, timestamp: new Date().toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }) },
        ...previous,
      ].slice(0, 4))
    } catch (submitError) {
      setError(submitError.message)
      setResult(null)
      setSummary('The agent did not return a usable response.')
    } finally {
      setLoading(false)
    }
  }

  const activeInfo = PANELS.find((panel) => panel.id === activePanel) || PANELS[0]

  return (
    <div className="neo-app-shell">
      <div className="neo-page-orb neo-page-orb-left" />
      <div className="neo-page-orb neo-page-orb-right" />

      <div className="neo-layout">
        <aside className="neo-sidebar">
          <div className="neo-brand">
            <div className="neo-brand-mark">
              <span />
              <span />
              <span />
            </div>
            <div>
              <h1>Solana Query</h1>
              <p>Agent Console</p>
            </div>
          </div>

          <nav className="neo-nav">
            {PANELS.map((panel) => {
              const Icon = panel.icon
              const isActive = panel.id === activePanel

              return (
                <button
                  key={panel.id}
                  type="button"
                  onClick={() => setActivePanel(panel.id)}
                  className={`neo-nav-item ${isActive ? 'is-active' : ''}`}
                >
                  <span className="neo-icon-shell">
                    <Icon size={18} />
                  </span>
                  <span>
                    <strong>{panel.title}</strong>
                    <small>{panel.blurb}</small>
                  </span>
                </button>
              )
            })}
          </nav>

          <section className="neo-card neo-sidebar-card">
            <div className="neo-sidebar-tip">
              <BadgeInfo size={18} />
              <span>Quick tip</span>
            </div>
            <p>Ask direct questions like “show wallet balance for &lt;address&gt;” or paste SQL into the query box.</p>
          </section>
        </aside>

        <main className="neo-main">
          <header className="neo-topbar">
            <div className="neo-status-pill">
              <span className={`neo-status-dot ${metrics.health === 'ok' ? 'is-ok' : 'is-warn'}`} />
              <span>{metrics.health === 'ok' ? 'Agent healthy' : 'Endpoint unavailable'}</span>
            </div>

            <label className="neo-inline-field">
              <span>Endpoint</span>
              <input
                value={endpoint}
                onChange={(event) => setEndpoint(event.target.value)}
                placeholder="http://127.0.0.1:8000"
              />
            </label>
          </header>

          <section className="neo-hero">
            <div className="neo-hero-copy">
              <p className="neo-overline">{activeInfo.title}</p>
              <h2>Query Solana in simple words.</h2>
              <p>
                Ask about accounts, balances, signatures, slots, or paste a SQL-style
                query that targets the live Solana agent.
              </p>
            </div>

            <div className="neo-card neo-hero-panel">
              <div className="neo-input-stack">
                <label className="neo-field">
                  <span>Tracked wallet</span>
                  <input
                    value={wallet}
                    onChange={(event) => setWallet(event.target.value)}
                    placeholder="Paste a Solana public key"
                  />
                </label>

                <label className="neo-field">
                  <span>Prompt or query</span>
                  <textarea
                    value={prompt}
                    onChange={(event) => setPrompt(event.target.value)}
                    placeholder="What is the latest slot on Solana?"
                    rows={5}
                  />
                </label>

                <div className="neo-chip-row">
                  {QUICK_ACTIONS.map((action) => (
                    <button
                      key={action}
                      type="button"
                      className="neo-chip"
                      onClick={() => {
                        setPrompt(action)
                        handleSubmit(action)
                      }}
                    >
                      {action}
                    </button>
                  ))}
                </div>

                <div className="neo-action-row">
                  <button type="button" className="neo-button neo-button-primary" onClick={() => handleSubmit()}>
                    <span>{loading ? 'Running query' : 'Run query'}</span>
                    <ArrowRight size={18} />
                  </button>
                </div>
              </div>
            </div>
          </section>

          <section className="neo-dashboard-grid">
            <div className="neo-results-column">
              <section className="neo-card neo-results-card">
                <div className="neo-section-head">
                  <div>
                    <p className="neo-overline">Result</p>
                    <h3>{summary}</h3>
                  </div>
                  <span className="neo-badge">{loading ? 'loading' : 'live'}</span>
                </div>

                {error ? <div className="neo-error">{error}</div> : null}

                <div className="neo-results-body">
                  <div className="neo-result-preview">
                    <h4>Structured output</h4>
                    <pre>{JSON.stringify(result, null, 2)}</pre>
                  </div>

                  <div className="neo-card neo-query-help">
                    <h4>Ready prompts</h4>
                    <ul>
                      <li>What is the latest slot on Solana?</li>
                      <li>Check Solana RPC health</li>
                      <li>Show wallet balance for a public key</li>
                      <li>SELECT * FROM status</li>
                    </ul>
                  </div>
                </div>
              </section>
            </div>

            <aside className="neo-side-stats">
              <StatCard title="Network health" value={metrics.health} detail="RPC availability" icon={HeartPulse} tone="mint" />
              <StatCard title="Current slot" value={metrics.slot} detail="Confirmed commitment" icon={Server} tone="sky" />
              <StatCard title="Agent version" value={metrics.version} detail="Solana core" icon={Database} tone="sand" />
              <StatCard title="Tracked balance" value={metrics.balance} detail="Wallet snapshot" icon={Wallet} tone="mint" />

              <section className="neo-card neo-recent-card">
                <div className="neo-section-head">
                  <div>
                    <p className="neo-overline">Recent</p>
                    <h3>Recent prompts</h3>
                  </div>
                </div>
                <div className="neo-recent-list">
                  {recentQueries.length === 0 ? (
                    <p className="neo-empty">Your recent requests will appear here.</p>
                  ) : (
                    recentQueries.map((item) => (
                      <div className="neo-recent-item" key={`${item.prompt}-${item.timestamp}`}>
                        <strong>{item.prompt}</strong>
                        <span>{item.timestamp}</span>
                      </div>
                    ))
                  )}
                </div>
              </section>
            </aside>
          </section>
        </main>
      </div>
    </div>
  )
}

export default App
