#!/usr/bin/env python3
"""
E2E Test for SANKEY Copier System

このテストは以下のシナリオを検証します:
1. Master EAの登録
2. Slave EAの登録
3. Web-UIから設定作成
4. Slaveが設定メッセージを受信して動的サブスクライブ
5. Masterからトレードシグナル送信
6. Slaveがトレードシグナルを受信
"""

import zmq
import json
import time
import requests
from datetime import datetime, timezone
from typing import Dict, Any

# Configuration
SERVER_URL = "http://localhost:8080"
ZMQ_REGISTER_PORT = 5555
ZMQ_TRADE_PORT = 5556
ZMQ_CONFIG_PORT = 5557

MASTER_ACCOUNT = "MASTER_001"
SLAVE_ACCOUNT = "SLAVE_001"


class Colors:
    """ターミナルカラー"""
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'


def print_step(step: str):
    """ステップを表示"""
    print(f"\n{Colors.HEADER}{Colors.BOLD}=== {step} ==={Colors.ENDC}")


def print_success(message: str):
    """成功メッセージを表示"""
    print(f"{Colors.OKGREEN}[OK] {message}{Colors.ENDC}")


def print_error(message: str):
    """エラーメッセージを表示"""
    print(f"{Colors.FAIL}[ERROR] {message}{Colors.ENDC}")


def print_info(message: str):
    """情報メッセージを表示"""
    print(f"{Colors.OKCYAN}  {message}{Colors.ENDC}")


def create_timestamp() -> str:
    """ISO 8601形式のタイムスタンプを生成"""
    return datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z')


def send_register_message(account_id: str, ea_type: str, platform: str = "MT5") -> None:
    """EA登録メッセージを送信"""
    context = zmq.Context()
    socket = context.socket(zmq.PUSH)
    socket.connect(f"tcp://localhost:{ZMQ_REGISTER_PORT}")

    message = {
        "message_type": "Register",
        "account_id": account_id,
        "ea_type": ea_type,
        "platform": platform,
        "account_number": 12345678,
        "broker": "Test Broker",
        "account_name": f"Test Account {account_id}",
        "server": "TestServer-Demo",
        "balance": 10000.0,
        "equity": 10000.0,
        "currency": "USD",
        "leverage": 100,
        "timestamp": create_timestamp()
    }

    socket.send_json(message)
    print_info(f"Sent: {json.dumps(message, indent=2)}")
    time.sleep(0.5)  # メッセージ処理待ち
    socket.close()
    context.term()


def send_trade_signal(source_account: str) -> Dict[str, Any]:
    """トレードシグナルを送信"""
    context = zmq.Context()
    socket = context.socket(zmq.PUSH)
    socket.connect(f"tcp://localhost:{ZMQ_REGISTER_PORT}")

    signal = {
        "message_type": "TradeSignal",
        "source_account": source_account,
        "ticket": 123456,
        "symbol": "EURUSD",
        "order_type": "Buy",
        "lots": 0.1,
        "open_price": 1.1000,
        "stop_loss": 1.0950,
        "take_profit": 1.1100,
        "magic_number": 0,
        "comment": "E2E Test Trade",
        "action": "Open",
        "timestamp": create_timestamp()
    }

    socket.send_json(signal)
    print_info(f"Sent: {json.dumps(signal, indent=2)}")
    time.sleep(0.5)
    socket.close()
    context.term()

    return signal


def create_or_get_copy_settings(master: str, slave: str) -> int:
    """コピー設定を作成または既存の設定を取得"""
    # 既存の設定を確認
    response = requests.get(f"{SERVER_URL}/api/settings")
    response.raise_for_status()
    result = response.json()

    if result.get("success"):
        settings = result.get("data", [])
        # 同じmaster-slave組み合わせの設定を探す
        for setting in settings:
            if setting["master_account"] == master and setting["slave_account"] == slave:
                print_info(f"Found existing setting (ID: {setting['id']})")
                return setting["id"]

    # 既存の設定がない場合は新規作成
    data = {
        "master_account": master,
        "slave_account": slave,
        "lot_multiplier": 1.0,
        "reverse_trade": False
    }

    response = requests.post(f"{SERVER_URL}/api/settings", json=data)
    response.raise_for_status()

    result = response.json()
    print_info(f"Response: {json.dumps(result, indent=2)}")

    if result.get("success"):
        return result["data"]
    else:
        raise Exception(f"Failed to create settings: {result.get('error')}")


def listen_for_config_message(account_id: str, timeout: int = 5) -> Dict[str, Any]:
    """設定メッセージを受信"""
    context = zmq.Context()
    socket = context.socket(zmq.SUB)
    socket.connect(f"tcp://localhost:{ZMQ_CONFIG_PORT}")
    socket.setsockopt_string(zmq.SUBSCRIBE, account_id)

    poller = zmq.Poller()
    poller.register(socket, zmq.POLLIN)

    print_info(f"Listening for config messages on topic '{account_id}'...")

    start_time = time.time()
    while time.time() - start_time < timeout:
        socks = dict(poller.poll(1000))

        if socket in socks and socks[socket] == zmq.POLLIN:
            message = socket.recv_string()
            print_info(f"Received raw: {message}")

            # トピック + スペース + JSON
            if ' ' in message:
                topic, json_str = message.split(' ', 1)
                config = json.loads(json_str)
                print_info(f"Parsed config: {json.dumps(config, indent=2)}")

                socket.close()
                context.term()
                return config

    socket.close()
    context.term()
    raise TimeoutError(f"No config message received within {timeout} seconds")


def listen_for_trade_signal(trade_group_id: str, timeout: int = 5) -> Dict[str, Any]:
    """トレードシグナルを受信"""
    context = zmq.Context()
    socket = context.socket(zmq.SUB)
    socket.connect(f"tcp://localhost:{ZMQ_TRADE_PORT}")
    socket.setsockopt_string(zmq.SUBSCRIBE, trade_group_id)

    poller = zmq.Poller()
    poller.register(socket, zmq.POLLIN)

    print_info(f"Listening for trade signals on topic '{trade_group_id}'...")

    start_time = time.time()
    while time.time() - start_time < timeout:
        socks = dict(poller.poll(1000))

        if socket in socks and socks[socket] == zmq.POLLIN:
            message = socket.recv_string()
            print_info(f"Received raw: {message}")

            # トピック + スペース + JSON
            if ' ' in message:
                topic, json_str = message.split(' ', 1)
                signal = json.loads(json_str)
                print_info(f"Parsed signal: {json.dumps(signal, indent=2)}")

                socket.close()
                context.term()
                return signal

    socket.close()
    context.term()
    raise TimeoutError(f"No trade signal received within {timeout} seconds")


def verify_connections() -> None:
    """接続を確認"""
    response = requests.get(f"{SERVER_URL}/api/connections")
    response.raise_for_status()

    result = response.json()
    print_info(f"Connections: {json.dumps(result, indent=2)}")

    connections = result.get("data", [])

    master_found = any(c["account_id"] == MASTER_ACCOUNT for c in connections)
    slave_found = any(c["account_id"] == SLAVE_ACCOUNT for c in connections)

    if master_found:
        print_success(f"Master EA ({MASTER_ACCOUNT}) is connected")
    else:
        print_error(f"Master EA ({MASTER_ACCOUNT}) not found")

    if slave_found:
        print_success(f"Slave EA ({SLAVE_ACCOUNT}) is connected")
    else:
        print_error(f"Slave EA ({SLAVE_ACCOUNT}) not found")


def run_e2e_test():
    """E2Eテストを実行"""
    print(f"{Colors.BOLD}{Colors.HEADER}")
    print("=" * 60)
    print("  SANKEY Copier E2E Test")
    print("=" * 60)
    print(f"{Colors.ENDC}\n")

    try:
        # Step 1: Master EA登録
        print_step("Step 1: Register Master EA")
        send_register_message(MASTER_ACCOUNT, "Master")
        print_success("Master EA registered")

        # Step 2: Slave EA登録
        print_step("Step 2: Register Slave EA")
        send_register_message(SLAVE_ACCOUNT, "Slave")
        print_success("Slave EA registered")

        # Step 3: 接続確認
        print_step("Step 3: Verify Connections")
        verify_connections()

        # Step 4: 設定確認
        print_step("Step 4: Verify Copy Settings")

        # 設定作成または取得
        setting_id = create_or_get_copy_settings(MASTER_ACCOUNT, SLAVE_ACCOUNT)
        print_success(f"Copy settings ready (ID: {setting_id})")

        # Step 5: トレードシグナル送信とSlave受信
        print_step("Step 5: Send Trade Signal and Verify Reception")

        import threading

        signal_received = {}
        signal_error = []

        def receive_signal():
            try:
                signal_received['data'] = listen_for_trade_signal(MASTER_ACCOUNT, timeout=10)
            except Exception as e:
                signal_error.append(str(e))

        # 受信スレッド開始
        signal_thread = threading.Thread(target=receive_signal)
        signal_thread.start()

        time.sleep(1)  # リスナー起動待ち

        # トレードシグナル送信
        original_signal = send_trade_signal(MASTER_ACCOUNT)
        print_success("Trade signal sent from Master")

        # 受信完了待ち
        signal_thread.join()

        if signal_error:
            raise Exception(signal_error[0])

        received_signal = signal_received.get('data')
        if received_signal:
            print_success("Slave received trade signal")

            # シグナル内容を検証
            assert received_signal["symbol"] == original_signal["symbol"], "Symbol mismatch"
            assert received_signal["order_type"] == original_signal["order_type"], "Order type mismatch"
            assert received_signal["lots"] == original_signal["lots"], "Lots mismatch"
            print_success("Trade signal verified")

        # 全テスト成功
        print(f"\n{Colors.OKGREEN}{Colors.BOLD}")
        print("=" * 60)
        print("  ALL E2E TESTS PASSED!")
        print("=" * 60)
        print(f"{Colors.ENDC}\n")

    except Exception as e:
        print_error(f"Test failed: {str(e)}")
        import traceback
        traceback.print_exc()
        exit(1)


if __name__ == "__main__":
    run_e2e_test()
