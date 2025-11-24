// Location: mt-bridge/src/msgpack/tests/serialization_tests.rs
// Purpose: Tests for MessagePack serialization buffer thread safety
// Why: Ensures concurrent serialization operations are safe

use crate::msgpack::*;

#[test]
fn test_serialization_buffer_thread_safety() {
    use std::thread;

    // Test that multiple threads can serialize concurrently
    let handles: Vec<_> = (0..4)
        .map(|i| {
            thread::spawn(move || {
                let msg = HeartbeatMessage {
                    message_type: "Heartbeat".to_string(),
                    account_id: format!("account_{}", i),
                    balance: 10000.0 + i as f64,
                    equity: 10000.0 + i as f64,
                    open_positions: i,
                    timestamp: "2025-01-01T00:00:00Z".to_string(),
                    version: "test".to_string(),
                    ea_type: "Master".to_string(),
                    platform: "MT5".to_string(),
                    account_number: i as i64,
                    broker: "TestBroker".to_string(),
                    account_name: format!("Account {}", i),
                    server: "TestServer".to_string(),
                    currency: "USD".to_string(),
                    leverage: 100,
                    is_trade_allowed: true,
                    symbol_prefix: None,
                    symbol_suffix: None,
                    symbol_map: None,
                };

                // This should not panic
                rmp_serde::to_vec_named(&msg).expect("Serialization failed")
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}
