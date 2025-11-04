import { useState, useEffect } from 'react'
import ConnectionsPanel from './components/ConnectionsPanel'

function App() {
  const [settings, setSettings] = useState([])
  const [connections, setConnections] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [wsMessages, setWsMessages] = useState([])
  const [showNewForm, setShowNewForm] = useState(false)
  const [editingSetting, setEditingSetting] = useState(null)

  // WebSocket connection
  useEffect(() => {
    const ws = new WebSocket(`ws://${window.location.hostname}:8080/ws`)

    ws.onopen = () => {
      console.log('WebSocket connected')
    }

    ws.onmessage = (event) => {
      const message = event.data
      console.log('WS message:', message)
      setWsMessages((prev) => [message, ...prev].slice(0, 20))

      // Refresh settings on updates
      if (message.startsWith('settings_')) {
        fetchSettings()
      }
    }

    ws.onerror = (error) => {
      console.error('WebSocket error:', error)
    }

    return () => ws.close()
  }, [])

  // Fetch connections
  const fetchConnections = async () => {
    try {
      const response = await fetch('/api/connections')

      // Check if response is ok before trying to parse JSON
      if (!response.ok) {
        throw new Error(`Server returned ${response.status}: ${response.statusText}`)
      }

      const data = await response.json()
      if (data.success) {
        setConnections(data.data || [])
      }
    } catch (err) {
      // Distinguish between network errors and other errors
      if (err instanceof TypeError && err.message.includes('fetch')) {
        console.error('Cannot connect to server - is rust-server running?')
      } else {
        console.error('Failed to fetch connections:', err)
      }
    }
  }

  // Fetch settings
  const fetchSettings = async () => {
    try {
      setLoading(true)
      const response = await fetch('/api/settings')

      // Check if response is ok before trying to parse JSON
      if (!response.ok) {
        throw new Error(`Server returned ${response.status}: ${response.statusText}`)
      }

      const data = await response.json()

      if (data.success) {
        setSettings(data.data || [])
        setError(null)
      } else {
        setError(data.error || 'Failed to load settings')
      }
    } catch (err) {
      // Provide user-friendly error messages
      if (err instanceof TypeError && (err.message.includes('fetch') || err.message.includes('Failed to fetch'))) {
        setError('サーバーに接続できません。Rust Serverが起動しているか確認してください。')
      } else if (err.message.includes('JSON')) {
        setError('サーバーからの応答が不正です。Rust Serverが正しく起動していない可能性があります。')
      } else if (err.message.includes('500') || err.message.includes('502') || err.message.includes('503')) {
        setError('サーバーに接続できません。Rust Serverが起動しているか確認してください。（プロキシエラー）')
      } else {
        setError('サーバーとの通信中にエラーが発生しました: ' + err.message)
      }
      console.error('Failed to fetch settings:', err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchSettings()
    fetchConnections()

    // Refresh connections every 5 seconds
    const interval = setInterval(fetchConnections, 5000)
    return () => clearInterval(interval)
  }, [])

  // Toggle enabled status
  const toggleEnabled = async (id, currentStatus) => {
    try {
      const response = await fetch(`/api/settings/${id}/toggle`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: !currentStatus }),
      })
      const data = await response.json()

      if (data.success) {
        fetchSettings()
      } else {
        alert('Failed to toggle: ' + data.error)
      }
    } catch (err) {
      alert('Error: ' + err.message)
    }
  }

  // Create new setting
  const createSetting = async (formData) => {
    try {
      const response = await fetch('/api/settings', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(formData),
      })
      const data = await response.json()

      if (data.success) {
        setShowNewForm(false)
        fetchSettings()
      } else {
        alert('Failed to create: ' + data.error)
      }
    } catch (err) {
      alert('Error: ' + err.message)
    }
  }

  // Update setting
  const updateSetting = async (id, updatedData) => {
    try {
      const response = await fetch(`/api/settings/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updatedData),
      })
      const data = await response.json()

      if (data.success) {
        setEditingSetting(null)
        fetchSettings()
      } else {
        alert('Failed to update: ' + data.error)
      }
    } catch (err) {
      alert('Error: ' + err.message)
    }
  }

  // Delete setting
  const deleteSetting = async (id) => {
    if (!confirm('Delete this copy setting?')) return

    try {
      const response = await fetch(`/api/settings/${id}`, {
        method: 'DELETE',
      })
      const data = await response.json()

      if (data.success) {
        fetchSettings()
      } else {
        alert('Failed to delete: ' + data.error)
      }
    } catch (err) {
      alert('Error: ' + err.message)
    }
  }

  if (loading && settings.length === 0) {
    return (
      <div className="min-h-screen bg-gray-100 flex items-center justify-center">
        <div className="text-xl">Loading...</div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gray-100">
      <div className="max-w-6xl mx-auto p-4">
        {/* Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h1 className="text-3xl font-bold text-gray-800">Forex Copier</h1>
          <p className="text-gray-600 mt-2">Trade copying management dashboard</p>
        </div>

        {/* Error Display */}
        {error && (
          <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-6">
            {error}
          </div>
        )}

        {/* EA Connections */}
        <div className="mb-6">
          <ConnectionsPanel />
        </div>

        {/* Real-time Activity */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-xl font-semibold mb-4">Recent Activity</h2>
          <div className="space-y-2 max-h-40 overflow-y-auto">
            {wsMessages.length === 0 ? (
              <p className="text-gray-500 text-sm">No activity yet</p>
            ) : (
              wsMessages.map((msg, idx) => (
                <div key={idx} className="text-sm text-gray-700 font-mono bg-gray-50 p-2 rounded">
                  {msg}
                </div>
              ))
            )}
          </div>
        </div>

        {/* Copy Settings */}
        <div className="bg-white rounded-lg shadow-md p-6">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-xl font-semibold">Copy Settings</h2>
            <button
              onClick={() => setShowNewForm(true)}
              className="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded-lg"
            >
              + New Setting
            </button>
          </div>

          {/* Settings List */}
          {settings.length === 0 ? (
            <p className="text-gray-500 text-center py-8">No copy settings configured</p>
          ) : (
            <div className="space-y-4">
              {settings.map((setting) => (
                <SettingCard
                  key={setting.id}
                  setting={setting}
                  onToggle={toggleEnabled}
                  onEdit={setEditingSetting}
                  onDelete={deleteSetting}
                />
              ))}
            </div>
          )}
        </div>

        {/* New Setting Form Modal */}
        {showNewForm && (
          <SettingFormModal
            connections={connections}
            onClose={() => setShowNewForm(false)}
            onSubmit={createSetting}
          />
        )}

        {/* Edit Setting Form Modal */}
        {editingSetting && (
          <SettingFormModal
            setting={editingSetting}
            connections={connections}
            onClose={() => setEditingSetting(null)}
            onSubmit={(data) => updateSetting(editingSetting.id, data)}
          />
        )}
      </div>
    </div>
  )
}

// Setting Card Component
function SettingCard({ setting, onToggle, onEdit, onDelete }) {
  return (
    <div className="border border-gray-200 rounded-lg p-4 hover:shadow-md transition">
      <div className="flex justify-between items-start">
        <div className="flex-1">
          <div className="flex items-center gap-3 mb-2">
            <h3 className="text-lg font-semibold">
              {setting.master_account} → {setting.slave_account}
            </h3>
            <span className={`px-2 py-1 text-xs rounded-full ${
              setting.enabled
                ? 'bg-green-100 text-green-800'
                : 'bg-gray-100 text-gray-800'
            }`}>
              {setting.enabled ? 'Active' : 'Inactive'}
            </span>
          </div>

          <div className="grid grid-cols-2 gap-4 text-sm text-gray-600">
            <div>
              <span className="font-medium">Lot Multiplier:</span>{' '}
              {setting.lot_multiplier || 'N/A'}
            </div>
            <div>
              <span className="font-medium">Reverse Trade:</span>{' '}
              {setting.reverse_trade ? 'Yes' : 'No'}
            </div>
            {setting.symbol_mappings.length > 0 && (
              <div className="col-span-2">
                <span className="font-medium">Symbol Mappings:</span>{' '}
                {setting.symbol_mappings.length} configured
              </div>
            )}
          </div>
        </div>

        <div className="flex gap-2">
          <button
            onClick={() => onToggle(setting.id, setting.enabled)}
            className={`px-3 py-1 rounded text-sm font-medium ${
              setting.enabled
                ? 'bg-red-100 text-red-700 hover:bg-red-200'
                : 'bg-green-100 text-green-700 hover:bg-green-200'
            }`}
          >
            {setting.enabled ? 'Disable' : 'Enable'}
          </button>
          <button
            onClick={() => onEdit(setting)}
            className="px-3 py-1 bg-blue-100 text-blue-700 hover:bg-blue-200 rounded text-sm font-medium"
          >
            Edit
          </button>
          <button
            onClick={() => onDelete(setting.id)}
            className="px-3 py-1 bg-red-500 text-white hover:bg-red-600 rounded text-sm font-medium"
          >
            Delete
          </button>
        </div>
      </div>
    </div>
  )
}

// Setting Form Modal
function SettingFormModal({ setting, connections, onClose, onSubmit }) {
  const [formData, setFormData] = useState(setting || {
    master_account: '',
    slave_account: '',
    lot_multiplier: 1.0,
    reverse_trade: false,
    symbol_mappings: [],
    filters: {
      allowed_symbols: null,
      blocked_symbols: null,
      allowed_magic_numbers: null,
      blocked_magic_numbers: null,
    },
  })

  const handleSubmit = (e) => {
    e.preventDefault()
    onSubmit(formData)
  }

  // Filter connections by type
  const masterAccounts = connections.filter(c => c.ea_type === 'Master')
  const slaveAccounts = connections.filter(c => c.ea_type === 'Slave')

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
      <div className="bg-white rounded-lg max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <div className="p-6">
          <h2 className="text-2xl font-bold mb-4">
            {setting ? 'Edit Setting' : 'New Copy Setting'}
          </h2>

          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label className="block text-sm font-medium mb-1">Master Account</label>
              <select
                value={formData.master_account}
                onChange={(e) => setFormData({...formData, master_account: e.target.value})}
                className="w-full border border-gray-300 rounded px-3 py-2"
                required
              >
                <option value="">Select Master Account</option>
                {masterAccounts.map((conn) => (
                  <option key={conn.account_id} value={conn.account_id}>
                    {conn.account_id} - {conn.account_number} ({conn.broker}) - {conn.platform}
                  </option>
                ))}
              </select>
              {masterAccounts.length === 0 && (
                <p className="text-sm text-amber-600 mt-1">
                  No master accounts connected. Start a Master EA first.
                </p>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium mb-1">Slave Account</label>
              <select
                value={formData.slave_account}
                onChange={(e) => setFormData({...formData, slave_account: e.target.value})}
                className="w-full border border-gray-300 rounded px-3 py-2"
                required
              >
                <option value="">Select Slave Account</option>
                {slaveAccounts.map((conn) => (
                  <option key={conn.account_id} value={conn.account_id}>
                    {conn.account_id} - {conn.account_number} ({conn.broker}) - {conn.platform}
                  </option>
                ))}
              </select>
              {slaveAccounts.length === 0 && (
                <p className="text-sm text-amber-600 mt-1">
                  No slave accounts connected. Start a Slave EA first.
                </p>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium mb-1">Lot Multiplier</label>
              <input
                type="number"
                step="0.01"
                value={formData.lot_multiplier || ''}
                onChange={(e) => setFormData({...formData, lot_multiplier: parseFloat(e.target.value)})}
                className="w-full border border-gray-300 rounded px-3 py-2"
              />
            </div>

            <div className="flex items-center">
              <input
                type="checkbox"
                checked={formData.reverse_trade}
                onChange={(e) => setFormData({...formData, reverse_trade: e.target.checked})}
                className="mr-2"
              />
              <label className="text-sm font-medium">Reverse Trade (Buy ↔ Sell)</label>
            </div>

            <div className="flex gap-2 justify-end pt-4">
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 bg-gray-200 hover:bg-gray-300 rounded"
              >
                Cancel
              </button>
              <button
                type="submit"
                className="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded"
              >
                {setting ? 'Update' : 'Create'}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  )
}

export default App
