// Lot calculation mode type
export type LotCalculationMode = 'multiplier' | 'margin_ratio';

// Sync mode for existing positions when slave connects
export type SyncMode = 'skip' | 'limit_order' | 'market_order';

// Warning codes from Status Engine (snake_case to match API response)
export type WarningCode =
  | 'slave_web_ui_disabled'
  | 'slave_offline'
  | 'slave_auto_trading_disabled'
  | 'no_master_assigned'
  | 'master_web_ui_disabled'
  | 'master_offline'
  | 'master_auto_trading_disabled'
  | 'master_cluster_degraded';

export interface CopySettings {
  id: number;
  status: number; // Runtime status from server (0=DISABLED,1=ENABLED,2=CONNECTED)
  warning_codes?: WarningCode[]; // Warning codes from Status Engine
  enabled_flag?: boolean; // User intent flag managed by Web UI toggle
  master_account: string;
  slave_account: string;
  lot_calculation_mode?: LotCalculationMode;
  lot_multiplier: number | null;
  reverse_trade: boolean;
  symbol_mappings: SymbolMapping[];
  filters: TradeFilters;
  symbol_prefix?: string;
  symbol_suffix?: string;
  symbol_map?: string; // Comma-separated format for Slave EA
  source_lot_min?: number | null;
  source_lot_max?: number | null;
  // Open Sync Policy settings
  sync_mode?: SyncMode;
  limit_order_expiry_min?: number | null;  // minutes (0 = GTC)
  market_sync_max_pips?: number | null;    // pips
  max_slippage?: number | null;            // points
  copy_pending_orders?: boolean;
  // Trade Execution settings
  max_retries?: number;                     // Max order retry count (default: 3)
  max_signal_delay_ms?: number;             // Max signal delay in ms (default: 5000)
  use_pending_order_for_delayed?: boolean;  // Use pending order for delayed signals
}

export interface SymbolMapping {
  source_symbol: string;
  target_symbol: string;
}

export interface TradeFilters {
  allowed_symbols: string[] | null;
  blocked_symbols: string[] | null;
  allowed_magic_numbers: number[] | null;
  blocked_magic_numbers: number[] | null;
}

export interface EaConnection {
  account_id: string;
  ea_type: 'Master' | 'Slave';
  platform: 'MT4' | 'MT5';
  account_number: number;
  broker: string;
  account_name: string;
  server: string;
  balance: number;
  equity: number;
  currency: string;
  leverage: number;
  last_heartbeat: string;
  status: 'Online' | 'Offline' | 'Timeout';
  connected_at: string;
  open_positions?: number; // Number of currently open positions
  is_trade_allowed: boolean; // MT terminal's Algorithm Trading button state
  // Legacy fields for backwards compatibility
  role?: 'master' | 'slave';
  is_online?: boolean;
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

export interface CreateSettingsRequest {
  master_account: string;
  slave_account: string;
  lot_calculation_mode?: LotCalculationMode;
  lot_multiplier: number | null;
  reverse_trade: boolean;
  status: number; // 0=DISABLED, 2=CONNECTED (enabled)
  symbol_prefix?: string;
  symbol_suffix?: string;
  symbol_mappings?: string; // Comma-separated format: "XAUUSD=GOLD,EURUSD=EUR"
  source_lot_min?: number | null;
  source_lot_max?: number | null;
  // Open Sync Policy
  sync_mode?: SyncMode;
  limit_order_expiry_min?: number | null;
  market_sync_max_pips?: number | null;
  max_slippage?: number | null;
  copy_pending_orders?: boolean;
  // Trade Execution settings
  max_retries?: number;
  max_signal_delay_ms?: number;
  use_pending_order_for_delayed?: boolean;
}

// ConnectionsView specific types
export interface AccountInfo {
  id: string;
  name: string;
  accountType: 'master' | 'slave';
  platform?: 'MT4' | 'MT5';
  isOnline: boolean;
  isEnabled: boolean; // User's switch state (enabled_flag)
  isActive: boolean; // Calculated active state (ready for trading)
  hasError: boolean;
  hasWarning: boolean;
  errorMsg: string;
  isExpanded: boolean;
  masterRuntimeStatus?: number;
  masterIntentEnabled?: boolean;
  slaveIntentEnabled?: boolean;
  runtimeStatus?: number; // Effective runtime status (0/1/2) used for badges/colors
}

// MT4/MT5 Installation types
export type MtType = 'MT4' | 'MT5';

export type Architecture = '32-bit' | '64-bit';

export interface InstalledComponents {
  dll: boolean;
  master_ea: boolean;
  slave_ea: boolean;
}

// 2-port architecture: receiver (PULL) and unified publisher (PUB)
export interface EaPortConfig {
  receiver_port: number;
  publisher_port: number;
}

export interface MtInstallation {
  id: string;
  name: string;
  type: MtType;
  platform: Architecture;
  path: string;
  executable: string;
  version: string | null;  // DLL version = client version
  components: InstalledComponents;
  port_config?: EaPortConfig;
  port_mismatch?: boolean;
}

export interface DetectionSummary {
  total_found: number;
}

export interface MtInstallationsResponse {
  success: boolean;
  data: MtInstallation[];
  detection_summary: DetectionSummary;
  server_ports?: EaPortConfig;
}

// Master EA Configuration types
export interface MasterConfig {
  account_id: string;
  symbol_prefix?: string | null;
  symbol_suffix?: string | null;
  config_version: number;
  timestamp: string;
}

export interface UpdateMasterConfigRequest {
  symbol_prefix?: string | null;
  symbol_suffix?: string | null;
}

// TradeGroup (Master settings) types
export interface MasterSettings {
  enabled: boolean;
  symbol_prefix?: string | null;
  symbol_suffix?: string | null;
  config_version: number;
}

export interface TradeGroup {
  id: string; // Master account ID
  master_settings: MasterSettings;
  master_runtime_status?: number; // Actual status evaluated by server
  master_warning_codes?: WarningCode[]; // Warning codes from Status Engine
  created_at: string;
  updated_at: string;
}

// TradeGroupMember (Slave settings) types
export interface SlaveSettings {
  lot_calculation_mode: LotCalculationMode;
  lot_multiplier: number | null;
  reverse_trade: boolean;
  symbol_prefix?: string | null;
  symbol_suffix?: string | null;
  symbol_mappings: SymbolMapping[];
  filters: TradeFilters;
  config_version: number;
  // Lot filtering: min/max lot size from master to copy
  source_lot_min?: number | null;
  source_lot_max?: number | null;
  // Open Sync Policy settings
  sync_mode?: SyncMode;                  // Sync mode: skip, limit_order, market_order
  limit_order_expiry_min?: number | null; // minutes (0 = GTC)
  market_sync_max_pips?: number | null;   // max deviation in pips
  max_slippage?: number | null;           // Max slippage in points (default: 30)
  copy_pending_orders?: boolean;          // Copy pending orders (limit/stop)
  // Trade Execution settings
  max_retries?: number;                   // Max order retry count (default: 3)
  max_signal_delay_ms?: number;           // Max signal delay in ms (default: 5000)
  use_pending_order_for_delayed?: boolean; // Use pending order for delayed signals
}

export interface TradeGroupMember {
  id: number;
  trade_group_id: string; // Master account ID
  slave_account: string;
  slave_settings: SlaveSettings;
  status: number; // Runtime status evaluated by server (0=DISABLED,1=ENABLED,2=CONNECTED)
  warning_codes: WarningCode[]; // Warning codes from Status Engine
  enabled_flag: boolean; // User intent flag (true when switch is ON)
  created_at: string;
  updated_at: string;
}
