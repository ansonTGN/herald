// =============================================================================
// Test Constants Module
// =============================================================================
//
// Centralized test constants to reduce hardcoded string duplication
// across test files.
//
// **Usage**:
// ```rust
// use crate::tests::helpers::test_constants::*;
//
// // Use constants instead of hardcoded strings
// let realm_id = TEST_REALM;
// let email = generate_test_email("user", 1);
// ```
//
// **Benefits**:
// - Single source of truth for test values
// - Easy to update test data across all tests
// - Reduces typos and inconsistencies
//
// =============================================================================

/// Default test realm ID used across most tests
pub const TEST_REALM: &str = "test-realm";

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a unique test email address
///
/// Useful when you need multiple test users with unique emails.
///
/// # Arguments
/// * `prefix` - Email prefix (e.g., "user")
/// * `index` - Index to make email unique
///
/// # Example
/// ```rust
/// let email1 = generate_test_email("user", 1); // user1@example.com
/// let email2 = generate_test_email("user", 2); // user2@example.com
/// ```
pub fn generate_test_email(prefix: &str, index: u32) -> String {
    format!("{}{}@example.com", prefix, index)
}

/// Generate a unique test realm ID
///
/// # Arguments
/// * `index` - Index to make realm ID unique
///
/// # Example
/// ```rust
/// let realm1 = generate_test_realm(1); // test-realm-1
/// let realm2 = generate_test_realm(2); // test-realm-2
/// ```
pub fn generate_test_realm(index: u32) -> String {
    format!("{}-{}", TEST_REALM, index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_test_email() {
        assert_eq!(
            generate_test_email("user", 1),
            "user1@example.com".to_string()
        );
        assert_eq!(
            generate_test_email("admin", 42),
            "admin42@example.com".to_string()
        );
    }

    #[test]
    fn test_generate_test_realm() {
        assert_eq!(generate_test_realm(1), "test-realm-1".to_string());
        assert_eq!(generate_test_realm(99), "test-realm-99".to_string());
    }
}
