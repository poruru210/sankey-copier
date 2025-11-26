// Lot calculation mode type
export type LotCalculationMode = 'multiplier' | 'margin_ratio';

export interface CopySettings {
  id: number;
  status: number; // 0=OFF (user disabled), 1=ON (user enabled)
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
  status: number; // 0=OFF (user disabled), 1=ON (user enabled)
  symbol_prefix?: string;
  symbol_suffix?: string;
  symbol_mappings?: string; // Comma-separated format: "XAUUSD=GOLD,EURUSD=EUR"
  source_lot_min?: number | null;
  source_lot_max?: number | null;
}

// ConnectionsView specific types
export interface AccountInfo {
  id: string;
  name: string;
  platform?: 'MT4' | 'MT5';
  isOnline: boolean;
  isEnabled: boolean; // User's switch state (status > 0)
  isActive: boolean; // Calculated active state (ready for trading)
  hasError: boolean;
  hasWarning: boolean;
  errorMsg: string;
  isExpanded: boolean;
}

// MT4/MT5 Installation types
export type MtType = 'MT4' | 'MT5';

export type Architecture = '32-bit' | '64-bit';

export interface InstalledComponents {
  dll: boolean;
  master_ea: boolean;
  slave_ea: boolean;
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
}

export interface DetectionSummary {
  total_found: number;
}

export interface MtInstallationsResponse {
  success: boolean;
  data: MtInstallation[];
  detection_summary: DetectionSummary;
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
  symbol_prefix?: string | null;
  symbol_suffix?: string | null;
  config_version: number;
}

export interface TradeGroup {
  id: string; // Master account ID
  master_settings: MasterSettings;
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
}

export interface TradeGroupMember {
  id: number;
  trade_group_id: string; // Master account ID
  slave_account: string;
  slave_settings: SlaveSettings;
  status: number; // 0=DISABLED, 1=ENABLED, 2=CONNECTED
  created_at: string;
  updated_at: string;
}
