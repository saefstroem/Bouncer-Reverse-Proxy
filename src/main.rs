use actix_cors::Cors;
use actix_web::{
    get,
    web::scope,
    App, HttpResponse, HttpServer, Responder,
};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde::Deserialize;
use std::env;

mod websockets;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Bouncer Reverse Proxy")
}

#[derive(Deserialize)]
pub struct Configuration {
    pub target: String,
    pub rate_limit: u64,
    pub ip_ban: bool,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let port: u64 = env::var("PORT")
        .expect("PORT not set")
        .parse::<u64>()
        .expect("Not a valid port");


    // Configure the rate limiter middleware
    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_header()
            .allow_any_method()
            .supports_credentials();
        App::new().wrap(cors).service(hello).service(
            // Setting up a scope for all keys-related routes
            scope("/ws").configure(websockets::router::config), // Utilizing the configuration from controllers::keys
        )
    });

    let ssl_cert_file = env::var("FULLCHAIN").unwrap_or_default();
    let ssl_key_file = env::var("PRIVKEY").unwrap_or_default();

    if ssl_cert_file.len() == 0 || ssl_key_file.len() == 0 {
        return server.bind(format!("0.0.0.0:{}", port))?.run().await;
    }

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file(ssl_key_file, SslFiletype::PEM)
        .unwrap();
    builder
        .set_certificate_chain_file(ssl_cert_file)
        .unwrap();

    server
        .bind_openssl(format!("0.0.0.0:{}", port), builder)?
        .run()
        .await
}
