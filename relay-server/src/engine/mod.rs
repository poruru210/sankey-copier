use crate::models::{CopySettings, OrderType, SymbolConverter, TradeSignal};
use anyhow::Result;

#[cfg(test)]
mod tests;

pub struct CopyEngine;

impl CopyEngine {
    pub fn new() -> Self {
        Self
    }

    /// Apply filters to determine if a trade should be copied
    pub fn should_copy_trade(&self, signal: &TradeSignal, settings: &CopySettings) -> bool {
        // Check if copying is enabled
        if !settings.enabled {
            return false;
        }

        // Check symbol filters
        if let Some(ref allowed) = settings.filters.allowed_symbols {
            if !allowed.contains(&signal.symbol) {
                tracing::debug!("Symbol {} not in allowed list", signal.symbol);
                return false;
            }
        }

        if let Some(ref blocked) = settings.filters.blocked_symbols {
            if blocked.contains(&signal.symbol) {
                tracing::debug!("Symbol {} is blocked", signal.symbol);
                return false;
            }
        }

        // Check magic number filters
        if let Some(ref allowed) = settings.filters.allowed_magic_numbers {
            if !allowed.contains(&signal.magic_number) {
                tracing::debug!("Magic number {} not in allowed list", signal.magic_number);
                return false;
            }
        }

        if let Some(ref blocked) = settings.filters.blocked_magic_numbers {
            if blocked.contains(&signal.magic_number) {
                tracing::debug!("Magic number {} is blocked", signal.magic_number);
                return false;
            }
        }

        true
    }

    /// Transform trade signal for slave account
    pub fn transform_signal(
        &self,
        signal: TradeSignal,
        settings: &CopySettings,
        converter: &SymbolConverter,
    ) -> Result<TradeSignal> {
        let mut transformed = signal.clone();

        // Convert symbol
        transformed.symbol = converter.convert(&signal.symbol, &settings.symbol_mappings);

        // Apply lot multiplier
        if let Some(multiplier) = settings.lot_multiplier {
            transformed.lots = signal.lots * multiplier;
            // Round to 2 decimal places
            transformed.lots = (transformed.lots * 100.0).round() / 100.0;
        }

        // Reverse trade if enabled
        if settings.reverse_trade {
            transformed.order_type = Self::reverse_order_type(&signal.order_type);
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
