use std::time::{SystemTime, UNIX_EPOCH};

use poem::{
    listener::TcpListener, 
    web::Data, 
    Route, EndpointExt
};
use poem_openapi::{
    payload::{Json, PlainText}, 
    types::ToJSON,
    OpenApi, OpenApiService, Object, Enum, ApiResponse
};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key // Or `Aes128Gcm`
};

use serde_json::{Value, json};
use serde::{Serialize, Deserialize};
use tracing::{Level, info};
use clap::Parser;
use base64::{Engine as _, engine::general_purpose};
use generic_array::{GenericArray, ArrayLength};


#[derive(Parser, Debug)]
struct CliArgs {
    #[arg(short, long)]
    secret_key: String,
}

#[derive(Debug, Clone)]
struct SecretKey(String);

#[derive(Debug, PartialEq, Enum, Serialize, Deserialize)]
enum NotificationType {
    OutOfRange, 
    Stabilized
}

#[derive(Object)]
struct NotifyBody {
    notification_type: NotificationType,
    researcher: String,
    measurement_id: String,
    experiment_id: String,
    cipher_data: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct HashData {
    notification_type: NotificationType,
    researcher: String,
    experiment_id: String, 
    measurement_id: String,  
    timestamp: f64,
}

#[derive(ApiResponse)]
enum NotifyResponse {
    /// Notification is successfully created
    #[oai(status = 200)]
    Ok,
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


struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/notify", method = "post")]
    async fn notify_post(&self, data_secret_key: Data<&SecretKey>, body: Json<NotifyBody>) -> Result<NotifyResponse, NotifyErrorResponse> {
        let secret_key = data_secret_key.0;

        let cipher_components: Vec<_> = body.0.cipher_data.split(".").collect();
        if cipher_components.len() != 2 {
            return Err(NotifyErrorResponse::BadRequest(PlainText(String::from("Invalid cipher."))))
        }

        let key: &[u8] = secret_key.0.as_bytes();
        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(&key);
        let nonce = general_purpose::STANDARD_NO_PAD.decode(cipher_components[0])
            .map_err(|_| { 
                NotifyErrorResponse::BadRequest(PlainText("Malformed b64 encoded nonce.".into())) 
            })?;
        let nonce = GenericArray::clone_from_slice(&nonce[..]);
        let ciphertext = general_purpose::STANDARD_NO_PAD.decode(cipher_components[1])
            .map_err(|_| { 
                NotifyErrorResponse::BadRequest(PlainText("Malformed b64 encoded ciphertext".into())) 
            })?;
        let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref())
            .map_err(|_| { 
                NotifyErrorResponse::BadRequest(
                    PlainText("Cipher text not encrypted with provided nonce and server key".into())
                ) 
            })?;
        let plaintext = String::from_utf8(plaintext)
            .map_err(|_| {
                NotifyErrorResponse::InternalServerError(
                    PlainText("Could not decode into utf8 string.".into())
                ) 
            })?;
        
        let hash_data: HashData = serde_json::from_str(&plaintext)
            .map_err(|_|{
                NotifyErrorResponse::InternalServerError(
                    PlainText("Could not deserialize json string into HashData.".into())
                ) 
            })?;

        // validate contents passed in body with the contents in the ciphertext
        if hash_data.measurement_id != body.measurement_id {
            return Err(NotifyErrorResponse::BadRequest(PlainText(
                format!("Unexpected measurement_id `{}`. Expected `{}`",
                        body.measurement_id, hash_data.measurement_id)
            )))
        } else if hash_data.experiment_id != body.experiment_id {
            return Err(NotifyErrorResponse::BadRequest(PlainText(
                format!("Unexpected experiment_id `{}`. Expected `{}`",
                        body.experiment_id, hash_data.experiment_id)
            )))
        } else if hash_data.researcher != body.researcher {
            return Err(NotifyErrorResponse::BadRequest(PlainText(
                format!("Unexpected researcher `{}`. Expected `{}`",
                        body.researcher, hash_data.researcher)
            )))
        } else if hash_data.notification_type != body.notification_type {
            return Err(NotifyErrorResponse::BadRequest(PlainText(
                format!("Unexpected notification_type `{:?}`. Expected `{:?}`",
                        body.notification_type, hash_data.notification_type)
            )))
        } 

        let current_time = SystemTime::now();
        let current_time = current_time
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let current_time: f64 = current_time.as_secs() as f64 * 1000_f64
            + current_time.subsec_nanos() as f64 / 1_000_000_f64;
        info!("measurement_id: {}\tlatency: {}s", body.measurement_id, (current_time - hash_data.timestamp)/(1000_f64));

        Ok(NotifyResponse::Ok)
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let args = CliArgs::parse();
    let secret_key = SecretKey(args.secret_key);
    info!("Notifications service secret key: {:?}", secret_key);

    // let start = SystemTime::now();
    // let since_the_epoch = start
    //     .duration_since(UNIX_EPOCH)
    //     .expect("Time went backwards");
    // let in_ms: f64 = since_the_epoch.as_secs() as f64 * 1000_f64
    //     + since_the_epoch.subsec_nanos() as f64 / 1_000_000_000_f64;
    // let message = json!({
    //     "notification_type": NotificationType::OutOfRange, 
    //     "researcher": "d.landau@uu.nl",
    //     "experiment_id": "5678", 
    //     "measurement_id": "1234", 
    //     "timestamp": in_ms,
    // });
    // println!("{:?}", message.to_json_string());

    // let key: &[u8] = secret_key.0.as_bytes();
    // let key = Key::<Aes256Gcm>::from_slice(key);
    // let cipher = Aes256Gcm::new(&key);
    // let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
    // let ciphertext = cipher.encrypt(&nonce, message.to_json_string().as_bytes().as_ref()).unwrap();

    // let b64_cipher: String = general_purpose::STANDARD_NO_PAD.encode(ciphertext);
    // let b64_nonce: String = general_purpose::STANDARD_NO_PAD.encode(nonce);
    // let b64_nonce_cipher = b64_nonce + "." + &b64_cipher;
    // println!("{:?}", b64_nonce_cipher);

    let api_service = OpenApiService::new(Api, "Hello World", "1.0")
        .server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();
    let app = Route::new()
        .nest("/api", api_service)
        .nest("/", ui)
        .data(secret_key);

    poem::Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}

#[cfg(test)]
mod test { 

}
