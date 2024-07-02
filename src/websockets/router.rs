
use actix_web::{get, web, HttpRequest};
use actix_web::{
    web::ServiceConfig,
    HttpResponse,
};
use once_cell::sync::Lazy;
use crate::websockets::proxy::start;


static ROUTE_TO:Lazy<String> = Lazy::new(|| {
    std::env::var("ROUTE_TO").expect("ROUTE_TO must be set")
});

#[get("/")]
pub async fn route_to_chain_ws(
    req: HttpRequest,
    payload: web::Payload,
) -> HttpResponse {
    let ws_connection_request = start(
        &req,
        ROUTE_TO.to_string(),
        payload,
        10000000,
    )
    .await;
    match ws_connection_request {
        Ok(ws_connection) => ws_connection,
        Err(_error) => HttpResponse::InternalServerError().into(),
    }
}

// Exporting the handlers to be used in the app configuration
pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(route_to_chain_ws);
}
