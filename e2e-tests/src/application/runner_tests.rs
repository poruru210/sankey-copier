// e2e-tests/src/platform/tests.rs

use super::runner::PlatformRunner;
use crate::domain::mql_types::{
    MqlTradeRequest, MqlTradeResult, MqlTradeTransaction, ENUM_DEINIT_REASON, ENUM_INIT_RETCODE,
};
use crate::domain::traits::ExpertAdvisor;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct MockEA {
    pub events: Arc<Mutex<Vec<String>>>,
    pub thread_ids: Arc<Mutex<Vec<thread::ThreadId>>>,
}

impl ExpertAdvisor for MockEA {
    fn on_init(&mut self) -> ENUM_INIT_RETCODE {
        self.events.lock().unwrap().push("OnInit".to_string());
        self.thread_ids.lock().unwrap().push(thread::current().id());
        ENUM_INIT_RETCODE::INIT_SUCCEEDED
    }

    fn on_deinit(&mut self, reason: ENUM_DEINIT_REASON) {
        self.events
            .lock()
            .unwrap()
            .push(format!("OnDeinit({:?})", reason));
        self.thread_ids.lock().unwrap().push(thread::current().id());
    }

    fn on_tick(&mut self) {
        self.events.lock().unwrap().push("OnTick".to_string());
        self.thread_ids.lock().unwrap().push(thread::current().id());
        // Simulate blocking work
        thread::sleep(Duration::from_millis(50));
    }

    fn on_timer(&mut self) {
        self.events.lock().unwrap().push("OnTimer".to_string());
        self.thread_ids.lock().unwrap().push(thread::current().id());
    }

    fn on_trade_transaction(
        &mut self,
        _trans: &MqlTradeTransaction,
        _request: &MqlTradeRequest,
        _result: &MqlTradeResult,
    ) {
        self.events
            .lock()
            .unwrap()
            .push("OnTradeTransaction".to_string());
        self.thread_ids.lock().unwrap().push(thread::current().id());
    }
}

#[test]
fn test_platform_lifecycle_and_threading() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let thread_ids = Arc::new(Mutex::new(Vec::new()));

    let ea = MockEA {
        events: events.clone(),
        thread_ids: thread_ids.clone(),
    };

    let mut runner = PlatformRunner::new(ea);

    // Give some time for Init to process
    thread::sleep(Duration::from_millis(50));

    // Send events rapidly
    runner.send_tick();
    runner.send_timer();
    runner.send_trade_transaction(
        MqlTradeTransaction::default(),
        MqlTradeRequest::default(),
        MqlTradeResult::default(),
    );

    // Give time for processing
    thread::sleep(Duration::from_millis(200));

    runner.stop(ENUM_DEINIT_REASON::REASON_REMOVE);

    // Wait for shutdown
    thread::sleep(Duration::from_millis(50));

    let captured_events = events.lock().unwrap().clone();
    let captured_threads = thread_ids.lock().unwrap().clone();

    // Debug output
    println!("Events: {:?}", captured_events);

    // 1. Verify Init was called
    assert_eq!(captured_events[0], "OnInit");

    // 2. Verify Deinit was called at the end
    let last_event = captured_events.last().unwrap();
    assert!(last_event.starts_with("OnDeinit"));

    // 3. Verify Tick and Timer were processed
    assert!(captured_events.contains(&"OnTick".to_string()));
    assert!(captured_events.contains(&"OnTimer".to_string()));
    assert!(captured_events.contains(&"OnTradeTransaction".to_string()));

    // 4. Verify Single Thread Execution
    assert!(
        !captured_threads.is_empty(),
        "Should have captured thread IDs"
    );
    let first_id = captured_threads[0];
    for (i, id) in captured_threads.iter().enumerate() {
        assert_eq!(
            *id, first_id,
            "Event at index {} ran on a different thread!",
            i
        );
    }

    // 5. Verify it's a different thread from the test thread
    assert_ne!(first_id, thread::current().id());
}
