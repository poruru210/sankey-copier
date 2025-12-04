import type { EaConnection, CopySettings, TradeGroup, TradeGroupMember } from '@/types';

/**
 * Mock data for Playwright tests
 */

export const mockConnections: EaConnection[] = [
  // Master accounts
  {
    account_id: 'FxPro_12345001',
    ea_type: 'Master',
    platform: 'MT5',
    account_number: 12345001,
    broker: 'FxPro',
    account_name: 'Trading Account A',
    server: 'FxPro-Live',
    balance: 10000,
    equity: 10500,
    currency: 'USD',
    leverage: 500,
    last_heartbeat: new Date().toISOString(),
    status: 'Online',
    connected_at: new Date().toISOString(),
    open_positions: 3,
    is_trade_allowed: true,
  },
  {
    account_id: 'OANDA_67890002',
    ea_type: 'Master',
    platform: 'MT5',
    account_number: 67890002,
    broker: 'OANDA',
    account_name: 'Trading Account B',
    server: 'OANDA-Live',
    balance: 5000,
    equity: 5200,
    currency: 'USD',
    leverage: 200,
    last_heartbeat: new Date(Date.now() - 120000).toISOString(), // 2 minutes ago
    status: 'Offline',
    connected_at: new Date().toISOString(),
    open_positions: 0,
    is_trade_allowed: true,
  },
  {
    account_id: 'XM_11111003',
    ea_type: 'Master',
    platform: 'MT4',
    account_number: 11111003,
    broker: 'XM',
    account_name: 'Trading Account C',
    server: 'XM-Real',
    balance: 15000,
    equity: 15300,
    currency: 'EUR',
    leverage: 888,
    last_heartbeat: new Date().toISOString(),
    status: 'Online',
    connected_at: new Date().toISOString(),
    open_positions: 5,
    is_trade_allowed: true,
  },
  // Slave accounts
  {
    account_id: 'FxPro_22222004',
    ea_type: 'Slave',
    platform: 'MT5',
    account_number: 22222004,
    broker: 'FxPro',
    account_name: 'Copy Account 1',
    server: 'FxPro-Live',
    balance: 3000,
    equity: 3100,
    currency: 'USD',
    leverage: 500,
    last_heartbeat: new Date().toISOString(),
    status: 'Online',
    connected_at: new Date().toISOString(),
    open_positions: 2,
    is_trade_allowed: true,
  },
  {
    account_id: 'OANDA_33333005',
    ea_type: 'Slave',
    platform: 'MT5',
    account_number: 33333005,
    broker: 'OANDA',
    account_name: 'Copy Account 2',
    server: 'OANDA-Live',
    balance: 2000,
    equity: 2050,
    currency: 'USD',
    leverage: 200,
    last_heartbeat: new Date().toISOString(),
    status: 'Online',
    connected_at: new Date().toISOString(),
    open_positions: 1,
    is_trade_allowed: true,
  },
  {
    account_id: 'XM_44444006',
    ea_type: 'Slave',
    platform: 'MT4',
    account_number: 44444006,
    broker: 'XM',
    account_name: 'Copy Account 3',
    server: 'XM-Real',
    balance: 4000,
    equity: 4200,
    currency: 'EUR',
    leverage: 888,
    last_heartbeat: new Date().toISOString(),
    status: 'Online',
    connected_at: new Date().toISOString(),
    open_positions: 3,
    is_trade_allowed: true,
  },
  {
    account_id: 'FxPro_55555007',
    ea_type: 'Slave',
    platform: 'MT5',
    account_number: 55555007,
    broker: 'FxPro',
    account_name: 'Copy Account 4',
    server: 'FxPro-Live',
    balance: 1000,
    equity: 1020,
    currency: 'USD',
    leverage: 500,
    last_heartbeat: new Date().toISOString(),
    status: 'Online',
    connected_at: new Date().toISOString(),
    open_positions: 1,
    is_trade_allowed: true,
  },
];

export const mockSettings: CopySettings[] = [
  {
    id: 1,
    status: 2, // STATUS_CONNECTED
    runtime_status: 2,
    enabled_flag: true,
    master_account: 'FxPro_12345001',
    slave_account: 'FxPro_22222004',
    lot_multiplier: 1.5,
    reverse_trade: false,
    symbol_mappings: [],
    filters: {
      allowed_symbols: null,
      blocked_symbols: null,
      allowed_magic_numbers: null,
      blocked_magic_numbers: null,
    },
  },
  {
    id: 2,
    status: 1, // STATUS_ENABLED (waiting)
    runtime_status: 1,
    enabled_flag: true,
    master_account: 'FxPro_12345001',
    slave_account: 'OANDA_33333005',
    lot_multiplier: 0.5,
    reverse_trade: true,
    symbol_mappings: [
      { source_symbol: 'EURUSD', target_symbol: 'EURUSD.m' },
    ],
    filters: {
      allowed_symbols: ['EURUSD', 'GBPUSD'],
      blocked_symbols: null,
      allowed_magic_numbers: null,
      blocked_magic_numbers: null,
    },
  },
  {
    id: 3,
    status: 1,
    runtime_status: 1,
    enabled_flag: true,
    master_account: 'OANDA_67890002',
    slave_account: 'XM_44444006',
    lot_multiplier: 2.0,
    reverse_trade: false,
    symbol_mappings: [],
    filters: {
      allowed_symbols: null,
      blocked_symbols: ['USDJPY'],
      allowed_magic_numbers: null,
      blocked_magic_numbers: null,
    },
  },
  {
    id: 4,
    status: 0, // STATUS_DISABLED
    runtime_status: 0,
    enabled_flag: false,
    master_account: 'XM_11111003',
    slave_account: 'FxPro_55555007',
    lot_multiplier: 1.0,
    reverse_trade: false,
    symbol_mappings: [],
    filters: {
      allowed_symbols: null,
      blocked_symbols: null,
      allowed_magic_numbers: null,
      blocked_magic_numbers: null,
    },
  },
];

const now = new Date().toISOString();

function toTradeGroupMember(setting: CopySettings): TradeGroupMember {
  return {
    id: setting.id,
    trade_group_id: setting.master_account,
    slave_account: setting.slave_account,
    slave_settings: {
      lot_calculation_mode: setting.lot_calculation_mode ?? 'multiplier',
      lot_multiplier: setting.lot_multiplier ?? 1,
      reverse_trade: setting.reverse_trade ?? false,
      symbol_prefix: setting.symbol_prefix ?? null,
      symbol_suffix: setting.symbol_suffix ?? null,
      symbol_mappings: setting.symbol_mappings ?? [],
      filters: setting.filters,
      config_version: 1,
      source_lot_min: setting.source_lot_min ?? null,
      source_lot_max: setting.source_lot_max ?? null,
      sync_mode: setting.sync_mode,
      limit_order_expiry_min: setting.limit_order_expiry_min ?? null,
      market_sync_max_pips: setting.market_sync_max_pips ?? null,
      max_slippage: setting.max_slippage ?? null,
      copy_pending_orders: setting.copy_pending_orders ?? false,
      max_retries: setting.max_retries,
      max_signal_delay_ms: setting.max_signal_delay_ms,
      use_pending_order_for_delayed: setting.use_pending_order_for_delayed ?? false,
    },
    status: setting.status,
    runtime_status: setting.runtime_status ?? setting.status,
    enabled_flag: setting.enabled_flag ?? (setting.status !== 0),
    created_at: now,
    updated_at: now,
  };
}

const masterAccounts = Array.from(
  new Set([
    ...mockConnections.filter((conn) => conn.ea_type === 'Master').map((conn) => conn.account_id),
    ...mockSettings.map((setting) => setting.master_account),
  ])
);

const membersMap: Record<string, TradeGroupMember[]> = {};

for (const setting of mockSettings) {
  const member = toTradeGroupMember(setting);
  if (!membersMap[setting.master_account]) {
    membersMap[setting.master_account] = [];
  }
  membersMap[setting.master_account].push(member);
}

export const mockTradeGroupMembers: Record<string, TradeGroupMember[]> = membersMap;

export const mockTradeGroups: TradeGroup[] = masterAccounts.map((masterAccount) => {
  const members = membersMap[masterAccount] ?? [];
  const masterEnabled = members.some((member) => member.enabled_flag);
  const highestRuntime = members.reduce((max, member) => Math.max(max, member.runtime_status ?? 0), 0);

  return {
    id: masterAccount,
    master_settings: {
      enabled: masterEnabled,
      symbol_prefix: null,
      symbol_suffix: null,
      config_version: 1,
    },
    master_runtime_status: highestRuntime,
    created_at: now,
    updated_at: now,
  };
});

export const mockVictoriaLogsConfig = {
  configured: true,
  config: {
    host: 'http://localhost:9428',
    batch_size: 100,
    flush_interval_secs: 5,
    source: 'playwright-tests',
  },
  enabled: true,
};

export const mockVictoriaLogsSettings = {
  enabled: true,
  endpoint: 'http://localhost:9428/api/v1/write',
  batch_size: 100,
  flush_interval_secs: 5,
};

export const mockServerLogs = [
  {
    timestamp: now,
    level: 'INFO',
    message: 'Relay server boot completed',
  },
  {
    timestamp: new Date(Date.now() - 15_000).toISOString(),
    level: 'WARN',
    message: 'MT5 heartbeat delayed for account FxPro_12345001',
  },
  {
    timestamp: new Date(Date.now() - 30_000).toISOString(),
    level: 'INFO',
    message: 'VictoriaLogs flush completed (batch=250)',
  },
];
