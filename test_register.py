#!/usr/bin/env python3
"""
Test script to simulate EA registration messages
"""
import zmq
import json
import time

def send_register_message():
    context = zmq.Context()
    socket = context.socket(zmq.PUSH)
    socket.connect("tcp://localhost:5555")

    # Wait for connection
    time.sleep(0.5)

    # Send Master EA registration
    master_msg = {
        "message_type": "Register",
        "account_id": "TEST_MASTER_001",
        "ea_type": "Master",
        "platform": "MT4",
        "account_number": 12345678,
        "broker": "Test Broker Ltd",
        "account_name": "Test Master Account",
        "server": "TestServer-Live",
        "balance": 10000.50,
        "equity": 10050.25,
        "currency": "USD",
        "leverage": 100,
        "timestamp": "2025-11-03T15:00:00Z"
    }

    print("Sending Master EA registration...")
    socket.send_json(master_msg)
    print(f"Sent: {json.dumps(master_msg, indent=2)}")

    time.sleep(1)

    # Send Slave EA registration
    slave_msg = {
        "message_type": "Register",
        "account_id": "TEST_SLAVE_001",
        "ea_type": "Slave",
        "platform": "MT5",
        "account_number": 87654321,
        "broker": "Another Broker Inc",
        "account_name": "Test Slave Account",
        "server": "SlaveServer-Demo",
        "balance": 5000.00,
        "equity": 5100.75,
        "currency": "EUR",
        "leverage": 200,
        "timestamp": "2025-11-03T15:00:01Z"
    }

    print("\nSending Slave EA registration...")
    socket.send_json(slave_msg)
    print(f"Sent: {json.dumps(slave_msg, indent=2)}")

    time.sleep(1)

    # Send Heartbeat
    heartbeat_msg = {
        "message_type": "Heartbeat",
        "account_id": "TEST_MASTER_001",
        "balance": 10100.00,
        "equity": 10150.50,
        "open_positions": 3,
        "timestamp": "2025-11-03T15:00:10Z"
    }

    print("\nSending Heartbeat...")
    socket.send_json(heartbeat_msg)
    print(f"Sent: {json.dumps(heartbeat_msg, indent=2)}")

    socket.close()
    context.term()

    print("\n✓ Test messages sent successfully!")
    print("Check the server logs and API endpoint: http://localhost:8080/api/connections")

if __name__ == "__main__":
    send_register_message()
