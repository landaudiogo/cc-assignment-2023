use crate::api::NotifyErrorResponse;
use event_hash::DecryptError;
use prometheus_client::{
    encoding::{EncodeLabelSet, EncodeLabelValue},
    metrics::{counter::Counter, family::Family},
};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ResponseCountLabels {
    pub group: Option<String>,
    pub response_type: ResponseType,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum ResponseType {
    Ok,
    HashError,
    InsertError,
    JwtError,
    InvalidData,
}

impl From<&DecryptError> for ResponseType {
    fn from(_e: &DecryptError) -> Self {
        ResponseType::HashError
    }
}

impl From<&sqlx::Error> for ResponseType {
    fn from(_e: &sqlx::Error) -> Self {
        ResponseType::InsertError
    }
}

impl From<&jsonwebtoken::errors::Error> for ResponseType {
    fn from(_e: &jsonwebtoken::errors::Error) -> Self {
        ResponseType::JwtError
    }
}

impl From<&NotifyErrorResponse> for ResponseType {
    fn from(_e: &NotifyErrorResponse) -> Self {
        ResponseType::InvalidData
    }
}

#[derive(Clone)]
pub struct Metrics {
    pub response_count: Family<ResponseCountLabels, Counter>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            response_count: Family::<ResponseCountLabels, Counter>::default(),
        }
    }
}
