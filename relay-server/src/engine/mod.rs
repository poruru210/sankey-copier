use crate::models::{MasterSettings, OrderType, SymbolConverter, TradeGroupMember, TradeSignal};
use anyhow::Result;

#[cfg(test)]
mod tests;

pub struct CopyEngine;

impl CopyEngine {
    pub fn new() -> Self {
        Self
    }

    /// Apply filters to determine if a trade should be copied
    pub fn should_copy_trade(&self, signal: &TradeSignal, member: &TradeGroupMember) -> bool {
        // Check if copying is enabled and master is connected (STATUS_CONNECTED = 2)
        if !member.is_connected() {
            return false;
        }

        // Check symbol filters (only if signal has symbol)
        if let Some(ref symbol) = signal.symbol {
            if let Some(ref allowed) = member.slave_settings.filters.allowed_symbols {
                if !allowed.contains(symbol) {
                    tracing::debug!("Symbol {} not in allowed list", symbol);
                    return false;
                }
            }

            if let Some(ref blocked) = member.slave_settings.filters.blocked_symbols {
                if blocked.contains(symbol) {
                    tracing::debug!("Symbol {} is blocked", symbol);
                    return false;
                }
            }
        }

        // Check magic number filters (only if signal has magic_number)
        if let Some(magic_number) = signal.magic_number {
            if let Some(ref allowed) = member.slave_settings.filters.allowed_magic_numbers {
                if !allowed.contains(&magic_number) {
                    tracing::debug!("Magic number {} not in allowed list", magic_number);
                    return false;
                }
            }

            if let Some(ref blocked) = member.slave_settings.filters.blocked_magic_numbers {
                if blocked.contains(&magic_number) {
                    tracing::debug!("Magic number {} is blocked", magic_number);
                    return false;
                }
            }
        }

        true
    }

    /// Transform trade signal for slave account
    pub fn transform_signal(
        &self,
        signal: TradeSignal,
        member: &TradeGroupMember,
        _master_settings: &MasterSettings,
        converter: &SymbolConverter,
    ) -> Result<TradeSignal> {
        let mut transformed = signal.clone();

        // Convert symbol using member's symbol mappings (if present)
        if let Some(ref symbol) = signal.symbol {
            transformed.symbol =
                Some(converter.convert(symbol, &member.slave_settings.symbol_mappings));
        }

        // Apply lot multiplier (if lots present)
        if let (Some(lots), Some(multiplier)) = (signal.lots, member.slave_settings.lot_multiplier)
        {
            let new_lots = lots * multiplier;
            // Round to 2 decimal places
            transformed.lots = Some((new_lots * 100.0).round() / 100.0);
        }

        // Reverse trade if enabled (if order_type present)
        if member.slave_settings.reverse_trade {
            if let Some(ref order_type) = signal.order_type {
                transformed.order_type = Some(Self::reverse_order_type(order_type));
            }
        }

        Ok(transformed)
    }

    fn reverse_order_type(order_type: &OrderType) -> OrderType {
        match order_type {
            OrderType::Buy => OrderType::Sell,
            OrderType::Sell => OrderType::Buy,
            OrderType::BuyLimit => OrderType::SellLimit,
            OrderType::SellLimit => OrderType::BuyLimit,
            OrderType::BuyStop => OrderType::SellStop,
            OrderType::SellStop => OrderType::BuyStop,
        }
    }
}

impl Default for CopyEngine {
    fn default() -> Self {
        Self::new()
    }
}
