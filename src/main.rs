use actix_web::{
    http::StatusCode, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
    ResponseError,
};
use awc::Client;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    message: String,
}

#[derive(Debug)]
pub struct MyError {
    name: String,
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl ResponseError for MyError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// Fonction pour transférer les requêtes
async fn forward(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<Client>,
) -> Result<HttpResponse, Error> {
    let server_test_url = "http://127.0.0.1:8081".to_string();

    let forwarded_req = client
        .request_from(server_test_url, req.head())
        .no_decompress();

    let mut res = forwarded_req.send_body(body).await.map_err(|e| {
        let my_error = MyError {
            name: format!("La requête de transfert a échoué : {}", e.to_string()),
        };
        actix_web::Error::from(my_error)
    })?;

    let mut client_resp = HttpResponse::build(res.status());

    for (header_name, header_value) in res.headers().iter() {
        client_resp.append_header((header_name.clone(), header_value.clone()));
    }

    client_resp.append_header(("X-Forwarded-By", "My Proxy"));

    // Modifie le corps de la réponse
    let new_body = "Ceci est une réponse modifiée par le proxy";

    Ok(client_resp.body(new_body)) // Renvoie la réponse du client
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let reqwest_client = ReqwestClient::new();

    let server = match HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Client::default()))
            .app_data(web::Data::new(reqwest_client.clone()))
            .route("/", web::route().to(forward))
    })
    .bind("127.0.0.1:8080")
    {
        Ok(server) => server,
        Err(e) => return Err(e),
    };

    server.run().await
}
