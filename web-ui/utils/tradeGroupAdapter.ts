// Trade Group Adapter
//
// Converts between new TradeGroups API format and legacy CopySettings format
// to minimize UI layer changes during migration.

import type {
  TradeGroup,
  TradeGroupMember,
  CopySettings,
  CreateSettingsRequest,
  SlaveSettings,
  MasterSettings,
  CreateTradeGroupRequest,
} from '@/types';

/**
 * Convert TradeGroup + TradeGroupMembers → CopySettings[]
 *
 * Flattens the hierarchical TradeGroup structure into a flat CopySettings array
 * for backwards compatibility with existing UI components.
 */
export function convertMembersToCopySettings(
  tradeGroups: TradeGroup[],
  allMembers: Map<string, TradeGroupMember[]>
): CopySettings[] {
  const copySettings: CopySettings[] = [];

  for (const tradeGroup of tradeGroups) {
    const members = allMembers.get(tradeGroup.id) || [];

    for (const member of members) {
      copySettings.push({
        id: member.id,
        status: member.status,
        warning_codes: member.warning_codes,
        enabled_flag: member.enabled_flag,
        master_account: member.trade_group_id,
        slave_account: member.slave_account,
        lot_calculation_mode: member.slave_settings.lot_calculation_mode,
        lot_multiplier: member.slave_settings.lot_multiplier,
        reverse_trade: member.slave_settings.reverse_trade,
        symbol_mappings: member.slave_settings.symbol_mappings,
        filters: member.slave_settings.filters,
        // Use slave's own settings - do NOT fallback to master's values
        // If slave's prefix/suffix is null/undefined, it should stay empty (not inherit from master)
        symbol_prefix: member.slave_settings.symbol_prefix ?? undefined,
        symbol_suffix: member.slave_settings.symbol_suffix ?? undefined,
        source_lot_min: member.slave_settings.source_lot_min ?? undefined,
        source_lot_max: member.slave_settings.source_lot_max ?? undefined,
        // Open Sync Policy settings
        sync_mode: member.slave_settings.sync_mode,
        limit_order_expiry_min: member.slave_settings.limit_order_expiry_min,
        market_sync_max_pips: member.slave_settings.market_sync_max_pips,
        max_slippage: member.slave_settings.max_slippage,
        copy_pending_orders: member.slave_settings.copy_pending_orders,
        // Trade Execution settings
        max_retries: member.slave_settings.max_retries,
        max_signal_delay_ms: member.slave_settings.max_signal_delay_ms,
        use_pending_order_for_delayed: member.slave_settings.use_pending_order_for_delayed,
      });
    }
  }

  return copySettings;
}

/**
 * Convert CopySettings → SlaveSettings (for updates)
 *
 * Extracts slave-specific settings from a CopySettings object.
 */
export function convertCopySettingsToSlaveSettings(settings: CopySettings): SlaveSettings {
  return {
    lot_calculation_mode: settings.lot_calculation_mode || 'multiplier',
    lot_multiplier: settings.lot_multiplier,
    reverse_trade: settings.reverse_trade,
    symbol_prefix: settings.symbol_prefix || null,
    symbol_suffix: settings.symbol_suffix || null,
    symbol_mappings: settings.symbol_mappings,
    filters: settings.filters,
    config_version: 0, // Will be set by server
    source_lot_min: settings.source_lot_min ?? null,
    source_lot_max: settings.source_lot_max ?? null,
    // Open Sync Policy settings
    sync_mode: settings.sync_mode,
    limit_order_expiry_min: settings.limit_order_expiry_min,
    market_sync_max_pips: settings.market_sync_max_pips,
    max_slippage: settings.max_slippage,
    copy_pending_orders: settings.copy_pending_orders,
    // Trade Execution settings
    max_retries: settings.max_retries,
    max_signal_delay_ms: settings.max_signal_delay_ms,
    use_pending_order_for_delayed: settings.use_pending_order_for_delayed,
  };
}

/**
 * Convert CreateSettingsRequest → TradeGroupMember creation data
 *
 * Prepares data for creating a new TradeGroupMember via the API.
 */
export function convertCreateRequestToMemberData(request: CreateSettingsRequest) {
  // Parse symbol_mappings from comma-separated format if provided
  const symbolMappings = request.symbol_mappings
    ? request.symbol_mappings.split(',').map(pair => {
      const [source, target] = pair.split('=');
      return { source_symbol: source.trim(), target_symbol: target.trim() };
    })
    : [];

  return {
    slave_account: request.slave_account,
    slave_settings: {
      lot_calculation_mode: request.lot_calculation_mode || 'multiplier',
      lot_multiplier: request.lot_multiplier,
      reverse_trade: request.reverse_trade,
      symbol_prefix: request.symbol_prefix || null,
      symbol_suffix: request.symbol_suffix || null,
      symbol_mappings: symbolMappings,
      filters: {
        allowed_symbols: null,
        blocked_symbols: null,
        allowed_magic_numbers: null,
        blocked_magic_numbers: null,
      },
      config_version: 0,
      source_lot_min: request.source_lot_min ?? null,
      source_lot_max: request.source_lot_max ?? null,
      // Open Sync Policy settings
      sync_mode: request.sync_mode,
      limit_order_expiry_min: request.limit_order_expiry_min,
      market_sync_max_pips: request.market_sync_max_pips,
      max_slippage: request.max_slippage,
      copy_pending_orders: request.copy_pending_orders,
      // Trade Execution settings
      max_retries: request.max_retries,
      max_signal_delay_ms: request.max_signal_delay_ms,
      use_pending_order_for_delayed: request.use_pending_order_for_delayed,
    },
    status: request.status,
    // Explicitly map status to enabled boolean.
    // Backend defaults to TRUE if this is missing, so we must provide it.
    enabled: request.status !== 0,
  };
}

/**
 * Convert CreateSettingsRequest → CreateTradeGroupRequest
 * 
 * Prepares data for creating a new TradeGroup (Master) + optional initial Member.
 */
export function convertCreateRequestToTradeGroupData(request: CreateSettingsRequest): CreateTradeGroupRequest {
  const memberData = convertCreateRequestToMemberData(request);

  return {
    id: request.master_account,
    master_settings: {
      enabled: false, // Default to false (safe mode) for new TradeGroups
      config_version: 1,
    },
    members: [{
      ...memberData,
      enabled: request.status !== 0, // 0=DISABLED
    }],
  };
}

/**
 * Extract master settings from CopySettings (for master config updates)
 *
 * Note: This is a best-effort extraction. In the new schema, master settings
 * are stored per TradeGroup, not per CopySettings.
 */
export function extractMasterSettings(settings: CopySettings): Partial<MasterSettings> {
  return {
    symbol_prefix: settings.symbol_prefix || null,
    symbol_suffix: settings.symbol_suffix || null,
  };
}
