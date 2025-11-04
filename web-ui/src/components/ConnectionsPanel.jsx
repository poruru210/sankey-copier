import React, { useState, useEffect } from 'react';

const ConnectionsPanel = () => {
  const [connections, setConnections] = useState([]);
  const [settings, setSettings] = useState([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(null);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const [connRes, settingsRes] = await Promise.all([
          fetch('/api/connections'),
          fetch('/api/settings')
        ]);

        if (!connRes.ok || !settingsRes.ok) {
          throw new Error('Failed to fetch data');
        }

        const connResult = await connRes.json();
        const settingsResult = await settingsRes.json();

        setConnections(connResult.data || []);
        setSettings(settingsResult.data || []);
        setError(null);
      } catch (err) {
        setError(err.message);
      } finally {
        setIsLoading(false);
      }
    };

    fetchData();

    // Refetch every 5 seconds
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, []);

  if (isLoading) {
    return (
      <div className="bg-white rounded-lg shadow-md p-6">
        <h2 className="text-xl font-semibold mb-4">EA Connections</h2>
        <p className="text-gray-500">Loading...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-white rounded-lg shadow-md p-6">
        <h2 className="text-xl font-semibold mb-4">EA Connections</h2>
        <p className="text-red-600">Error: {error}</p>
      </div>
    );
  }

  const getStatusColor = (status) => {
    switch (status) {
      case 'Online':
        return '#4CAF50';
      case 'Timeout':
        return '#FF9800';
      case 'Offline':
        return '#F44336';
      default:
        return '#9E9E9E';
    }
  };

  const getStatusBadge = (status) => {
    return (
      <span
        style={{
          backgroundColor: getStatusColor(status),
          color: 'white',
          padding: '3px 8px',
          borderRadius: '12px',
          fontSize: '11px',
          fontWeight: 'bold',
        }}
      >
        {status}
      </span>
    );
  };

  const masterConnections = connections.filter((c) => c.ea_type === 'Master');
  const slaveConnections = connections.filter((c) => c.ea_type === 'Slave');

  // Build connection map: master -> slaves
  const connectionMap = {};
  settings.forEach(setting => {
    if (setting.enabled) {
      if (!connectionMap[setting.master_account]) {
        connectionMap[setting.master_account] = [];
      }
      connectionMap[setting.master_account].push({
        slaveId: setting.slave_account,
        settingId: setting.id,
        lotMultiplier: setting.lot_multiplier,
        reverseTrade: setting.reverse_trade
      });
    }
  });

  const renderAccountCard = (conn, isReceiver = false) => {
    const bgColor = isReceiver ? '#FFF3E0' : '#E3F2FD';
    const textColor = isReceiver ? '#E65100' : '#0D47A1';
    const label = isReceiver ? 'RECEIVER' : 'SOURCE';

    return (
      <div
        style={{
          border: `2px solid ${conn.status === 'Online' ? (isReceiver ? '#FF9800' : '#2196F3') : '#ccc'}`,
          borderRadius: '12px',
          padding: '16px',
          backgroundColor: conn.status === 'Online' ? bgColor : '#f5f5f5',
          minWidth: '280px',
          boxShadow: '0 2px 8px rgba(0,0,0,0.1)',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
          <span
            style={{
              backgroundColor: textColor,
              color: 'white',
              padding: '4px 10px',
              borderRadius: '4px',
              fontSize: '11px',
              fontWeight: 'bold',
              letterSpacing: '0.5px'
            }}
          >
            {label}
          </span>
          {getStatusBadge(conn.status)}
        </div>

        <div style={{ marginBottom: '12px' }}>
          <div style={{ fontSize: '18px', fontWeight: 'bold', color: '#333', marginBottom: '4px' }}>
            {conn.account_id}
          </div>
          <div style={{ fontSize: '12px', color: '#666' }}>
            {conn.account_number} • {conn.platform}
          </div>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px', fontSize: '12px' }}>
          <div>
            <div style={{ color: '#999', marginBottom: '2px' }}>Balance</div>
            <div style={{ fontWeight: '600', color: '#4CAF50' }}>
              {conn.balance.toFixed(2)} {conn.currency}
            </div>
          </div>
          <div>
            <div style={{ color: '#999', marginBottom: '2px' }}>Equity</div>
            <div style={{ fontWeight: '600', color: '#2196F3' }}>
              {conn.equity.toFixed(2)} {conn.currency}
            </div>
          </div>
        </div>

        <div style={{ marginTop: '12px', paddingTop: '12px', borderTop: '1px solid #ddd' }}>
          <div style={{ fontSize: '11px', color: '#666' }}>
            <div>{conn.broker}</div>
            <div>Leverage: 1:{conn.leverage}</div>
          </div>
        </div>
      </div>
    );
  };

  const renderConnectionGroup = (masterConn) => {
    const slaves = connectionMap[masterConn.account_id] || [];
    const connectedSlaves = slaves.map(s =>
      slaveConnections.find(sc => sc.account_id === s.slaveId)
    ).filter(Boolean);

    return (
      <div
        key={masterConn.account_id}
        style={{
          marginBottom: '32px',
          padding: '20px',
          backgroundColor: '#fafafa',
          borderRadius: '12px',
          border: '1px solid #e0e0e0'
        }}
      >
        <div style={{ display: 'flex', alignItems: 'flex-start', gap: '40px' }}>
          {/* Master (Source) */}
          <div>{renderAccountCard(masterConn, false)}</div>

          {/* Arrow and Connection Info */}
          {connectedSlaves.length > 0 && (
            <div style={{ display: 'flex', alignItems: 'center', flex: 1 }}>
              <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: '16px' }}>
                {connectedSlaves.map((slaveConn, idx) => {
                  const slaveInfo = slaves.find(s => s.slaveId === slaveConn.account_id);
                  return (
                    <div key={idx} style={{ display: 'flex', alignItems: 'center', gap: '20px' }}>
                      {/* Connection Arrow */}
                      <div style={{
                        display: 'flex',
                        alignItems: 'center',
                        flex: '0 0 auto',
                        padding: '8px 16px',
                        backgroundColor: '#fff',
                        borderRadius: '8px',
                        border: '1px solid #ddd'
                      }}>
                        <div style={{ fontSize: '11px', color: '#666', marginRight: '8px' }}>
                          {slaveInfo.reverseTrade ? '🔄 Reverse' : '➡️ Copy'}
                        </div>
                        <div style={{ fontSize: '11px', fontWeight: '600', color: '#333' }}>
                          Lot: ×{slaveInfo.lotMultiplier || 1.0}
                        </div>
                      </div>

                      {/* Slave (Receiver) */}
                      <div>{renderAccountCard(slaveConn, true)}</div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* No connection message */}
          {connectedSlaves.length === 0 && (
            <div style={{
              flex: 1,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              padding: '20px',
              color: '#999',
              fontStyle: 'italic',
              fontSize: '14px'
            }}>
              No receivers configured
            </div>
          )}
        </div>
      </div>
    );
  };

  // Unconnected slaves
  const connectedSlaveIds = new Set(
    Object.values(connectionMap).flat().map(s => s.slaveId)
  );
  const unconnectedSlaves = slaveConnections.filter(
    sc => !connectedSlaveIds.has(sc.account_id)
  );

  return (
    <div className="bg-white rounded-lg shadow-md p-6">
      <h2 className="text-xl font-semibold mb-6">Copy Connections</h2>

      {connections.length === 0 ? (
        <p className="text-gray-500">No EAs connected. Start your MT4/MT5 Expert Advisors to see them here.</p>
      ) : (
        <>
          {/* Connected Masters with their Slaves */}
          {masterConnections.length > 0 ? (
            <div>
              {masterConnections.map(renderConnectionGroup)}
            </div>
          ) : (
            <div style={{
              padding: '20px',
              backgroundColor: '#f5f5f5',
              borderRadius: '8px',
              textAlign: 'center',
              color: '#666',
              marginBottom: '20px'
            }}>
              No master accounts connected
            </div>
          )}

          {/* Unconnected Slaves */}
          {unconnectedSlaves.length > 0 && (
            <div style={{ marginTop: '32px', paddingTop: '32px', borderTop: '2px solid #e0e0e0' }}>
              <h3 style={{ fontSize: '16px', fontWeight: '600', color: '#666', marginBottom: '16px' }}>
                Unconnected Receivers ({unconnectedSlaves.length})
              </h3>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: '16px' }}>
                {unconnectedSlaves.map(conn => (
                  <div key={conn.account_id}>
                    {renderAccountCard(conn, true)}
                  </div>
                ))}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
};

export default ConnectionsPanel;
