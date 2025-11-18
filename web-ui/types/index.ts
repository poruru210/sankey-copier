export interface CopySettings {
  id: number;
  status: number; // 0=DISABLED, 1=ENABLED, 2=CONNECTED
  master_account: string;
  slave_account: string;
  lot_multiplier: number | null;
  reverse_trade: boolean;
  symbol_mappings: SymbolMapping[];
  filters: TradeFilters;
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
  lot_multiplier: number | null;
  reverse_trade: boolean;
  status: number; // 0=DISABLED, 1=ENABLED, 2=CONNECTED
}

// ConnectionsView specific types
export interface AccountInfo {
  id: string;
  name: string;
  isOnline: boolean;
  isEnabled: boolean;
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
