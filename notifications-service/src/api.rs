use event_hash::{DecryptError, HashData};
use poem::web::Data;
use poem_openapi::{
    param::Query,
    payload::{Json, PlainText},
    ApiResponse, Enum, Object, OpenApi,
};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

use crate::jwt;
use crate::store;

#[derive(Debug, Clone)]
pub struct SecretKey(pub String);

#[derive(Debug, PartialEq, Enum, Serialize, Deserialize)]
enum BodyNotificationType {
    OutOfRange,
    Stabilized,
}

#[derive(Object)]
struct NotifyBody {
    notification_type: BodyNotificationType,
    researcher: String,
    measurement_id: String,
    experiment_id: String,
    cipher_data: String,
}

impl NotifyBody {
    fn validate_body(&self, hash_data: &HashData) -> Result<(), NotifyErrorResponse> {
        if hash_data.measurement_id != self.measurement_id {
            return Err(NotifyErrorResponse::BadRequest(PlainText(format!(
                "Unexpected measurement_id `{}`. Expected `{}`",
                self.measurement_id, hash_data.measurement_id
            ))));
        } else if hash_data.experiment_id != self.experiment_id {
            return Err(NotifyErrorResponse::BadRequest(PlainText(format!(
                "Unexpected experiment_id `{}`. Expected `{}`",
                self.experiment_id, hash_data.experiment_id
            ))));
        } else if hash_data.researcher != self.researcher {
            return Err(NotifyErrorResponse::BadRequest(PlainText(format!(
                "Unexpected researcher `{}`. Expected `{}`",
                self.researcher, hash_data.researcher
            ))));
        }
        if let None = hash_data.notification_type {
            return Err(NotifyErrorResponse::BadRequest(PlainText(format!(
                "Unexpected notification. Measurement `{}` should not have been notified",
                hash_data.measurement_id
            ))));
        }

        let hash_notification = hash_data
            .notification_type
            .as_ref()
            .expect("verify whether hash_data is None beforehand");
        let hash_notification_string = serde_json::to_string(hash_notification)
            .expect("Serializable HashData NotificationType");
        let body_notification_string = serde_json::to_string(&self.notification_type)
            .expect("Serializable Body NotificationType");
        if hash_notification_string != body_notification_string {
            return Err(NotifyErrorResponse::BadRequest(PlainText(format!(
                "Unexpected notification_type `{:?}`. Expected `{:?}`",
                self.notification_type, hash_notification
            ))));
        }
        Ok(())
    }
}

#[derive(ApiResponse)]
enum NotifyResponse {
    /// Notification is successfully created
    #[oai(status = 200)]
    Ok(PlainText<String>),
}

#[derive(ApiResponse)]
enum NotifyErrorResponse {
    /// Request could not be processed
    #[oai(status = 400)]
    BadRequest(PlainText<String>),

    /// The server has encountered an error
    #[oai(status = 500)]
    InternalServerError(PlainText<String>),
}

impl From<DecryptError> for NotifyErrorResponse {
    fn from(e: DecryptError) -> Self {
        match e {
            DecryptError::MalformedHashDataString => {
                NotifyErrorResponse::BadRequest(PlainText("Invalid cipher".into()))
            }
            DecryptError::MalformedB64Nonce => {
                NotifyErrorResponse::BadRequest(PlainText("Malformed b64 encoded nonce".into()))
            }
            DecryptError::MalformedB64Ciphertext => NotifyErrorResponse::BadRequest(PlainText(
                "Malformed b64 encoded ciphertext".into(),
            )),
            DecryptError::DecryptionError => NotifyErrorResponse::BadRequest(PlainText(
                "Cipher text not encrypted with provided nonce and server key".into(),
            )),
            DecryptError::Utf8DecodingError => NotifyErrorResponse::InternalServerError(PlainText(
                "Could not decode into utf8 string".into(),
            )),
            DecryptError::JsonDeserializationError => NotifyErrorResponse::InternalServerError(
                PlainText("Could not deserialize json string into HashData.".into()),
            ),
        }
    }
}

impl From<sqlx::Error> for NotifyErrorResponse {
    fn from(e: sqlx::Error) -> Self {
        info!("sqlx error: {:?}", e);
        NotifyErrorResponse::InternalServerError(PlainText(format!("Failed to insert values")))
    }
}

impl From<jsonwebtoken::errors::Error> for NotifyErrorResponse {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        info!("jsonwebtoken error: {:?}", e);
        NotifyErrorResponse::InternalServerError(PlainText(format!(
            "Token must be encoded by ECC private key"
        )))
    }
}

fn compute_latency(ts: f64) -> f64 {
    let current_time = SystemTime::now();
    let current_time = current_time
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let current_time: f64 =
        current_time.as_secs() as f64 + current_time.subsec_nanos() as f64 / 1_000_000_000_f64;
    current_time - ts
}

pub struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/notify", method = "post")]
    async fn notify_post(
        &self,
        data_secret_key: Data<&SecretKey>,
        pool: Data<&Option<Pool<Postgres>>>,
        body: Json<NotifyBody>,
        token: Query<Option<String>>,
    ) -> Result<NotifyResponse, NotifyErrorResponse> {
        let pool = pool.0;
        let secret_key = data_secret_key.0;
        let key: &[u8] = secret_key.0.as_bytes();
        let body = body.0;

        let hash_data = HashData::decrypt(key, &body.cipher_data)?;
        body.validate_body(&hash_data)?;
        let latency = compute_latency(hash_data.timestamp);

        if let (Some(token), Some(pool)) = (token.0, pool.as_ref()) {
            let claims = jwt::decode(&token)?.claims;
            store::insert_latency(pool, &claims.sub, &body.measurement_id, &latency).await?;
        }

        info!(
            "measurement_id: {}\tlatency: {}s",
            body.measurement_id, latency
        );
        Ok(NotifyResponse::Ok(PlainText(format!("{}", latency))))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aes_gcm::aead::{AeadCore, OsRng};
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm,
        Key, // Or `Aes128Gcm`
    };
    use base64::{engine::general_purpose, Engine as _};
    use poem::{middleware::AddDataEndpoint, test::TestClient, EndpointExt, Route};
    use poem_openapi::{types::ToJSON, OpenApiService};
    use serde_json::json;

    const SECRET_KEY: &str = "QJUHsPhnA0eiqHuJqsPgzhDozYO4f1zh";

    fn message_for_comparison() -> String {
        json!({
            "notification_type": BodyNotificationType::OutOfRange,
            "researcher": "d.landau@uu.nl",
            "experiment_id": "5678",
            "measurement_id": "1234",
            "timestamp": 1693833763.2243981,
        })
        .to_json_string()
    }

    fn create_hash_data() -> HashData {
        HashData {
            notification_type: Some(event_hash::NotificationType::OutOfRange),
            researcher: "d.landau@uu.nl".into(),
            experiment_id: "5678".into(),
            measurement_id: "1234".into(),
            timestamp: 1692029115.4314,
        }
    }

    fn create_cipher_data(message: String) -> String {
        let secret_key = SecretKey(SECRET_KEY.into());
        let key: &[u8] = secret_key.0.as_bytes();
        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(&key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
        let ciphertext = cipher.encrypt(&nonce, message.as_bytes().as_ref()).unwrap();

        let b64_cipher: String = general_purpose::STANDARD_NO_PAD.encode(ciphertext);
        let b64_nonce: String = general_purpose::STANDARD_NO_PAD.encode(nonce);
        b64_nonce + "." + &b64_cipher
    }

    fn get_client(
    ) -> TestClient<AddDataEndpoint<AddDataEndpoint<Route, SecretKey>, Option<Pool<Postgres>>>>
    {
        let secret_key = SecretKey(SECRET_KEY.into());
        let api_service =
            OpenApiService::new(Api, "Hello World", "1.0").server("http://localhost:3000/api");
        let app = Route::new()
            .nest("/api", api_service)
            .data(secret_key)
            .data(None);
        TestClient::new(app)
    }

    #[tokio::test]
    async fn post_notify_valid_request() {
        let client = get_client();
        let hash_data = create_hash_data();
        let secret_key = SecretKey(SECRET_KEY.into());
        let key: &[u8] = secret_key.0.as_bytes();
        let cipher_data = hash_data.encrypt(key);
        let body = json!({
            "notification_type": "OutOfRange",
            "researcher": "d.landau@uu.nl",
            "measurement_id": "1234",
            "experiment_id": "5678",
            "cipher_data": cipher_data
        });
        let mut res = client.post("/api/notify").body_json(&body).send().await;
        println!("{:?}", res.0.take_body());
        assert_eq!(res.0.status(), 200);
    }

    #[tokio::test]
    async fn post_notify_invalid_cipher_composition() {
        let client = get_client();
        let mut res = client
            .post("/api/notify")
            .body_json(&json!({
                "notification_type": "OutOfRange",
                "researcher": "d.landau@uu.nl",
                "measurement_id": "1234",
                "experiment_id": "5678",
                "cipher_data": "R8n76xYE4v/AUk1X5hM/+kkLHH5KYdoDpKiz7dUxybXaq++DcjXcuqM4GxNFg/jbvjmTnS/rh7FKoXvjJu1sg4Gc/cELVkDJ+ZWl0HTS81AfyQQmFH/CID53T3ynTtFmYATtWCnGxWiHffo/RFVSNXdQQvb2x5YBFA4DX7mznPpaC3qzwtzGEGgYtkDkzS0cVC4Kd5gWgJwInx7SHBIoflHZvfzUi329vIU"
            }))
            .send().await;
        assert_eq!(res.0.status(), 400);
        assert_eq!(
            res.0.take_body().into_string().await.unwrap(),
            "Invalid cipher"
        );
    }

    #[tokio::test]
    async fn post_notify_invalid_b64_nonce() {
        let client = get_client();
        let mut res = client
            .post("/api/notify")
            .body_json(&json!({
                "notification_type": "OutOfRange",
                "researcher": "d.landau@uu.nl",
                "measurement_id": "1234",
                "experiment_id": "5678",
                "cipher_data": "~8n76xYE4v/AUk1X.5hM/+kkLHH5KYdoDpKiz7dUxybXaq++DcjXcuqM4GxNFg/jbvjmTnS/rh7FKoXvjJu1sg4Gc/cELVkDJ+ZWl0HTS81AfyQQmFH/CID53T3ynTtFmYATtWCnGxWiHffo/RFVSNXdQQvb2x5YBFA4DX7mznPpaC3qzwtzGEGgYtkDkzS0cVC4Kd5gWgJwInx7SHBIoflHZvfzUi329vIU"
            }))
            .send().await;
        assert_eq!(res.0.status(), 400);
        assert_eq!(
            res.0.take_body().into_string().await.unwrap(),
            "Malformed b64 encoded nonce"
        );
    }

    #[tokio::test]
    async fn post_notify_invalid_b64_ciphertext() {
        let client = get_client();
        let mut res = client
            .post("/api/notify")
            .body_json(&json!({
                "notification_type": "OutOfRange",
                "researcher": "d.landau@uu.nl",
                "measurement_id": "1234",
                "experiment_id": "5678",
                "cipher_data": "R8n76xYE4v/AUk1X.~hM/+kkLHH5KYdoDpKiz7dUxybXaq++DcjXcuqM4GxNFg/jbvjmTnS/rh7FKoXvjJu1sg4Gc/cELVkDJ+ZWl0HTS81AfyQQmFH/CID53T3ynTtFmYATtWCnGxWiHffo/RFVSNXdQQvb2x5YBFA4DX7mznPpaC3qzwtzGEGgYtkDkzS0cVC4Kd5gWgJwInx7SHBIoflHZvfzUi329vIU"
            }))
            .send().await;
        assert_eq!(res.0.status(), 400);
        assert_eq!(
            res.0.take_body().into_string().await.unwrap(),
            "Malformed b64 encoded ciphertext"
        );
    }

    #[tokio::test]
    async fn post_notify_not_encrypted_with_server_key() {
        let client = get_client();
        let mut res = client
            .post("/api/notify")
            .body_json(&json!({
                "notification_type": "OutOfRange",
                "researcher": "d.landau@uu.nl",
                "measurement_id": "1234",
                "experiment_id": "5678",
                "cipher_data": "S8n76xYE4v/AUk1X.5hM/+kkLHH5KYdoDpKiz7dUxybXaq++DcjXcuqM4GxNFg/jbvjmTnS/rh7FKoXvjJu1sg4Gc/cELVkDJ+ZWl0HTS81AfyQQmFH/CID53T3ynTtFmYATtWCnGxWiHffo/RFVSNXdQQvb2x5YBFA4DX7mznPpaC3qzwtzGEGgYtkDkzS0cVC4Kd5gWgJwInx7SHBIoflHZvfzUi329vIU"
            }))
            .send().await;
        assert_eq!(res.0.status(), 400);
        assert_eq!(
            res.0.take_body().into_string().await.unwrap(),
            "Cipher text not encrypted with provided nonce and server key"
        );
    }

    #[tokio::test]
    async fn post_notify_incompatible_measurement_id() {
        let message = message_for_comparison();
        let client = get_client();
        let json_content = json!({
            "notification_type": "OutOfRange",
            "researcher": "d.landau@uu.nl",
            "measurement_id": "234",
            "experiment_id": "5678",
            "cipher_data": create_cipher_data(message)
        });
        println!("{:?}", json_content);
        let mut res = client
            .post("/api/notify")
            .body_json(&json_content)
            .send()
            .await;

        assert_eq!(res.0.status(), 400);
        assert_eq!(
            res.0.take_body().into_string().await.unwrap(),
            "Unexpected measurement_id `234`. Expected `1234`"
        );
    }

    #[tokio::test]
    async fn post_notify_incompatible_experiment_id() {
        let message = message_for_comparison();
        let client = get_client();
        let mut res = client
            .post("/api/notify")
            .body_json(&json!({
                "notification_type": "OutOfRange",
                "researcher": "d.landau@uu.nl",
                "measurement_id": "1234",
                "experiment_id": "678",
                "cipher_data": create_cipher_data(message)
            }))
            .send()
            .await;
        assert_eq!(res.0.status(), 400);
        assert_eq!(
            res.0.take_body().into_string().await.unwrap(),
            "Unexpected experiment_id `678`. Expected `5678`"
        );
    }

    #[tokio::test]
    async fn post_notify_incompatible_researcher() {
        let message = message_for_comparison();
        let client = get_client();
        let mut res = client
            .post("/api/notify")
            .body_json(&json!({
                "notification_type": "OutOfRange",
                "researcher": "diogo.landau@uu.nl",
                "measurement_id": "1234",
                "experiment_id": "5678",
                "cipher_data": create_cipher_data(message)
            }))
            .send()
            .await;
        assert_eq!(res.0.status(), 400);
        assert_eq!(
            res.0.take_body().into_string().await.unwrap(),
            "Unexpected researcher `diogo.landau@uu.nl`. Expected `d.landau@uu.nl`"
        );
    }

    #[tokio::test]
    async fn post_notify_incompatible_notification_type() {
        let message = message_for_comparison();
        let client = get_client();
        let mut res = client
            .post("/api/notify")
            .body_json(&json!({
                "notification_type": "Stabilized",
                "researcher": "d.landau@uu.nl",
                "measurement_id": "1234",
                "experiment_id": "5678",
                "cipher_data": create_cipher_data(message)
            }))
            .send()
            .await;
        assert_eq!(res.0.status(), 400);
        assert_eq!(
            res.0.take_body().into_string().await.unwrap(),
            "Unexpected notification_type `Stabilized`. Expected `OutOfRange`"
        );
    }

    #[tokio::test]
    async fn post_notify_invalid_data_types() {
        let message = message_for_comparison();
        let client = get_client();
        let res = client
            .post("/api/notify")
            .body_json(&json!({
                "notification_type": "NotInEnum",
                "researcher": "d.landau@uu.nl",
                "measurement_id": "1234",
                "experiment_id": "678",
                "cipher_data": create_cipher_data(message)
            }))
            .send()
            .await;
        assert_eq!(res.0.status(), 400);
    }
}
