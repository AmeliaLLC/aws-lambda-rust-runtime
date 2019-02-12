//! This module defines the `RuntimeApiError` trait that developers should implement
//! to send their custom errors to the AWS Lambda Runtime Client SDK. The module also
//! defines the `ApiError` type returned by the `RuntimeClient` implementations.
use std::{env, error::Error, fmt, io, num::ParseIntError, option::Option};

use backtrace;
use http::{header::ToStrError, uri::InvalidUri};
use hyper;
use serde_derive::Serialize;
use serde_json;

/// Error type description for the `ErrorResponse` event. This type should be returned
/// for errors that were handled by the function code or framework.
pub const ERROR_TYPE_HANDLED: &str = "Handled";
/// Error type description for the `ErrorResponse` event. This type is used for unhandled,
/// unexpcted errors.
pub const ERROR_TYPE_UNHANDLED: &str = "Unhandled";

/// This object is used to generate requests to the Lambda Runtime APIs.
/// It is used for both the error response APIs and fail init calls.
/// custom error types should implement the `RuntimeError` trait and return
/// this object to be compatible with the APIs.
#[derive(Serialize)]
pub struct ErrorResponse {
    /// The error message generated by the application.
    #[serde(rename = "errorMessage")]
    pub error_message: String,
    /// The error type for Lambda. This can be `Handled` or `Unhandled`.
    /// Developers can use the `ERROR_TYPE_HANDLED` and `ERROR_TYPE_UNHANDLED`
    /// constants to populate this field.
    #[serde(rename = "errorType")]
    pub error_type: String,
    /// The stack trace for the exception as vector of strings. In the framework,
    /// this value is automatically populated using the `backtrace` crate.
    #[serde(rename = "stackTrace")]
    pub stack_trace: Option<Vec<String>>,
}

impl ErrorResponse {
    /// Creates a new `RuntimeError` object with the handled error type.
    ///
    /// # Arguments
    ///
    /// * `message` The error message for the Lambda Runtime APIs.
    ///
    /// # Return
    /// A populated `RuntimeError` object that can be used with the Lambda Runtime API.
    pub fn handled(message: String) -> ErrorResponse {
        ErrorResponse {
            error_message: message,
            error_type: String::from(ERROR_TYPE_HANDLED),
            stack_trace: Option::default(),
        }
    }

    /// Creates a new `RuntimeError` object with the unhandled error type.
    ///
    /// # Arguments
    ///
    /// * `message` The error message for the Lambda Runtime APIs.
    ///
    /// # Return
    /// A populated `RuntimeError` object that can be used with the Lambda Runtime API.
    pub fn unhandled(message: String) -> ErrorResponse {
        ErrorResponse {
            error_message: message,
            error_type: String::from(ERROR_TYPE_UNHANDLED),
            stack_trace: Option::default(),
        }
    }
}

/// Custom errors for the framework should implement this trait. The client calls
/// the `to_response()` method automatically to produce an object that can be serialized
/// and sent to the Lambda Runtime APIs.
pub trait RuntimeApiError {
    /// Creates a `RuntimeError` object for the current error. This is
    /// then serialized and sent to the Lambda runtime APIs.
    ///
    /// # Returns
    /// A populated `RuntimeError` object.
    fn to_response(&self) -> ErrorResponse;
}

/// Represents an error generated by the Lambda Runtime API client.
#[derive(Debug, Clone)]
pub struct ApiError {
    msg: String,
    /// The `Backtrace` object from the `backtrace` crate used to store
    /// the stack trace of the error.
    pub backtrace: Option<backtrace::Backtrace>,
    /// Whether the current error is recoverable. If the error is not
    /// recoverable a runtime should panic to force the Lambda service
    /// to restart the execution environment.
    pub recoverable: bool,
}

impl ApiError {
    pub(crate) fn new(description: &str) -> ApiError {
        let mut trace: Option<backtrace::Backtrace> = None;
        let is_backtrace = env::var("RUST_BACKTRACE");
        if is_backtrace.is_ok() && is_backtrace.unwrap() == "1" {
            trace!("Begin backtrace collection");
            trace = Option::from(backtrace::Backtrace::new());
            trace!("Completed backtrace collection");
        }
        ApiError {
            msg: String::from(description),
            backtrace: trace,
            recoverable: true,
        }
    }

    pub(crate) fn unrecoverable(&mut self) -> &ApiError {
        self.recoverable = false;

        self
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

// This is important for other errors to wrap this one.
impl Error for ApiError {
    fn description(&self) -> &str {
        &self.msg
    }

    fn cause(&self) -> Option<&dyn Error> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::new(e.description())
    }
}

impl From<InvalidUri> for ApiError {
    fn from(e: InvalidUri) -> Self {
        ApiError::new(e.description())
    }
}

impl From<hyper::Error> for ApiError {
    fn from(e: hyper::Error) -> Self {
        ApiError::new(e.description())
    }
}

impl From<ToStrError> for ApiError {
    fn from(e: ToStrError) -> Self {
        ApiError::new(e.description())
    }
}

impl From<ParseIntError> for ApiError {
    fn from(e: ParseIntError) -> Self {
        ApiError::new(e.description())
    }
}

impl From<io::Error> for ApiError {
    fn from(e: io::Error) -> Self {
        ApiError::new(e.description())
    }
}

impl RuntimeApiError for ApiError {
    fn to_response(&self) -> ErrorResponse {
        let backtrace = format!("{:?}", self.backtrace);
        let trace_vec = backtrace
            .lines()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let mut err = ErrorResponse::unhandled(self.msg.clone());
        err.stack_trace = Option::from(trace_vec);

        err
    }
}
