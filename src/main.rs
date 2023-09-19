// Importation des bibliothèques nécessaires
use actix_web::{
    http::StatusCode, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
    ResponseError,
}; // actix_web est un framework web puissant et flexible
use awc::Client; // awc est un client HTTP pour Actix
use reqwest::Client as ReqwestClient; // reqwest est un client HTTP asynchrone facile à utiliser
use serde::{Deserialize, Serialize}; // serde est une bibliothèque pour la sérialisation et la désérialisation de données structurées
use std::fmt;

// Définition d'une structure pour la réponse
#[derive(Debug, Serialize, Deserialize)]
struct Response {
    message: String,
}

// Définition d'une structure pour l'erreur personnalisée
#[derive(Debug)]
pub struct MyError {
    name: String,
}

// Implémentation de Display pour MyError
impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// Implémentation de ResponseError pour MyError
impl ResponseError for MyError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// // Fonction pour gérer les requêtes GET
// async fn get_request(_client: web::Data<ReqwestClient>) -> impl Responder {
//     HttpResponse::Ok().body("Requête GET reçue") // Renvoie une réponse HTTP avec le statut 200 et le corps "Requête GET reçue"
// }

// // Fonction pour gérer les requêtes POST
// async fn post_request(
//     _client: web::Data<ReqwestClient>,
//     _body: web::Json<Response>,
// ) -> impl Responder {
//     HttpResponse::Ok().body("Requête POST reçue") // Renvoie une réponse HTTP avec le statut 200 et le corps "Requête POST reçue"
// }

// // Fonction pour gérer les requêtes PUT
// async fn put_request(
//     _client: web::Data<ReqwestClient>,
//     _body: web::Json<Response>,
// ) -> impl Responder {
//     HttpResponse::Ok().body("Requête PUT reçue") // Renvoie une réponse HTTP avec le statut 200 et le corps "Requête PUT reçue"
// }

// // Fonction pour gérer les requêtes PATCH
// async fn patch_request(
//     _client: web::Data<ReqwestClient>,
//     _body: web::Json<Response>,
// ) -> impl Responder {
//     HttpResponse::Ok().body("Requête PATCH reçue") // Renvoie une réponse HTTP avec le statut 200 et le corps "Requête PATCH reçue"
// }

// Fonction pour transférer les requêtes
async fn forward(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<Client>,
) -> Result<HttpResponse, Error> {
    let server_test_url = "http://127.0.0.1:8081".to_string(); // URL de votre serveur de test

    let forwarded_req = client
        .request_from(server_test_url, req.head())
        .no_decompress();
    let mut res = forwarded_req.send_body(body).await.map_err(|e| {
        let my_error = MyError {
            name: format!("La requête de transfert a échoué : {}", e.to_string()),
        };
        actix_web::Error::from(my_error) // Convertit MyError en actix_web::Error
    })?;

    let mut client_resp = HttpResponse::build(res.status());

    for (header_name, header_value) in res.headers().iter() {
        client_resp.append_header((header_name.clone(), header_value.clone()));
    }

    // Ajoute un en-tête spécifique au proxy
    client_resp.append_header(("X-Forwarded-By", "My Proxy"));

    // Modifie le corps de la réponse
    let new_body = "Ceci est une réponse modifiée par le proxy";

    // Log la réponse modifiée
    println!("Réponse modifiée : {}", new_body);

    Ok(client_resp.body(new_body)) // Renvoie la réponse du client
}

// Fonction principale
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let reqwest_client = ReqwestClient::new(); // Crée un nouveau client Reqwest

    let server = match HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Client::default())) // Ajoute les données du client par défaut à l'application
            .app_data(web::Data::new(reqwest_client.clone())) // Ajoute les données du client Reqwest à l'application
            .route("/", web::route().to(forward)) // Ajoute une route pour transférer les requêtes à l'application
    })
    .bind("127.0.0.1:8080")
    {
        Ok(server) => server,
        Err(e) => return Err(e),
    };

    server.run().await
}
