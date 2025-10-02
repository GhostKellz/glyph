/// FFI C ABI Tests
///
/// These tests verify the C ABI compatibility layer for Rune (Zig) integration.
/// They ensure that the FFI surface is stable and compatible with external libraries.

use glyph::ffi::{FfiError, FfiResult, FfiVersion, strings};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use serde_json::json;

#[test]
fn test_ffi_error_codes_are_stable() {
    // Ensure error codes don't change (ABI stability)
    assert_eq!(FfiError::Success as c_int, 0);
    assert_eq!(FfiError::InvalidArgument as c_int, -1);
    assert_eq!(FfiError::OutOfMemory as c_int, -2);
    assert_eq!(FfiError::NotFound as c_int, -3);
    assert_eq!(FfiError::ExecutionFailed as c_int, -4);
    assert_eq!(FfiError::VersionMismatch as c_int, -5);
    assert_eq!(FfiError::ThreadSafetyViolation as c_int, -6);
    assert_eq!(FfiError::IoError as c_int, -7);
    assert_eq!(FfiError::PermissionDenied as c_int, -8);
    assert_eq!(FfiError::Timeout as c_int, -9);
    assert_eq!(FfiError::NotImplemented as c_int, -10);
    assert_eq!(FfiError::UnknownError as c_int, -99);
}

#[test]
fn test_ffi_error_roundtrip() {
    let errors = vec![
        FfiError::Success,
        FfiError::InvalidArgument,
        FfiError::ExecutionFailed,
        FfiError::NotImplemented,
    ];

    for err in errors {
        let code: c_int = err.into();
        let back: FfiError = code.into();
        assert_eq!(err, back);
    }
}

#[test]
fn test_cstring_null_termination() {
    let test_str = "test string";
    let cstr = strings::string_to_cstr(test_str).unwrap();

    // Verify null termination
    let bytes = cstr.as_bytes_with_nul();
    assert_eq!(bytes[bytes.len() - 1], 0);
}

#[test]
fn test_cstring_to_rust_string_conversion() {
    let original = "Hello, Rune!";
    let cstr = CString::new(original).unwrap();

    let converted = unsafe { strings::cstr_to_string(cstr.as_ptr()) }.unwrap();
    assert_eq!(converted, original);
}

#[test]
fn test_cstring_null_pointer_handling() {
    let result = unsafe { strings::cstr_to_string(std::ptr::null()) };
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), FfiError::InvalidArgument);
}

#[test]
fn test_json_serialization_roundtrip() {
    let value = json!({
        "tool": "read_file",
        "arguments": {
            "path": "/etc/hosts",
            "encoding": "utf-8"
        },
        "metadata": {
            "timestamp": 1234567890,
            "user": "test"
        }
    });

    let json_cstr = strings::value_to_json_string(&value).unwrap();
    let json_str = json_cstr.to_str().unwrap();
    let back = strings::json_string_to_value(json_str).unwrap();

    assert_eq!(back, value);
}

#[test]
fn test_json_invalid_input() {
    let invalid_json = "{invalid json}";
    let result = strings::json_string_to_value(invalid_json);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), FfiError::InvalidArgument);
}

#[test]
fn test_json_empty_object() {
    let value = json!({});
    let json_cstr = strings::value_to_json_string(&value).unwrap();
    let back = strings::json_string_to_value(json_cstr.to_str().unwrap()).unwrap();
    assert_eq!(back, value);
}

#[test]
fn test_json_nested_structures() {
    let value = json!({
        "level1": {
            "level2": {
                "level3": {
                    "value": [1, 2, 3, 4, 5]
                }
            }
        }
    });

    let json_cstr = strings::value_to_json_string(&value).unwrap();
    let back = strings::json_string_to_value(json_cstr.to_str().unwrap()).unwrap();
    assert_eq!(back, value);
}

#[test]
fn test_ffi_version_struct_layout() {
    // Ensure FfiVersion has correct C layout
    let version = FfiVersion {
        major: 0,
        minor: 1,
        patch: 0,
    };

    assert_eq!(version.major, 0);
    assert_eq!(version.minor, 1);
    assert_eq!(version.patch, 0);

    // Verify size matches expected C struct size
    assert_eq!(std::mem::size_of::<FfiVersion>(), std::mem::size_of::<c_int>() * 3);
}

#[test]
fn test_ffi_result_struct_layout() {
    // Ensure FfiResult has correct C layout
    let test_data = CString::new("test").unwrap();
    let result = FfiResult {
        error: 0,
        data: test_data.as_ptr(),
    };

    assert_eq!(result.error, 0);
    assert!(!result.data.is_null());
}

#[test]
fn test_unicode_string_handling() {
    let unicode_str = "Hello 世界 🌍 Привет";
    let cstr = strings::string_to_cstr(unicode_str).unwrap();
    let back = unsafe { strings::cstr_to_string(cstr.as_ptr()) }.unwrap();
    assert_eq!(back, unicode_str);
}

#[test]
fn test_empty_string_handling() {
    let empty = "";
    let cstr = strings::string_to_cstr(empty).unwrap();
    let back = unsafe { strings::cstr_to_string(cstr.as_ptr()) }.unwrap();
    assert_eq!(back, empty);
}

#[test]
fn test_large_json_payload() {
    // Test with larger payload similar to MCP tool responses
    let large_value = json!({
        "content": vec!["line"; 1000].join("\n"),
        "metadata": {
            "lines": 1000,
            "size_bytes": 5000,
        },
        "is_error": false
    });

    let json_cstr = strings::value_to_json_string(&large_value).unwrap();
    let back = strings::json_string_to_value(json_cstr.to_str().unwrap()).unwrap();
    assert_eq!(back, large_value);
}

#[test]
fn test_string_with_null_byte_rejection() {
    // CString should reject strings with interior null bytes
    let invalid_str = "hello\0world";
    let result = strings::string_to_cstr(invalid_str);
    assert!(result.is_err());
}

#[test]
fn test_concurrent_ffi_calls() {
    use std::thread;

    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                let value = json!({"thread": i, "data": "test"});
                let json_cstr = strings::value_to_json_string(&value).unwrap();
                let back = strings::json_string_to_value(json_cstr.to_str().unwrap()).unwrap();
                assert_eq!(back["thread"], i);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

/// Simulates a call from Zig to Rust through FFI
#[test]
fn test_simulated_zig_to_rust_call() {
    // Simulate Zig calling Rust FFI
    let tool_name = CString::new("read_file").unwrap();
    let args_json = CString::new(r#"{"path":"/etc/hosts"}"#).unwrap();

    // Rust receives these pointers
    let tool = unsafe { strings::cstr_to_string(tool_name.as_ptr()) }.unwrap();
    let args = unsafe { strings::cstr_to_string(args_json.as_ptr()) }.unwrap();
    let args_value = strings::json_string_to_value(&args).unwrap();

    assert_eq!(tool, "read_file");
    assert_eq!(args_value["path"], "/etc/hosts");
}

/// Simulates Rust returning data to Zig through FFI
#[test]
fn test_simulated_rust_to_zig_return() {
    // Rust prepares response
    let response = json!({
        "content": "file contents here",
        "is_error": false
    });

    let response_cstr = strings::value_to_json_string(&response).unwrap();

    // Zig receives this pointer
    let received = unsafe { CStr::from_ptr(response_cstr.as_ptr()) };
    let received_str = received.to_str().unwrap();
    let received_value = strings::json_string_to_value(received_str).unwrap();

    assert_eq!(received_value, response);
}

#[test]
fn test_memory_alignment() {
    // Verify C-compatible alignment
    use std::mem::align_of;

    assert!(align_of::<FfiError>() <= 4);
    assert!(align_of::<FfiVersion>() <= 4);
    assert!(align_of::<FfiResult>() <= 8);
}
