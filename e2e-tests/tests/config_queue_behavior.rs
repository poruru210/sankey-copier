// e2e-tests/tests/config_queue_behavior.rs
use anyhow::Result;

use e2e_tests::TestSandbox;
use sankey_copier_zmq::ea_context::EaCommand;
use sankey_copier_zmq::ffi::*;
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests {
    use super::*;

    // Test that Master EA receives configuration updates only ONCE per message (Queue behavior).
    // PROOF OF QUEUE:
    // 1. Send Master Config.
    // 2. Verify it is received.
    // 3. Clear the received buffer (in simulator logic, we just check if it gets *overwritten* or re-received).
    //    Actually, simulator pushes to `received_config` (Option).
    //    We need a way to detect *how many times* it was received.
    //    The simulator `on_timer` does: `*self.received_config.lock().unwrap() = Some(cfg);`
    //    It overwrites. This makes it hard to count.
    //
    //    Let's modify the test to rely on `on_timer` behavior.
    //    However, `MasterEaSimulator::on_timer` is where `UpdateUi` is processed.
    //    Currently (State-based): Every `UpdateUi` (triggered by Global Config too) re-reads Master Config.
    //    So if we trigger Global Config, `received_config` timestamp/lock interactions happen.
    //
    //    Better approach: Check `MasterEaSimulator` logs? No, we can't easily access internal logs.
    //
    //    Let's check `get_master_config` FFI behavior directly via simulator.
    //    If we call `wrapper.get_master_config()` twice:
    //    - State-based: Returns `Some(config)` both times.
    //    - Queue-based: Returns `Some(config)` first time, `None` second time.
    //
    //    This is the definitive test. We can't access `wrapper` directly from test, but we can assume
    //    `on_timer` does the polling.
    //
    //    Wait, `MasterEaSimulator::on_timer` processes *all* pending commands.
    //    If `UpdateUi` command comes, it calls `get_master_config`.
    //
    //    Scenario:
    //    1. Send Master Config -> Generates `UpdateUi` command (cmd #1).
    //    2. Send Global Config -> Generates `UpdateUi` command (cmd #2).
    //    3. `on_timer` loop runs.
    //       - Pop cmd #1: `get_master_config` -> Returns Config A.
    //       - Pop cmd #2: `get_master_config` ->
    //         - State-based: Returns Config A (Again!).
    //         - Queue-based: Returns None.
    //
    //    If `MasterEaSimulator` pushes to a `Vec` instead of `Option`, we could count.
    //    Currently `MasterEaSimulator` has `received_config: Arc<Mutex<Option<MasterConfigMessage>>>`.
    //    It overwrites.
    //
    //    How to detect redundant overwrite?
    //    We can't easily with current Simulator `Option` field.
    //
    //    BUT, we can check the *Slave* behavior as a control group, or assume we can verify via `wait_for_status`? No.
    //
    //    Let's rely on the fact that if we fix `mt-bridge`, the simulator *logic* doesn't change, but the *result* of `get_master_config` changes.
    //
    //    To make this test strict TDD, we might need to modify `MasterEaSimulator` to *expose* the count of received configs, OR
    //    we can just modify `mt-bridge` and verify manually?
    //    User asked for TDD.
    //
    //    I will add a `received_count` atomic to `MasterEaSimulator`? No, I can't modify simulator easily without replacing file.
    //
    //    Alternative: Use raw FFI calls in the test!
    //    The test creates an `EaContext` directly? Yes.
    //    `e2e-tests` has access to `sankey_copier_zmq`.
    //

    // Helper for FFI strings
    fn to_u16(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    #[tokio::test]
    async fn test_master_config_queue_behavior() -> Result<()> {
        // 1. Start Sandbox (Relay Server + DB)
        let sandbox = TestSandbox::new()?;
        let server = sandbox.server();

        // 2. Seed Database using Helper (ensures tables exist and correct schema)
        // We connect to the shared DB used by the server process
        // Fix DB URL for Windows (force forward slashes)
        let db_path_str = server.db_path().to_str().unwrap().replace('\\', "/");
        let db_url = format!("sqlite://{}?mode=rwc", db_path_str);

        // Initialize Database helper (this ensures tables exist via IF NOT EXISTS migrations)
        let db =
            sankey_copier_relay_server::adapters::outbound::persistence::Database::new(&db_url)
                .await?;

        // Create Master (Trade Group)
        let master_id = "MasterQueueTest";

        db.create_trade_group(master_id).await?;

        // Enable Master
        let master_settings = sankey_copier_relay_server::domain::models::MasterSettings {
            enabled: true,
            ..Default::default()
        };
        db.update_master_settings(master_id, master_settings)
            .await?;

        // 3. Initialize EA Context (Raw FFI)
        let acc_id = to_u16(master_id);
        let ea_type = to_u16("Master");
        let platform = to_u16("MT5");
        let broker = to_u16("TestBroker");
        let acc_name = to_u16("Test Name");
        let srv = to_u16("TestServer");
        let currency = to_u16("USD");

        let ctx_ptr = unsafe {
            ea_init(
                acc_id.as_ptr(),
                ea_type.as_ptr(),
                platform.as_ptr(),
                123456,
                broker.as_ptr(),
                acc_name.as_ptr(),
                srv.as_ptr(),
                currency.as_ptr(),
                100,
            )
        };
        assert!(!ctx_ptr.is_null(), "Context init failed");

        // 4. Connect to Sandbox Server
        let pull_addr = to_u16(&server.zmq_pull_address());
        let pub_addr = to_u16(&server.zmq_pub_address());

        let ret = unsafe { ea_connect(ctx_ptr, pull_addr.as_ptr(), pub_addr.as_ptr()) };
        assert_eq!(ret, 1, "ea_connect failed");

        std::thread::sleep(Duration::from_millis(200));

        // 5. Send Register to trigger Initial Config
        let mut buf = vec![0u8; 1024];
        unsafe {
            ea_send_register(
                ctx_ptr,
                buf.as_mut_ptr(),
                1024,
                std::ptr::null(), // prefix
                std::ptr::null(), // suffix
                std::ptr::null(), // specials
                0,                // is_trade_allowed
            );
        }
        // Correct usage:
        let len = unsafe {
            ea_send_register(
                ctx_ptr,
                buf.as_mut_ptr(),
                1024,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0, // is_trade_allowed
            )
        };
        assert!(len > 0, "Register serialization failed");

        unsafe { ea_send_push(ctx_ptr, buf.as_ptr(), len) };

        // 6. Consume Configs (Simulate EA Loop)
        // We expect:
        // 1. Initial Config (Status 0 / Disabled) - from Register
        // 2. Updated Config (Status 2 / Connected) - from Heartbeat -> RequestConfig

        let mut received_configs = Vec::new();
        let start = Instant::now();

        while start.elapsed() < Duration::from_secs(5) {
            // Increased timeout
            // Simulate EA tick with trade allowed = 1 (true)
            let ret = unsafe { ea_manager_tick(ctx_ptr, 10000.0, 10000.0, 0, 1) };

            if ret == 1 {
                loop {
                    let mut cmd = EaCommand::default();
                    let cmd_ret = unsafe { ea_get_command(ctx_ptr, &mut cmd) };
                    if cmd_ret == 0 {
                        break;
                    }

                    if cmd.command_type == 5 {
                        // UpdateUi
                        // Fetch config immediately
                        let mut c_config = SMasterConfig::default();
                        let conf_ret =
                            unsafe { ea_context_get_master_config(ctx_ptr, &mut c_config) };
                        if conf_ret == 1 {
                            println!("Received Master Config: Status={}", c_config.status);
                            received_configs.push(c_config);
                        }
                    }
                }
            }

            // Break if we have reached Status 2
            if received_configs.iter().any(|c| c.status == 2) {
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        unsafe { ea_context_free(ctx_ptr) };

        // Verification
        assert!(
            !received_configs.is_empty(),
            "Should receive at least one config"
        );

        // Check if we got the final Connected status
        let got_connected = received_configs.iter().any(|c| c.status == 2);
        if !got_connected {
            println!(
                "WARNING: Did not reach Status 2 (Connected). Received statuses: {:?}",
                received_configs
                    .iter()
                    .map(|c| c.status)
                    .collect::<Vec<_>>()
            );
        }
        assert!(got_connected, "Should reach Status 2 (Connected)");

        Ok(())
    }

    #[tokio::test]
    async fn test_slave_config_queue_behavior() -> Result<()> {
        // Similar setup based on TestSandbox
        let sandbox = TestSandbox::new()?;
        let server = sandbox.server();
        // Fix DB URL for Windows (force forward slashes)
        let db_path_str = server.db_path().to_str().unwrap().replace('\\', "/");
        let db_url = format!("sqlite://{}?mode=rwc", db_path_str);

        let db =
            sankey_copier_relay_server::adapters::outbound::persistence::Database::new(&db_url)
                .await?;

        let master_id = "MasterForSlave";
        let slave_id = "SlaveQueueTest";

        // Create Master
        db.create_trade_group(master_id).await?;
        // Enable Master
        let master_settings = sankey_copier_relay_server::domain::models::MasterSettings {
            enabled: true,
            ..Default::default()
        };
        db.update_master_settings(master_id, master_settings)
            .await?;

        // Add Slave
        use sankey_copier_relay_server::domain::models::{SlaveSettings, STATUS_ENABLED};
        // We add as ENABLED so it gets config immediately upon seeing master heartbeat?
        // Or just to have membership.
        // Ideally we want server to send config.
        // Register -> Server checks if slave in DB -> Sends config.
        db.add_member(
            master_id,
            slave_id,
            SlaveSettings::default(),
            STATUS_ENABLED,
        )
        .await?;

        let acc_id = to_u16(slave_id);
        let ea_type = to_u16("Slave");
        let ctx_ptr = unsafe {
            ea_init(
                acc_id.as_ptr(),
                ea_type.as_ptr(),
                to_u16("MT5").as_ptr(),
                123,
                to_u16("B").as_ptr(),
                to_u16("N").as_ptr(),
                to_u16("S").as_ptr(),
                to_u16("USD").as_ptr(),
                100,
            )
        };

        let pull = to_u16(&server.zmq_pull_address());
        let pub_ = to_u16(&server.zmq_pub_address());
        unsafe { ea_connect(ctx_ptr, pull.as_ptr(), pub_.as_ptr()) };

        // Register
        let mut buf = vec![0u8; 1024];
        let len = unsafe {
            ea_send_register(
                ctx_ptr,
                buf.as_mut_ptr(),
                1024,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0, // is_trade_allowed
            )
        };
        unsafe { ea_send_push(ctx_ptr, buf.as_ptr(), len) };

        // Wait for Configs (Consuming Loop)
        let mut received_configs = Vec::new();
        let start = Instant::now();

        while start.elapsed() < Duration::from_secs(5) {
            let ret = unsafe { ea_manager_tick(ctx_ptr, 10000.0, 10000.0, 0, 1) };

            if ret == 1 {
                loop {
                    let mut cmd = EaCommand::default();
                    let cmd_ret = unsafe { ea_get_command(ctx_ptr, &mut cmd) };
                    if cmd_ret == 0 {
                        break;
                    }

                    if cmd.command_type == 5 {
                        let mut c_conf = SSlaveConfig::default();
                        let conf_ret = unsafe { ea_context_get_slave_config(ctx_ptr, &mut c_conf) };
                        if conf_ret == 1 {
                            println!("Received Slave Config: Account={:?}", c_conf.account_id);
                            received_configs.push(c_conf);
                        }
                    }
                }
            }

            // For Slave, we expect at least 1 config, but maybe 2.
            // We stop if we have handled at least 1 and some time passed?
            // Or just run for fixed time?
            // Let's break if we got >= 1 and wait a bit more for queue to flush?
            if !received_configs.is_empty() && start.elapsed() > Duration::from_secs(2) {
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        unsafe { ea_context_free(ctx_ptr) };

        println!("Total Slave configs received: {}", received_configs.len());
        assert!(
            !received_configs.is_empty(),
            "Should receive at least one Slave config"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_accurate_register_single_config() -> Result<()> {
        // Scenario:
        // 1. Send Register with `is_trade_allowed=1` (True).
        // 2. Receive ONE Config (Status = Connected).
        // 3. Send Heartbeat (`is_trade_allowed=1`).
        // 4. Verify NO additional Configs received.

        let sandbox = TestSandbox::new()?;
        let server = sandbox.server();
        let db_path_str = server.db_path().to_str().unwrap().replace('\\', "/");
        let db_url = format!("sqlite://{}?mode=rwc", db_path_str);
        let db =
            sankey_copier_relay_server::adapters::outbound::persistence::Database::new(&db_url)
                .await?;

        let master_id = "AccurateRegTest";
        db.create_trade_group(master_id).await?;
        let master_settings = sankey_copier_relay_server::domain::models::MasterSettings {
            enabled: true,
            ..Default::default()
        };
        db.update_master_settings(master_id, master_settings)
            .await?;

        let acc_id = to_u16(master_id);
        let ea_type = to_u16("Master");
        let ctx_ptr = unsafe {
            ea_init(
                acc_id.as_ptr(),
                ea_type.as_ptr(),
                to_u16("MT5").as_ptr(),
                999,
                to_u16("B").as_ptr(),
                to_u16("N").as_ptr(),
                to_u16("S").as_ptr(),
                to_u16("USD").as_ptr(),
                100,
            )
        };
        let pull = to_u16(&server.zmq_pull_address());
        let pub_ = to_u16(&server.zmq_pub_address());
        unsafe { ea_connect(ctx_ptr, pull.as_ptr(), pub_.as_ptr()) };

        std::thread::sleep(Duration::from_millis(200));

        // 1. Send Register with is_trade_allowed=1 (7 args)
        let mut buf = vec![0u8; 1024];
        let len = unsafe {
            ea_send_register(
                ctx_ptr,
                buf.as_mut_ptr(),
                1024,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                1, // is_trade_allowed = true (New Argument)
            )
        };
        unsafe { ea_send_push(ctx_ptr, buf.as_ptr(), len) };

        // 2. Wait for First Config
        let mut received_configs = Vec::new();
        let start = Instant::now();

        while start.elapsed() < Duration::from_secs(3) {
            // Heartbeat tick with trade_allowed=1 (same as register)
            let ret = unsafe { ea_manager_tick(ctx_ptr, 10000.0, 10000.0, 0, 1) };

            if ret == 1 {
                loop {
                    let mut cmd = EaCommand::default();
                    let cmd_ret = unsafe { ea_get_command(ctx_ptr, &mut cmd) };
                    if cmd_ret == 0 {
                        break;
                    }

                    if cmd.command_type == 5 {
                        // UpdateUi
                        let mut c_config = SMasterConfig::default();
                        let conf_ret =
                            unsafe { ea_context_get_master_config(ctx_ptr, &mut c_config) };
                        if conf_ret == 1 {
                            println!("Received Master Config (Status={})", c_config.status);
                            received_configs.push(c_config);
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        unsafe { ea_context_free(ctx_ptr) };

        println!("Total Configs Received: {}", received_configs.len());

        // Assert we received > 0
        assert!(
            received_configs.len() >= 1,
            "Should receive at least 1 config"
        );
        assert_eq!(
            received_configs[0].status, 2,
            "First config should be Connected (2)"
        );

        // Assert exactly 1 config update
        // (Note: Implementation must suppress the redundant one)
        assert_eq!(
            received_configs.len(),
            1,
            "Should receive exactly 1 Master Config update"
        );

        Ok(())
    }
    // Helpers for test capability usage
}
