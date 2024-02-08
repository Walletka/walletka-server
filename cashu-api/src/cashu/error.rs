use std::fmt;

use axum::http::StatusCode;
use cashu_sdk::{lightning_invoice::ParseOrSemanticError, url};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Error {
    InvoiceNotPaid,
    InvoiceExpired,
    DecodeInvoice,
    StatusCode(StatusCode),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvoiceNotPaid => write!(f, "Lightning invoice not paid yet."),
            Self::InvoiceExpired => write!(f, "Lightning invoice expired."),
            Self::DecodeInvoice => write!(f, "Failed to decode LN Invoice"),
            Self::StatusCode(code) => write!(f, "{}", code),
        }
    }
}

impl From<StatusCode> for Error {
    fn from(code: StatusCode) -> Self {
        Self::StatusCode(code)
    }
}

impl From<ParseOrSemanticError> for Error {
    fn from(_err: ParseOrSemanticError) -> Self {
        Self::DecodeInvoice
    }
}

impl From<url::Error> for Error {
    fn from(_err: url::Error) -> Self {
        Self::DecodeInvoice
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    code: u16,
    error: String,
}

impl ErrorResponse {
    pub fn new(code: u16, error: &str) -> Self {
        Self {
            code,
            error: error.to_string(),
        }
    }

    pub fn as_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
