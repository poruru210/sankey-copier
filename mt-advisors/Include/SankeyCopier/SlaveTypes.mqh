//+------------------------------------------------------------------+
//|                                       SankeyCopierSlaveTypes.mqh |
//|                        Copyright 2025, SANKEY Copier Project      |
//|                     Slave EA specific type definitions            |
//+------------------------------------------------------------------+
// Purpose: Defines types and constants specific to Slave EA operation
// Why: Separates Slave-specific code from common shared code
//      to improve code organization and maintainability
#property copyright "Copyright 2025, SANKEY Copier Project"
#property strict

#ifndef SANKEY_COPIER_SLAVE_TYPES_MQH
#define SANKEY_COPIER_SLAVE_TYPES_MQH

#include "Common.mqh"

// =============================================================================
// Lot Calculation Mode Constants
// =============================================================================
// These constants define how the Slave EA calculates lot sizes for copied trades

#define LOT_CALC_MODE_MULTIPLIER    0  // Fixed multiplier (lot = master_lot * multiplier)
#define LOT_CALC_MODE_MARGIN_RATIO  1  // Based on equity ratio (lot = master_lot * slave_equity / master_equity)

// =============================================================================
// Copy Configuration Structure
// =============================================================================
// Holds all configuration parameters for copying trades from a specific Master EA
// Each Slave EA can have multiple CopyConfig entries (one per Master)

struct CopyConfig {
    string master_account;           // Master EA's account identifier
    string trade_group_id;           // Trade group for PUB/SUB topic subscription
    int    status;                   // Connection status (STATUS_DISABLED/ENABLED/CONNECTED)
    int    lot_calculation_mode;     // 0=multiplier, 1=margin_ratio
    double lot_multiplier;           // Fixed lot multiplier (when mode=0)
    bool   reverse_trade;            // Reverse trade direction (Buy->Sell, Sell->Buy)
    int    config_version;           // Configuration version for sync
    string symbol_prefix;            // Master's symbol prefix (for symbol transformation)
    string symbol_suffix;            // Master's symbol suffix (for symbol transformation)
    SymbolMapping symbol_mappings[]; // Symbol name mappings (e.g., XAUUSD -> GOLD)
    TradeFilters filters;            // Trade filters (allowed/blocked symbols/magic numbers)
    // Lot filtering
    double source_lot_min;           // Min lot from master (0 = no filter)
    double source_lot_max;           // Max lot from master (0 = no filter)
    double master_equity;            // Master's equity for margin_ratio mode calculation
};

#endif // SANKEY_COPIER_SLAVE_TYPES_MQH
