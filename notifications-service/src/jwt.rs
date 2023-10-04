use jsonwebtoken::{self, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    exp: usize,
    pub sub: String,
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

pub fn decode(token: &str) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    jsonwebtoken::decode::<Claims>(
        &token,
        &DecodingKey::from_rsa_pem(include_bytes!("../ca.crt")).unwrap(),
        &Validation::new(Algorithm::RS256),
    )
}
