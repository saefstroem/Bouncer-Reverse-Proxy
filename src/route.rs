use actix_web::HttpResponse;



#[post("/{chain}/{key}")]
pub async fn route_to_chain(
    path: Path<(String, String)>,
    req: HttpRequest,
    mut payload: web::Payload,
    client: web::Data<Client>,
    state: web::Data<Forest>,
) -> HttpResponse {
    let (chain, key) = path.into_inner();
    if key.is_empty() || chain.is_empty() {
        return HttpResponse::BadRequest().into();
    }
    let key_hash = hash_now(&key);

    match get::<Key>(&state.keys_tree, &key_hash).await {
        Ok(key_details) => {
            let now_in_seconds = get_unix_time_now();
            let expiry_ok = key_details.expires > now_in_seconds;
            if !expiry_ok {
                return HttpResponse::Unauthorized().into();
            }
            let rate_limit_ok:bool = {
                if key_details.timeout_millis==0{
                    true
                }else{
                    let now_in_millis=get_unix_time_now_millis();
                    let rate_limit_ok=now_in_millis>=key_details.wait_until_millis;
                    if rate_limit_ok {
                        let new_wait_until_millis=now_in_millis+key_details.timeout_millis;
                        let _=set::<Key>(&state.keys_tree, &key_hash, &Key {
                            expires: key_details.expires,
                            timeout_millis: key_details.timeout_millis,
                            wait_until_millis: new_wait_until_millis,
                        }).await;
                    };
                    rate_limit_ok
                }
            };

            if !rate_limit_ok {
                return HttpResponse::TooManyRequests().into();
            }



            match get::<Chain>(&state.chains_tree, &chain).await {
                Ok(chain_details) => {
                    // Read the payload as a stream of chunks (`Bytes`)
                    let mut bytes = web::BytesMut::new();
                    while let Some(item) = payload.next().await {
                        bytes.extend_from_slice(&item.unwrap());
                    }
                    spawn(report(Transfer{
                        key_hash,
                        is_ws: false,
                        bytes: bytes.len() as u128,
                    }));
                    let proxy_req = client
                        .request_from(chain_details.http_url, req.head())
                        .no_decompress();
                    let res = proxy_req.send_body(web::Bytes::from(bytes)).await;
                    match res {
                        Ok(recv_response) => {
                            /*  let mut client_resp = HttpResponse::build(recv_response.status());

                            // Remove `Connection` as per
                            // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
                            for (header_name, header_value) in recv_response
                                .headers()
                                .iter()
                                .filter(|(h, _)| *h != "connection")
                            {
                                client_resp
                                    .insert_header((header_name.clone(), header_value.clone()));
                            }*/
                            let mut client_resp = HttpResponse::build(recv_response.status());
                            for (header_name, header_value) in recv_response.headers() {
                                if header_name != "connection" {
                                    client_resp
                                        .append_header((header_name.clone(), header_value.clone()));
                                }
                            }
                            client_resp.streaming(recv_response)
                        }
                        Err(_error) => HttpResponse::InternalServerError().into(),
                    }
                }
                Err(error) => match error {
                    DatabaseError::NotFound => HttpResponse::NotFound().into(),
                    _ => {
                        log::error!("Could not register because: {}", error);
                        HttpResponse::InternalServerError().into()
                    }
                },
            }
        }
        Err(error) => match error {
            DatabaseError::NotFound => HttpResponse::NotFound().into(),
            _ => {
                log::error!("Could not register because: {}", error);
                HttpResponse::InternalServerError().into()
            }
        },
    }
}

pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(route_to_chain);
}
