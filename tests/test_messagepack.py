#!/usr/bin/env python3
"""
MessagePack Integration Tests

Tests the MessagePack serialization/deserialization format used by the Forex Copier.
Ensures that messages can be properly exchanged between Master and Slave EAs.
"""

import msgpack
import pytest
from datetime import datetime


class TestMessagePackSerialization:
    """Test MessagePack serialization for all message types"""

    def test_register_message(self):
        """Test Register message serialization"""
        msg = {
            'message_type': 'Register',
            'account_id': 'test_account_123',
            'ea_type': 'Master',
            'platform': 'MT5',
            'account_number': 12345,
            'broker': 'TestBroker',
            'account_name': 'Test Account',
            'server': 'TestServer-Live',
            'balance': 10000.50,
            'equity': 10000.50,
            'currency': 'USD',
            'leverage': 100,
            'timestamp': '2025-01-01T00:00:00Z'
        }

        # Serialize
        packed = msgpack.packb(msg)
        assert len(packed) > 0, "Packed data should not be empty"

        # Deserialize
        unpacked = msgpack.unpackb(packed, raw=False)

        # Verify all fields
        assert unpacked['message_type'] == 'Register'
        assert unpacked['account_id'] == 'test_account_123'
        assert unpacked['ea_type'] == 'Master'
        assert unpacked['platform'] == 'MT5'
        assert unpacked['account_number'] == 12345
        assert unpacked['broker'] == 'TestBroker'
        assert unpacked['balance'] == 10000.50
        assert unpacked['equity'] == 10000.50

    def test_unregister_message(self):
        """Test Unregister message serialization"""
        msg = {
            'message_type': 'Unregister',
            'account_id': 'test_account_123',
            'timestamp': '2025-01-01T00:00:00Z'
        }

        packed = msgpack.packb(msg)
        unpacked = msgpack.unpackb(packed, raw=False)

        assert unpacked['message_type'] == 'Unregister'
        assert unpacked['account_id'] == 'test_account_123'
        assert unpacked['timestamp'] == '2025-01-01T00:00:00Z'

    def test_heartbeat_message(self):
        """Test Heartbeat message serialization"""
        msg = {
            'message_type': 'Heartbeat',
            'account_id': 'test_account_123',
            'balance': 10500.75,
            'equity': 10600.25,
            'open_positions': 3,
            'timestamp': '2025-01-01T00:00:00Z'
        }

        packed = msgpack.packb(msg)
        unpacked = msgpack.unpackb(packed, raw=False)

        assert unpacked['message_type'] == 'Heartbeat'
        assert unpacked['account_id'] == 'test_account_123'
        assert unpacked['balance'] == 10500.75
        assert unpacked['equity'] == 10600.25
        assert unpacked['open_positions'] == 3

    def test_trade_signal_open(self):
        """Test trade signal Open action"""
        msg = {
            'action': 'Open',
            'ticket': 123456,
            'symbol': 'EURUSD',
            'order_type': 'Buy',
            'lots': 0.1,
            'open_price': 1.0850,
            'stop_loss': 1.0800,
            'take_profit': 1.0900,
            'magic_number': 0,
            'comment': 'Test trade',
            'timestamp': '2025-01-01T00:00:00Z',
            'source_account': 'master_account'
        }

        packed = msgpack.packb(msg)
        unpacked = msgpack.unpackb(packed, raw=False)

        assert unpacked['action'] == 'Open'
        assert unpacked['ticket'] == 123456
        assert unpacked['symbol'] == 'EURUSD'
        assert unpacked['order_type'] == 'Buy'
        assert unpacked['lots'] == pytest.approx(0.1)
        assert unpacked['open_price'] == pytest.approx(1.0850)
        assert unpacked['stop_loss'] == pytest.approx(1.0800)
        assert unpacked['take_profit'] == pytest.approx(1.0900)

    def test_trade_signal_close(self):
        """Test trade signal Close action (minimal fields)"""
        msg = {
            'action': 'Close',
            'ticket': 123456,
            'timestamp': '2025-01-01T00:00:00Z',
            'source_account': 'master_account'
        }

        packed = msgpack.packb(msg)
        unpacked = msgpack.unpackb(packed, raw=False)

        assert unpacked['action'] == 'Close'
        assert unpacked['ticket'] == 123456
        assert 'symbol' not in unpacked or unpacked['symbol'] is None

    def test_trade_signal_modify(self):
        """Test trade signal Modify action"""
        msg = {
            'action': 'Modify',
            'ticket': 123456,
            'stop_loss': 1.0750,
            'take_profit': 1.0950,
            'timestamp': '2025-01-01T00:00:00Z',
            'source_account': 'master_account'
        }

        packed = msgpack.packb(msg)
        unpacked = msgpack.unpackb(packed, raw=False)

        assert unpacked['action'] == 'Modify'
        assert unpacked['ticket'] == 123456
        assert unpacked['stop_loss'] == pytest.approx(1.0750)
        assert unpacked['take_profit'] == pytest.approx(1.0950)

    def test_config_message(self):
        """Test configuration message"""
        msg = {
            'account_id': 'slave_account_123',
            'master_account': 'master_account_456',
            'trade_group_id': 'group_789',
            'timestamp': '2025-01-01T00:00:00Z',
            'enabled': True,
            'lot_multiplier': 1.5,
            'reverse_trade': False,
            'symbol_mappings': [
                {
                    'source_symbol': 'EURUSD',
                    'target_symbol': 'EURUSD.raw'
                }
            ],
            'filters': {
                'allowed_symbols': ['EURUSD', 'GBPUSD'],
                'blocked_symbols': None,
                'allowed_magic_numbers': [0, 123],
                'blocked_magic_numbers': None
            },
            'config_version': 1
        }

        packed = msgpack.packb(msg)
        unpacked = msgpack.unpackb(packed, raw=False)

        assert unpacked['account_id'] == 'slave_account_123'
        assert unpacked['enabled'] is True
        assert unpacked['lot_multiplier'] == pytest.approx(1.5)
        assert len(unpacked['symbol_mappings']) == 1
        assert unpacked['symbol_mappings'][0]['source_symbol'] == 'EURUSD'

    def test_optional_fields_omitted(self):
        """Test that None/null optional fields are omitted"""
        msg_full = {
            'action': 'Open',
            'ticket': 123456,
            'symbol': 'EURUSD',
            'order_type': 'Buy',
            'lots': 0.1,
            'open_price': 1.0850,
            'stop_loss': 1.0800,
            'take_profit': 1.0900,
            'magic_number': 0,
            'comment': 'Test',
            'timestamp': '2025-01-01T00:00:00Z',
            'source_account': 'master'
        }

        # Minimal message with only required fields
        msg_minimal = {
            'action': 'Close',
            'ticket': 123456,
            'timestamp': '2025-01-01T00:00:00Z',
            'source_account': 'master'
        }

        packed_full = msgpack.packb(msg_full)
        packed_minimal = msgpack.packb(msg_minimal)

        # Minimal message should be smaller
        assert len(packed_minimal) < len(packed_full), \
            f"Minimal message ({len(packed_minimal)} bytes) should be smaller than full ({len(packed_full)} bytes)"

    def test_binary_format(self):
        """Test that MessagePack produces binary data"""
        msg = {
            'action': 'Open',
            'ticket': 123456,
            'symbol': 'EURUSD'
        }

        packed = msgpack.packb(msg)

        # Should be bytes
        assert isinstance(packed, bytes)

        # Should be binary (not ASCII/UTF-8 text)
        assert packed != msg['action'].encode('utf-8')

    def test_cross_language_compatibility(self):
        """Test data types that should work across Rust/Python/MQL"""
        msg = {
            'string_field': 'test_string',
            'int_field': 12345,
            'long_field': 9223372036854775807,  # max i64
            'float_field': 1.23456789,
            'bool_field': True,
            'array_field': ['a', 'b', 'c'],
            'nested_field': {
                'key1': 'value1',
                'key2': 123
            }
        }

        packed = msgpack.packb(msg)
        unpacked = msgpack.unpackb(packed, raw=False)

        assert unpacked['string_field'] == 'test_string'
        assert unpacked['int_field'] == 12345
        assert unpacked['long_field'] == 9223372036854775807
        assert unpacked['float_field'] == pytest.approx(1.23456789)
        assert unpacked['bool_field'] is True
        assert unpacked['array_field'] == ['a', 'b', 'c']
        assert unpacked['nested_field']['key1'] == 'value1'


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
