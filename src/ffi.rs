//! Generic FFI Interface for Language Integrations
//!
//! This module provides a generic FFI interface that can be extended
//! to support high-performance integrations with other languages like Zig, C++, etc.
//!
//! Currently provides the foundation for future language integrations.
//! Specific implementations (like Rune) can be added as separate modules.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use serde_json::Value;

/// Generic FFI error codes that can be used across different language integrations
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FfiError {
    Success = 0,
    InvalidArgument = -1,
    OutOfMemory = -2,
    NotFound = -3,
    ExecutionFailed = -4,
    VersionMismatch = -5,
    ThreadSafetyViolation = -6,
    IoError = -7,
    PermissionDenied = -8,
    Timeout = -9,
    NotImplemented = -10,
    UnknownError = -99,
}

impl From<c_int> for FfiError {
    fn from(value: c_int) -> Self {
        match value {
            0 => FfiError::Success,
            -1 => FfiError::InvalidArgument,
            -2 => FfiError::OutOfMemory,
            -3 => FfiError::NotFound,
            -4 => FfiError::ExecutionFailed,
            -5 => FfiError::VersionMismatch,
            -6 => FfiError::ThreadSafetyViolation,
            -7 => FfiError::IoError,
            -8 => FfiError::PermissionDenied,
            -9 => FfiError::Timeout,
            -10 => FfiError::NotImplemented,
            _ => FfiError::UnknownError,
        }
    }
}

impl From<FfiError> for c_int {
    fn from(error: FfiError) -> Self {
        error as c_int
    }
}

/// Generic result type for FFI operations
#[repr(C)]
pub struct FfiResult {
    pub error: c_int,
    pub data: *const c_char,
}

/// Opaque handle type for FFI integrations
#[repr(C)]
pub struct FfiHandle {
    _private: [u8; 0],
}

/// Version information for FFI compatibility
#[repr(C)]
pub struct FfiVersion {
    pub major: c_int,
    pub minor: c_int,
    pub patch: c_int,
}

/// Generic FFI interface that language integrations can implement
pub trait LanguageIntegration {
    /// Get version information
    fn version() -> FfiVersion;

    /// Initialize the integration
    fn init() -> Result<Box<Self>, FfiError>;

    /// Clean up resources
    fn cleanup(&mut self);

    /// Execute a tool/function
    fn execute(&self, name: &str, args: &Value) -> Result<Value, FfiError>;

    /// Check if a tool/function is available
    fn has_tool(&self, name: &str) -> bool;

    /// List available tools/functions
    fn list_tools(&self) -> Vec<String>;
}

/// Helper functions for FFI string handling
pub mod strings {
    use super::*;

    /// Convert a C string to a Rust string
    pub unsafe fn cstr_to_string(cstr: *const c_char) -> Result<String, FfiError> {
        if cstr.is_null() {
            return Err(FfiError::InvalidArgument);
        }

        unsafe { CStr::from_ptr(cstr) }
            .to_str()
            .map(|s| s.to_string())
            .map_err(|_| FfiError::InvalidArgument)
    }

    /// Convert a Rust string to a C string (caller must free)
    pub fn string_to_cstr(s: &str) -> Result<CString, FfiError> {
        CString::new(s).map_err(|_| FfiError::InvalidArgument)
    }

    /// Create a JSON string from a Value (caller must free)
    pub fn value_to_json_string(value: &Value) -> Result<CString, FfiError> {
        serde_json::to_string(value)
            .map_err(|_| FfiError::ExecutionFailed)
            .and_then(|s| string_to_cstr(&s))
    }

    /// Parse a JSON string to a Value
    pub fn json_string_to_value(json: &str) -> Result<Value, FfiError> {
        serde_json::from_str(json).map_err(|_| FfiError::InvalidArgument)
    }
}

/// Placeholder for future Zig integration
/// This can be implemented when a working Zig library is available
pub mod zig {
    use super::*;

    pub struct ZigIntegration {
        // Placeholder - implement when Zig library is ready
    }

    impl LanguageIntegration for ZigIntegration {
        fn version() -> FfiVersion {
            FfiVersion { major: 0, minor: 1, patch: 0 }
        }

        fn init() -> Result<Box<Self>, FfiError> {
            Err(FfiError::NotImplemented)
        }

        fn cleanup(&mut self) {}

        fn execute(&self, _name: &str, _args: &Value) -> Result<Value, FfiError> {
            Err(FfiError::NotImplemented)
        }

        fn has_tool(&self, _name: &str) -> bool {
            false
        }

        fn list_tools(&self) -> Vec<String> {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_error_conversion() {
        assert_eq!(FfiError::from(0), FfiError::Success);
        assert_eq!(FfiError::from(-1), FfiError::InvalidArgument);
        assert_eq!(c_int::from(FfiError::Success), 0);
    }

    #[test]
    fn test_string_helpers() {
        let test_str = "hello world";
        let cstr = strings::string_to_cstr(test_str).unwrap();
        let back = unsafe { strings::cstr_to_string(cstr.as_ptr()) }.unwrap();
        assert_eq!(back, test_str);
    }

    #[test]
    fn test_json_helpers() {
        let value = serde_json::json!({"key": "value"});
        let json_str = strings::value_to_json_string(&value).unwrap();
        let back = strings::json_string_to_value(json_str.to_str().unwrap()).unwrap();
        assert_eq!(back, value);
    }
}