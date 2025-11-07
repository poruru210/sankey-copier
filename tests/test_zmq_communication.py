#!/usr/bin/env python3
"""
ZeroMQ Communication Integration Tests

Tests the ZeroMQ message exchange patterns used by the Forex Copier.
Simulates Master and Slave EA communication.
"""

import zmq
import msgpack
import time
import pytest
import threading
from typing import Optional


class TestZMQCommunication:
    """Test ZeroMQ communication patterns"""

    def setup_method(self):
        """Setup ZeroMQ context for each test"""
        self.context = zmq.Context()

    def teardown_method(self):
        """Cleanup ZeroMQ context after each test"""
        self.context.term()

    def test_push_pull_pattern(self):
        """Test PUSH/PULL pattern for registration messages"""
        # Create PUSH socket (Master)
        push_socket = self.context.socket(zmq.PUSH)
        push_socket.bind("tcp://127.0.0.1:15555")

        # Create PULL socket (Server)
        pull_socket = self.context.socket(zmq.PULL)
        pull_socket.connect("tcp://127.0.0.1:15555")

        time.sleep(0.1)  # Allow sockets to connect

        # Send registration message
        register_msg = {
            'message_type': 'Register',
            'account_id': 'test_account',
            'ea_type': 'Master',
            'platform': 'MT5'
        }
        push_socket.send(msgpack.packb(register_msg))

        # Receive message
        received = pull_socket.recv()
        unpacked = msgpack.unpackb(received, raw=False)

        assert unpacked['message_type'] == 'Register'
        assert unpacked['account_id'] == 'test_account'

        push_socket.close()
        pull_socket.close()

    def test_pub_sub_pattern_with_topic(self):
        """Test PUB/SUB pattern with topic filtering"""
        # Create PUB socket (Master)
        pub_socket = self.context.socket(zmq.PUB)
        pub_socket.bind("tcp://127.0.0.1:15556")

        # Create SUB socket (Slave)
        sub_socket = self.context.socket(zmq.SUB)
        sub_socket.connect("tcp://127.0.0.1:15556")

        # Subscribe to specific trade group
        trade_group_id = "group_123"
        sub_socket.setsockopt_string(zmq.SUBSCRIBE, trade_group_id)

        time.sleep(0.5)  # PUB/SUB needs more time to establish

        # Send trade signal with topic
        trade_signal = {
            'action': 'Open',
            'ticket': 123456,
            'symbol': 'EURUSD',
            'order_type': 'Buy',
            'lots': 0.1
        }

        # Format: topic + space + MessagePack data
        message = trade_group_id.encode('utf-8') + b' ' + msgpack.packb(trade_signal)
        pub_socket.send(message)

        # Receive message
        received = sub_socket.recv()

        # Parse topic and payload
        space_pos = received.find(b' ')
        topic = received[:space_pos].decode('utf-8')
        payload = received[space_pos + 1:]

        unpacked = msgpack.unpackb(payload, raw=False)

        assert topic == trade_group_id
        assert unpacked['action'] == 'Open'
        assert unpacked['ticket'] == 123456

        pub_socket.close()
        sub_socket.close()

    def test_pub_sub_topic_filtering(self):
        """Test that SUB only receives subscribed topics"""
        pub_socket = self.context.socket(zmq.PUB)
        pub_socket.bind("tcp://127.0.0.1:15557")

        sub_socket = self.context.socket(zmq.SUB)
        sub_socket.connect("tcp://127.0.0.1:15557")
        sub_socket.setsockopt_string(zmq.SUBSCRIBE, "group_A")

        time.sleep(0.5)

        # Send messages to different groups
        msg = msgpack.packb({'test': 'data'})
        pub_socket.send(b"group_A " + msg)
        pub_socket.send(b"group_B " + msg)
        pub_socket.send(b"group_A " + msg)

        # Should only receive group_A messages
        received_topics = []
        for _ in range(2):
            received = sub_socket.recv()
            topic = received.split(b' ')[0].decode('utf-8')
            received_topics.append(topic)

        assert len(received_topics) == 2
        assert all(topic == "group_A" for topic in received_topics)

        pub_socket.close()
        sub_socket.close()

    def test_multiple_subscribers(self):
        """Test PUB socket can send to multiple SUB sockets"""
        pub_socket = self.context.socket(zmq.PUB)
        pub_socket.bind("tcp://127.0.0.1:15558")

        # Create 3 slave subscribers
        sub_sockets = []
        for i in range(3):
            sub = self.context.socket(zmq.SUB)
            sub.connect("tcp://127.0.0.1:15558")
            sub.setsockopt_string(zmq.SUBSCRIBE, "")  # Subscribe to all
            sub_sockets.append(sub)

        time.sleep(0.5)

        # Send trade signal
        trade_signal = msgpack.packb({'action': 'Open', 'ticket': 999})
        pub_socket.send(b"trade_group " + trade_signal)

        # All subscribers should receive it
        for sub in sub_sockets:
            received = sub.recv()
            payload = received.split(b' ', 1)[1]
            unpacked = msgpack.unpackb(payload, raw=False)
            assert unpacked['ticket'] == 999

        pub_socket.close()
        for sub in sub_sockets:
            sub.close()

    def test_non_blocking_receive(self):
        """Test non-blocking receive (DONTWAIT flag)"""
        pull_socket = self.context.socket(zmq.PULL)
        pull_socket.bind("tcp://127.0.0.1:15559")

        # Try to receive without any data (should raise EAGAIN)
        with pytest.raises(zmq.Again):
            pull_socket.recv(zmq.DONTWAIT)

        pull_socket.close()

    def test_heartbeat_continuous_send(self):
        """Test continuous heartbeat sending (simulating EA behavior)"""
        push_socket = self.context.socket(zmq.PUSH)
        push_socket.bind("tcp://127.0.0.1:15560")

        pull_socket = self.context.socket(zmq.PULL)
        pull_socket.connect("tcp://127.0.0.1:15560")

        time.sleep(0.1)

        # Send 5 heartbeats
        for i in range(5):
            heartbeat = {
                'message_type': 'Heartbeat',
                'account_id': 'test_account',
                'balance': 10000.0 + i,
                'equity': 10000.0 + i,
                'open_positions': i
            }
            push_socket.send(msgpack.packb(heartbeat))

        # Receive all heartbeats
        for i in range(5):
            received = pull_socket.recv()
            unpacked = msgpack.unpackb(received, raw=False)
            assert unpacked['message_type'] == 'Heartbeat'
            assert unpacked['open_positions'] == i

        push_socket.close()
        pull_socket.close()

    def test_config_update_pub_sub(self):
        """Test configuration update via PUB/SUB"""
        pub_socket = self.context.socket(zmq.PUB)
        pub_socket.bind("tcp://127.0.0.1:15561")

        sub_socket = self.context.socket(zmq.SUB)
        sub_socket.connect("tcp://127.0.0.1:15561")
        account_id = "slave_account_123"
        sub_socket.setsockopt_string(zmq.SUBSCRIBE, account_id)

        time.sleep(0.5)

        # Send config update
        config = {
            'account_id': 'slave_account_123',
            'master_account': 'master_account_456',
            'trade_group_id': 'group_789',
            'enabled': True,
            'lot_multiplier': 1.5,
            'reverse_trade': False,
            'config_version': 1
        }

        message = account_id.encode('utf-8') + b' ' + msgpack.packb(config)
        pub_socket.send(message)

        # Receive config
        received = sub_socket.recv()
        space_pos = received.find(b' ')
        payload = received[space_pos + 1:]
        unpacked = msgpack.unpackb(payload, raw=False)

        assert unpacked['account_id'] == 'slave_account_123'
        assert unpacked['enabled'] is True
        assert unpacked['lot_multiplier'] == pytest.approx(1.5)

        pub_socket.close()
        sub_socket.close()

    def test_concurrent_communication(self):
        """Test concurrent sending and receiving"""
        pub_socket = self.context.socket(zmq.PUB)
        pub_socket.bind("tcp://127.0.0.1:15562")

        sub_socket = self.context.socket(zmq.SUB)
        sub_socket.connect("tcp://127.0.0.1:15562")
        sub_socket.setsockopt_string(zmq.SUBSCRIBE, "")

        time.sleep(0.5)

        received_count = [0]
        expected_count = 10

        def receiver():
            for _ in range(expected_count):
                sub_socket.recv()
                received_count[0] += 1

        # Start receiver thread
        receiver_thread = threading.Thread(target=receiver)
        receiver_thread.start()

        # Send messages concurrently
        for i in range(expected_count):
            msg = msgpack.packb({'sequence': i})
            pub_socket.send(b"test " + msg)
            time.sleep(0.01)

        receiver_thread.join(timeout=2.0)

        assert received_count[0] == expected_count

        pub_socket.close()
        sub_socket.close()

    def test_large_message(self):
        """Test sending large MessagePack messages"""
        push_socket = self.context.socket(zmq.PUSH)
        push_socket.bind("tcp://127.0.0.1:15563")

        pull_socket = self.context.socket(zmq.PULL)
        pull_socket.connect("tcp://127.0.0.1:15563")

        time.sleep(0.1)

        # Create large message (simulate many symbol mappings)
        large_msg = {
            'account_id': 'test',
            'symbol_mappings': [
                {'source_symbol': f'SYMBOL{i}', 'target_symbol': f'TARGET{i}'}
                for i in range(100)
            ]
        }

        push_socket.send(msgpack.packb(large_msg))
        received = pull_socket.recv()
        unpacked = msgpack.unpackb(received, raw=False)

        assert len(unpacked['symbol_mappings']) == 100

        push_socket.close()
        pull_socket.close()


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
