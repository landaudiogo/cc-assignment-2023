use clap::Parser;
use poem::{listener::TcpListener, EndpointExt, Route};
use poem_openapi::OpenApiService;
use tracing::{info, Level};

mod api;
mod jwt;
use api::Api;

#[derive(Parser, Debug)]
struct CliArgs {
    #[arg(short, long)]
    secret_key: String,

    #[arg(short, long)]
    external_ip: String,
}

#[derive(Debug, Clone)]
struct SecretKey(String);

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let args = CliArgs::parse();
    let secret_key = SecretKey(args.secret_key);
    info!("Notifications service secret key: {:?}", secret_key);
    let external_ip = args.external_ip;

    let api_service = OpenApiService::new(Api, "Notifications Service", "1.1")
        .server(format!("http://{}:3000/api", external_ip));
    let ui = api_service.swagger_ui();
    let app = Route::new()
        .nest("/api", api_service)
        .nest("/", ui)
        .data(secret_key);

    poem::Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
