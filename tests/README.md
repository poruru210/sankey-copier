# Integration Tests

This directory contains integration tests for the SANKEY Copier MessagePack implementation.

## Test Structure

- `test_messagepack.py` - MessagePack serialization/deserialization tests
- `test_zmq_communication.py` - ZeroMQ communication pattern tests

## Prerequisites

Install Python dependencies:

```bash
pip install -r requirements.txt
```

## Running Tests

### Run all tests:
```bash
pytest -v
```

### Run specific test file:
```bash
pytest test_messagepack.py -v
pytest test_zmq_communication.py -v
```

### Run specific test:
```bash
pytest test_messagepack.py::TestMessagePackSerialization::test_register_message -v
```

### Run with output:
```bash
pytest -v -s
```

## Test Coverage

### MessagePack Serialization Tests (`test_messagepack.py`)

Tests MessagePack format compatibility:
- ✓ Register message serialization
- ✓ Unregister message serialization
- ✓ Heartbeat message serialization
- ✓ Trade signal (Open/Close/Modify) serialization
- ✓ Config message serialization
- ✓ Optional field omission
- ✓ Binary format verification
- ✓ Cross-language compatibility

### ZeroMQ Communication Tests (`test_zmq_communication.py`)

Tests ZeroMQ communication patterns:
- ✓ PUSH/PULL pattern (registration messages)
- ✓ PUB/SUB pattern with topic filtering (trade signals, config)
- ✓ Multiple subscribers
- ✓ Non-blocking receive
- ✓ Continuous heartbeat sending
- ✓ Concurrent communication
- ✓ Large message handling

## Rust Unit Tests

Run Rust unit tests:

```bash
cd ../mql-zmq-dll
cargo test
```

### Rust Test Coverage

- ✓ RegisterMessage serialization/deserialization
- ✓ UnregisterMessage serialization/deserialization
- ✓ HeartbeatMessage serialization/deserialization
- ✓ TradeSignalMessage serialization/deserialization
- ✓ Close action minimal fields
- ✓ ConfigMessage serialization/deserialization
- ✓ MessagePack size optimization (optional fields)
- ✓ Thread-safe buffer usage

## Testing 32-bit vs 64-bit

The Rust DLL supports both 32-bit (MT4) and 64-bit (MT5) platforms:

### Build for 32-bit (MT4):
```bash
cd ../mql-zmq-dll
cargo build --release --target i686-pc-windows-msvc
cargo test --target i686-pc-windows-msvc
```

### Build for 64-bit (MT5):
```bash
cd ../mql-zmq-dll
cargo build --release --target x86_64-pc-windows-msvc
cargo test --target x86_64-pc-windows-msvc
```

## Continuous Integration

Tests can be integrated into CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
name: Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-python@v4
        with:
          python-version: '3.11'
      - run: pip install -r tests/requirements.txt
      - run: pytest tests/ -v
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cd mql-zmq-dll && cargo test
```

## Troubleshooting

### Port conflicts
If tests fail due to port conflicts, the test framework automatically assigns different ports to each test. If issues persist, check for processes using ports 15555-15563:

```bash
netstat -ano | grep "15555"
```

### ZeroMQ socket cleanup
Tests automatically cleanup sockets in `teardown_method()`. If tests hang, ensure no zombie processes are holding sockets open.

### MessagePack compatibility
All tests verify that data can be serialized in Python and would be compatible with Rust/MQL deserialization (and vice versa).
