use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};

use crate::requests::ResponseError;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct Labels {
    pub host_name: String,
    pub response_type: ResponseType,
    pub endpoint: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum ResponseType {
    Ok,
    ServerError,
    BodyDecodingError,
    DeserializationError,
    ValidationError,
}

impl From<&Result<(), ResponseError>> for ResponseType {
    fn from(response: &Result<(), ResponseError>) -> Self {
        match response {
            Ok(()) => ResponseType::Ok,
            Err(ResponseError::ServerError) => ResponseType::ServerError,
            Err(ResponseError::BodyDecodingError) => ResponseType::BodyDecodingError,
            Err(ResponseError::DeserializationError) => ResponseType::DeserializationError,
            Err(ResponseError::ValidationError) => ResponseType::ValidationError,
        }
    }
}
