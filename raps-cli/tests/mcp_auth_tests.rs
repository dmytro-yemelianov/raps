//! Integration tests for MCP authentication guidance
//!
//! Tests the auth_guidance module functions and constants.

use raps_cli::mcp::auth_guidance::{
    AuthRequirement, AuthState, MISSING_CLIENT_ID, MISSING_CLIENT_SECRET, SETUP_INSTRUCTIONS,
    THREE_LEGGED_PROMPT, format_error_guidance, get_tool_auth_requirement,
    get_tool_availability_summary,
};

// ==================== AuthState Tests ====================

#[test]
fn test_auth_state_no_credentials() {
    let state = AuthState {
        has_client_id: false,
        has_client_secret: false,
        two_legged_valid: false,
        three_legged_valid: false,
        three_legged_expired: false,
    };

    assert!(!state.has_any_credentials());
    assert!(!state.has_complete_2leg_credentials());
}

#[test]
fn test_auth_state_partial_credentials_id_only() {
    let state = AuthState {
        has_client_id: true,
        has_client_secret: false,
        two_legged_valid: false,
        three_legged_valid: false,
        three_legged_expired: false,
    };

    assert!(state.has_any_credentials());
    assert!(!state.has_complete_2leg_credentials());
}

#[test]
fn test_auth_state_partial_credentials_secret_only() {
    let state = AuthState {
        has_client_id: false,
        has_client_secret: true,
        two_legged_valid: false,
        three_legged_valid: false,
        three_legged_expired: false,
    };

    assert!(state.has_any_credentials());
    assert!(!state.has_complete_2leg_credentials());
}

#[test]
fn test_auth_state_complete_2leg_credentials() {
    let state = AuthState {
        has_client_id: true,
        has_client_secret: true,
        two_legged_valid: true,
        three_legged_valid: false,
        three_legged_expired: false,
    };

    assert!(state.has_any_credentials());
    assert!(state.has_complete_2leg_credentials());
}

#[test]
fn test_auth_state_full_auth() {
    let state = AuthState {
        has_client_id: true,
        has_client_secret: true,
        two_legged_valid: true,
        three_legged_valid: true,
        three_legged_expired: false,
    };

    assert!(state.has_any_credentials());
    assert!(state.has_complete_2leg_credentials());
}

// ==================== AuthRequirement Tests ====================

#[test]
fn test_auth_requirement_auth_tools() {
    assert_eq!(
        get_tool_auth_requirement("auth_test"),
        AuthRequirement::Either
    );
    assert_eq!(
        get_tool_auth_requirement("auth_status"),
        AuthRequirement::Either
    );
    assert_eq!(
        get_tool_auth_requirement("auth_login"),
        AuthRequirement::Either
    );
    assert_eq!(
        get_tool_auth_requirement("auth_logout"),
        AuthRequirement::Either
    );
}

#[test]
fn test_auth_requirement_oss_tools() {
    assert_eq!(
        get_tool_auth_requirement("bucket_list"),
        AuthRequirement::TwoLegged
    );
    assert_eq!(
        get_tool_auth_requirement("bucket_create"),
        AuthRequirement::TwoLegged
    );
    assert_eq!(
        get_tool_auth_requirement("object_list"),
        AuthRequirement::TwoLegged
    );
    assert_eq!(
        get_tool_auth_requirement("object_delete"),
        AuthRequirement::TwoLegged
    );
}

#[test]
fn test_auth_requirement_derivative_tools() {
    assert_eq!(
        get_tool_auth_requirement("translate_start"),
        AuthRequirement::TwoLegged
    );
    assert_eq!(
        get_tool_auth_requirement("translate_status"),
        AuthRequirement::TwoLegged
    );
}

#[test]
fn test_auth_requirement_dm_tools() {
    assert_eq!(
        get_tool_auth_requirement("hub_list"),
        AuthRequirement::ThreeLegged
    );
    assert_eq!(
        get_tool_auth_requirement("project_list"),
        AuthRequirement::ThreeLegged
    );
    assert_eq!(
        get_tool_auth_requirement("folder_list"),
        AuthRequirement::ThreeLegged
    );
    assert_eq!(
        get_tool_auth_requirement("item_info"),
        AuthRequirement::ThreeLegged
    );
}

#[test]
fn test_auth_requirement_acc_tools() {
    assert_eq!(
        get_tool_auth_requirement("issue_list"),
        AuthRequirement::ThreeLegged
    );
    assert_eq!(
        get_tool_auth_requirement("rfi_list"),
        AuthRequirement::ThreeLegged
    );
    assert_eq!(
        get_tool_auth_requirement("acc_assets_list"),
        AuthRequirement::ThreeLegged
    );
}

#[test]
fn test_auth_requirement_admin_tools() {
    assert_eq!(
        get_tool_auth_requirement("admin_project_list"),
        AuthRequirement::TwoLegged
    );
    assert_eq!(
        get_tool_auth_requirement("admin_user_add"),
        AuthRequirement::TwoLegged
    );
}

#[test]
fn test_auth_requirement_unknown_tool() {
    // Unknown tools default to 2-legged
    assert_eq!(
        get_tool_auth_requirement("unknown_tool"),
        AuthRequirement::TwoLegged
    );
}

// ==================== Tool Availability Summary Tests ====================

#[test]
fn test_tool_availability_no_auth() {
    let state = AuthState {
        has_client_id: false,
        has_client_secret: false,
        two_legged_valid: false,
        three_legged_valid: false,
        three_legged_expired: false,
    };

    let summary = get_tool_availability_summary(&state);

    insta::assert_snapshot!(summary);
}

#[test]
fn test_tool_availability_2leg_only() {
    let state = AuthState {
        has_client_id: true,
        has_client_secret: true,
        two_legged_valid: true,
        three_legged_valid: false,
        three_legged_expired: false,
    };

    let summary = get_tool_availability_summary(&state);

    insta::assert_snapshot!(summary);
}

#[test]
fn test_tool_availability_full_auth() {
    let state = AuthState {
        has_client_id: true,
        has_client_secret: true,
        two_legged_valid: true,
        three_legged_valid: true,
        three_legged_expired: false,
    };

    let summary = get_tool_availability_summary(&state);

    insta::assert_snapshot!(summary);
}

// ==================== Error Guidance Tests ====================

#[test]
fn test_error_guidance_missing_client_id() {
    let guidance = format_error_guidance("Missing APS_CLIENT_ID environment variable");

    insta::assert_snapshot!(guidance);
}

#[test]
fn test_error_guidance_missing_client_secret() {
    let guidance = format_error_guidance("Missing APS_CLIENT_SECRET");

    insta::assert_snapshot!(guidance);
}

#[test]
fn test_error_guidance_unauthorized() {
    let guidance = format_error_guidance("HTTP 401 Unauthorized");

    insta::assert_snapshot!(guidance);
}

#[test]
fn test_error_guidance_network_error() {
    let guidance = format_error_guidance("Connection timeout");

    insta::assert_snapshot!(guidance);
}

#[test]
fn test_error_guidance_expired_token() {
    let guidance = format_error_guidance("Token expired");

    insta::assert_snapshot!(guidance);
}

#[test]
fn test_error_guidance_generic_error() {
    let guidance = format_error_guidance("Some unknown error occurred");

    insta::assert_snapshot!(guidance);
}

// ==================== Constant Content Tests ====================

#[test]
fn test_setup_instructions_content() {
    insta::assert_snapshot!(SETUP_INSTRUCTIONS);
}

#[test]
fn test_missing_client_id_content() {
    assert!(MISSING_CLIENT_ID.contains("APS_CLIENT_ID"));
    assert!(MISSING_CLIENT_ID.contains("aps.autodesk.com"));
}

#[test]
fn test_missing_client_secret_content() {
    assert!(MISSING_CLIENT_SECRET.contains("APS_CLIENT_SECRET"));
    assert!(MISSING_CLIENT_SECRET.contains("secure"));
}

#[test]
fn test_three_legged_prompt_content() {
    assert!(THREE_LEGGED_PROMPT.contains("auth_login"));
    assert!(THREE_LEGGED_PROMPT.contains("raps auth login"));
    assert!(THREE_LEGGED_PROMPT.contains("Autodesk"));
}
