//! Rust FFI bindings for Rune (Zig MCP Tools)
//!
//! This module provides safe Rust wrappers around the Rune C ABI,
//! enabling high-performance MCP tool execution from the Rust Glyph server.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use serde_json::Value;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuneError {
    Success = 0,
    InvalidArgument = -1,
    OutOfMemory = -2,
    ToolNotFound = -3,
    ExecutionFailed = -4,
    VersionMismatch = -5,
    ThreadSafetyViolation = -6,
    IoError = -7,
    PermissionDenied = -8,
    Timeout = -9,
    UnknownError = -99,
}

impl From<c_int> for RuneError {
    fn from(value: c_int) -> Self {
        match value {
            0 => RuneError::Success,
            -1 => RuneError::InvalidArgument,
            -2 => RuneError::OutOfMemory,
            -3 => RuneError::ToolNotFound,
            -4 => RuneError::ExecutionFailed,
            -5 => RuneError::VersionMismatch,
            -6 => RuneError::ThreadSafetyViolation,
            -7 => RuneError::IoError,
            -8 => RuneError::PermissionDenied,
            -9 => RuneError::Timeout,
            _ => RuneError::UnknownError,
        }
    }
}

#[repr(C)]
pub struct RuneHandle {
    _private: [u8; 0],
}

#[repr(C)]
pub struct RuneResultHandle {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RuneResult {
    pub success: bool,
    pub error_code: RuneError,
    pub data: *const c_char,
    pub data_len: usize,
    pub error_message: *const c_char,
    pub error_len: usize,
}

#[repr(C)]
pub struct RuneVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[repr(C)]
pub struct RuneToolInfo {
    pub name: *const c_char,
    pub name_len: usize,
    pub description: *const c_char,
    pub description_len: usize,
}

// External function declarations
unsafe extern "C" {
    fn rune_init() -> *mut RuneHandle;
    fn rune_cleanup(handle: *mut RuneHandle);
    fn rune_get_version() -> RuneVersion;

    fn rune_register_tool(
        handle: *mut RuneHandle,
        name: *const c_char,
        name_len: usize,
        description: *const c_char,
        description_len: usize,
    ) -> c_int;

    fn rune_get_tool_count(handle: *mut RuneHandle) -> usize;

    fn rune_get_tool_info(
        handle: *mut RuneHandle,
        index: usize,
        out_info: *mut RuneToolInfo,
    ) -> c_int;

    fn rune_execute_tool(
        handle: *mut RuneHandle,
        name: *const c_char,
        name_len: usize,
        params_json: *const c_char,
        params_len: usize,
    ) -> *mut RuneResultHandle;

    fn rune_free_result(handle: *mut RuneResultHandle);
}

/// Safe Rust wrapper around the Rune engine
pub struct Rune {
    handle: *mut RuneHandle,
}

impl Rune {
    /// Initialize a new Rune engine instance
    pub fn new() -> Result<Self, RuneError> {
        let handle = unsafe { rune_init() };
        if handle.is_null() {
            return Err(RuneError::OutOfMemory);
        }

        Ok(Rune { handle })
    }

    /// Get the version of the Rune library
    pub fn version() -> (u32, u32, u32) {
        let version = unsafe { rune_get_version() };
        (version.major, version.minor, version.patch)
    }

    /// Register a tool with the Rune engine
    pub fn register_tool(&mut self, name: &str, description: Option<&str>) -> Result<(), RuneError> {
        let name_cstr = CString::new(name).map_err(|_| RuneError::InvalidArgument)?;
        let desc_cstr = description.map(|d| CString::new(d)).transpose()
            .map_err(|_| RuneError::InvalidArgument)?;

        let (desc_ptr, desc_len) = if let Some(ref desc) = desc_cstr {
            (desc.as_ptr(), desc.as_bytes().len())
        } else {
            (ptr::null(), 0)
        };

        let result = unsafe {
            rune_register_tool(
                self.handle,
                name_cstr.as_ptr(),
                name_cstr.as_bytes().len(),
                desc_ptr,
                desc_len,
            )
        };

        let error = RuneError::from(result);
        if error != RuneError::Success {
            return Err(error);
        }

        Ok(())
    }

    /// Get the number of registered tools
    pub fn tool_count(&self) -> usize {
        unsafe { rune_get_tool_count(self.handle) }
    }

    /// Get information about a tool by index
    pub fn tool_info(&self, index: usize) -> Result<(String, Option<String>), RuneError> {
        let mut info = RuneToolInfo {
            name: ptr::null(),
            name_len: 0,
            description: ptr::null(),
            description_len: 0,
        };

        let result = unsafe { rune_get_tool_info(self.handle, index, &mut info) };
        let error = RuneError::from(result);
        if error != RuneError::Success {
            return Err(error);
        }

        let name = if !info.name.is_null() && info.name_len > 0 {
            let slice = unsafe { std::slice::from_raw_parts(info.name as *const u8, info.name_len) };
            String::from_utf8_lossy(slice).into_owned()
        } else {
            return Err(RuneError::InvalidArgument);
        };

        let description = if !info.description.is_null() && info.description_len > 0 {
            let slice = unsafe { std::slice::from_raw_parts(info.description as *const u8, info.description_len) };
            Some(String::from_utf8_lossy(slice).into_owned())
        } else {
            None
        };

        Ok((name, description))
    }

    /// Execute a tool with JSON parameters
    pub fn execute_tool(&self, name: &str, params: &Value) -> Result<Value, RuneError> {
        let name_cstr = CString::new(name).map_err(|_| RuneError::InvalidArgument)?;
        let params_str = params.to_string();
        let params_cstr = CString::new(params_str).map_err(|_| RuneError::InvalidArgument)?;

        let result_handle = unsafe {
            rune_execute_tool(
                self.handle,
                name_cstr.as_ptr(),
                name_cstr.as_bytes().len(),
                params_cstr.as_ptr(),
                params_cstr.as_bytes().len(),
            )
        };

        if result_handle.is_null() {
            return Err(RuneError::ExecutionFailed);
        }

        // Convert the result to a safe Rust value
        let result = RuneExecutionResult::from_handle(result_handle)?;
        result.into_json()
    }
}

impl Drop for Rune {
    fn drop(&mut self) {
        unsafe {
            rune_cleanup(self.handle);
        }
    }
}

/// Safe wrapper around Rune execution results
pub struct RuneExecutionResult {
    handle: *mut RuneResultHandle,
    result: RuneResult,
}

impl RuneExecutionResult {
    /// Create a result wrapper from a handle
    fn from_handle(handle: *mut RuneResultHandle) -> Result<Self, RuneError> {
        if handle.is_null() {
            return Err(RuneError::InvalidArgument);
        }

        // Cast handle to result to read the data
        let result = unsafe { *(handle as *const RuneResult) };

        Ok(RuneExecutionResult { handle, result })
    }

    /// Convert the result to a JSON value
    pub fn into_json(self) -> Result<Value, RuneError> {
        if self.result.success {
            if !self.result.data.is_null() && self.result.data_len > 0 {
                let slice = unsafe {
                    std::slice::from_raw_parts(self.result.data as *const u8, self.result.data_len)
                };
                let json_str = std::str::from_utf8(slice)
                    .map_err(|_| RuneError::ExecutionFailed)?;
                serde_json::from_str(json_str)
                    .map_err(|_| RuneError::ExecutionFailed)
            } else {
                Ok(Value::Null)
            }
        } else {
            Err(self.result.error_code)
        }
    }

    /// Get the error message if the result failed
    pub fn error_message(&self) -> Option<String> {
        if !self.result.success && !self.result.error_message.is_null() && self.result.error_len > 0 {
            let slice = unsafe {
                std::slice::from_raw_parts(self.result.error_message as *const u8, self.result.error_len)
            };
            Some(String::from_utf8_lossy(slice).into_owned())
        } else {
            None
        }
    }
}

impl Drop for RuneExecutionResult {
    fn drop(&mut self) {
        unsafe {
            rune_free_result(self.handle);
        }
    }
}

unsafe impl Send for Rune {}
unsafe impl Sync for Rune {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rune_version() {
        let (major, minor, patch) = Rune::version();
        assert_eq!(major, 0);
        assert_eq!(minor, 1);
        assert_eq!(patch, 0);
    }

    #[test]
    fn test_rune_init() {
        let rune = Rune::new().expect("Failed to initialize Rune");
        // Rune should initialize successfully
        assert_eq!(rune.tool_count(), 0); // No tools registered initially
    }
}