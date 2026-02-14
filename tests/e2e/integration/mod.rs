//! Integration test utilities and helpers

pub mod fixtures {
    use std::sync::Once;
    
    static INIT: Once = Once::new();
    
    /// Initialize test environment
    pub fn init() {
        INIT.call_once(|| {
            // Initialize logging for tests
            let _ = tracing_subscriber::fmt()
                .with_test_writer()
                .try_init();
        });
    }
}

pub mod mock {
    use httpmock::MockServer;
    
    /// Create a mock server for testing
    pub fn create_mock_server() -> MockServer {
        MockServer::start()
    }
}

pub mod asserts {
    /// Assert that a response contains expected JSON fields
    pub fn assert_json_contains(response: &serde_json::Value, expected: &serde_json::Value) {
        if let serde_json::Value::Object(map) = expected {
            for (key, value) in map {
                assert_eq!(
                    response.get(key),
                    Some(value),
                    "JSON field '{}' mismatch",
                    key
                );
            }
        }
    }
}
