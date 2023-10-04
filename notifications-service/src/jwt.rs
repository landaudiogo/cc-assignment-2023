use jsonwebtoken::{self, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    exp: usize,
    sub: String,
}

impl Claims {
    pub fn new(subject: String) -> Self {
        Self {
            exp: 1704129818,
            sub: subject,
        }
    }
}

pub fn encode(claims: &Claims) -> Result<String, jsonwebtoken::errors::Error> {
    jsonwebtoken::encode(
        &Header::new(Algorithm::RS256),
        claims,
        &EncodingKey::from_rsa_pem(include_bytes!("../ca.key")).unwrap(),
    )
}
