//! Error module for the orderbook: typed error with (code, message)
//!
//! - Use `ErrorType` when you know the semantic category
//! - Use `OrderBookError` as the concrete error type
//! - Format: Display -> "[{code}] {message}"
//!
//! Quick start:
//! ```rust
//! use crate::error::{Result, OrderBookError, ErrorType, make_error, lib_err};
//!
//! fn demo() -> Result<()> {
//!     // from semantic type
//!     let e1: OrderBookError = ErrorType::InvalidPrice.into();
//!     assert_eq!(e1.code, 1103);
//!     assert_eq!(e1.to_string(), "[1103] Invalid order price");
//!
//!     // from numeric code
//!     let e2 = OrderBookError::from_code(1110);
//!     assert_eq!(e2.to_string(), "[1110] Order not found");
//!
//!     // from custom message (default code 1000)
//!     let e3 = OrderBookError::from_message("something happened");
//!     assert_eq!(e3.to_string(), "[1000] something happened");
//!
//!     // utility that accepts code OR message OR ErrorType
//!     let _ = make_error(404u32);
//!     let _ = make_error("free-form message");
//!     let _ = make_error(ErrorType::OrderNotFount);
//!
//!     Ok(())
//! }
//! ```

use std::borrow::Cow;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum ErrorType {
    // 10xx General issues
    Default,

    // 11xx Request issues
    InvalidPrice,
    InvalidPriceOrQuantity,
    InvalidQuantity,
    OrderAlredyExists,
    OrderNotFount,
    
    // 12xx Internal error
    InsufficientQuantity,
    InvalidPriceLevel,
}

impl ErrorType {
    /// Numeric code for the error type.
    pub fn code(self) -> u32 {
        match self {
            // 10xx General issues
            ErrorType::Default => 1000,
        
            // 11xx Request issues
            ErrorType::InvalidQuantity => 1102,
            ErrorType::InvalidPrice => 1103,
            ErrorType::InvalidPriceOrQuantity => 1104,
            ErrorType::OrderAlredyExists => 1109,
            ErrorType::OrderNotFount => 1110,
        
            // 12xx Internal error
            ErrorType::InsufficientQuantity => 1200,
            ErrorType::InvalidPriceLevel => 1201,
        }
    }

    /// Default human message for the error type.
    pub const fn message(self) -> &'static str {
        match self {
            // 10xx General issues
            ErrorType::Default => "Something wrong",    
            // 11xx Request issues
            ErrorType::InvalidQuantity => "Invalid order quantity",
            ErrorType::InvalidPrice => "Invalid order price",
            ErrorType::InvalidPriceOrQuantity => "Invalid order price or quantity",
            ErrorType::OrderAlredyExists => "Order already exists",
            ErrorType::OrderNotFount => "Order not found",
        
            // 12xx Internal error
            ErrorType::InsufficientQuantity => "Insufficient quantity to calculate price",
            ErrorType::InvalidPriceLevel => "Invalid order price level",
        }
    }
}

/// Concrete error type carrying both code and message.
///
/// `Display` renders as `"[{code}] {message}"`.
#[derive(Debug, Error, Clone, Eq, PartialEq)]
#[error("[{code}] {message}")]
#[non_exhaustive]
pub struct OrderBookError {
    pub code: u32,
    pub message: String,
}

impl OrderBookError {
    /// Create from explicit code and message.
    #[inline]
    pub fn new(code: u32, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }

    /// Create from a known numeric code, using the standard message if known.
    #[inline]
    pub fn from_code(code: u32) -> Self {
        let msg = default_message_for_code(code);
        Self::new(code, msg)
    }

    /// Create from a free-form message, using the default code (1000).
    #[inline]
    pub fn from_message(message: impl Into<String>) -> Self {
        Self::new(ErrorType::Default.code(), message)
    }

    /// Return a new error with the same code but a different message.
    #[inline]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }
}

/// Map known numeric codes to their default messages.
/// Unknown codes get `"Unknown error ({code})"`.
#[inline]
pub fn default_message_for_code(code: u32) -> Cow<'static, str> {
    match code {
        // 10xx General issues
        1000 => Cow::Borrowed(ErrorType::Default.message()),

        // 11xx Request issues
        1102 => Cow::Borrowed(ErrorType::InvalidQuantity.message()),
        1103 => Cow::Borrowed(ErrorType::InvalidPrice.message()),
        1104 => Cow::Borrowed(ErrorType::InvalidPriceOrQuantity.message()),
        1109 => Cow::Borrowed(ErrorType::OrderAlredyExists.message()),
        1110 => Cow::Borrowed(ErrorType::OrderNotFount.message()),

        // 12xx Internal error
        1200 => Cow::Borrowed(ErrorType::InsufficientQuantity.message()),
        1201 => Cow::Borrowed(ErrorType::InvalidPriceLevel.message()),

        _ => Cow::Owned(format!("Unknown error ({code})")),
    }
}

/* ---------- Conversions & utilities ---------- */
impl From<ErrorType> for OrderBookError {
    #[inline]
    fn from(t: ErrorType) -> Self {
        Self::new(t.code(), t.message())
    }
}

/// Trait to create a `OrderBookError` from different inputs (code, message or type).
pub trait IntoOrderBookError {
    fn into_error(self) -> OrderBookError;
}

impl IntoOrderBookError for ErrorType {
    #[inline]
    fn into_error(self) -> OrderBookError {
        self.into()
    }
}

impl IntoOrderBookError for u32 {
    #[inline]
    fn into_error(self) -> OrderBookError {
        OrderBookError::from_code(self)
    }
}

impl IntoOrderBookError for &str {
    #[inline]
    fn into_error(self) -> OrderBookError {
        OrderBookError::from_message(self)
    }
}

impl IntoOrderBookError for String {
    #[inline]
    fn into_error(self) -> OrderBookError {
        OrderBookError::from_message(self)
    }
}

/// One-stop utility: accepts either a code (`u32`), a message (`&str`/`String`) or an `ErrorType`.
#[inline]
pub fn make_error<E: IntoOrderBookError>(e: E) -> OrderBookError {
    e.into_error()
}

/// Result alias for the library.
pub type Result<T> = std::result::Result<T, OrderBookError>;
