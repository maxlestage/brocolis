// Importation des bibliothèques nécessaires
use actix_web::{
    http::StatusCode, web, App, Error, HttpRequest, HttpResponse, HttpServer, ResponseError,
};
use awc::Client;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use std::fmt;

// Structure pour la réponse
#[derive(Debug, Serialize, Deserialize)]
struct Response {
    message: String,
}

// Structure pour les erreurs personnalisées
#[derive(Debug)]
pub struct MyError {
    name: String,
}

// Implémentation de l'affichage pour MyError
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

// Fonction pour transférer les requêtes
async fn forward(
    req: HttpRequest,          // La requête HTTP entrante
    body: web::Bytes,          // Le corps de la requête
    client: web::Data<Client>, // Le client HTTP
) -> Result<HttpResponse, Error> {
    // Renvoie une réponse HTTP ou une erreur
    // URL du serveur de test
    let server_test_url = "http://127.0.0.1:8081".to_string();

    // Décompose le corps de la requête
    let mut body_str = String::from_utf8(body.to_vec()).unwrap();
    println!("le corps de la requête initiale : {:?}", &body_str);

    // Modifie le corps de la requête
    body_str = "Ceci est une requête modifiée par le proxy".to_string();

    // Recompose le corps de la requête
    let new_body = web::Bytes::from(body_str);

    // Crée une nouvelle requête à partir de la requête entrante
    let forwarded_req = client
        .request_from(server_test_url, req.head())
        .no_decompress();

    // Envoie la requête et gère les erreurs
    let mut res = forwarded_req.send_body(new_body).await.map_err(|e| {
        let my_error = MyError {
            name: format!("La requête de transfert a échoué : {}", e.to_string()),
        };
        actix_web::Error::from(my_error)
    })?;

    // Log the response body from the test server
    println!("Réponse du serveur de test : {:?}", &res.body().await?);

    // Construit la réponse
    let mut client_resp = HttpResponse::build(res.status());

    // Ajoute les en-têtes de la réponse
    for (header_name, header_value) in res.headers().iter() {
        client_resp.append_header((header_name.clone(), header_value.clone()));
    }

    // Ajoute un en-tête spécifique au proxy
    client_resp.append_header(("X-Forwarded-By", "My Proxy"));

    // Modifie le corps de la réponse
    let new_body = "Ceci est une réponse modifiée par le proxy";

    // Renvoie la réponse du client
    Ok(client_resp.body(new_body))
}

// Point d'entrée de l'application
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Crée un nouveau client Reqwest
    let reqwest_client = ReqwestClient::new();

    // Crée un nouveau serveur HTTP
    let server = match HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Client::default())) // Ajoute le client HTTP à l'application
            .app_data(web::Data::new(reqwest_client.clone())) // Ajoute le client Reqwest à l'application
            .route("/", web::route().to(forward)) // Ajoute la route pour la fonction forward
    })
    .bind("127.0.0.1:8080") // Lie le serveur à l'adresse et au port spécifiés
    {
        Ok(server) => server, // Si le serveur est créé avec succès, renvoie le serveur
        Err(e) => return Err(e), // Si une erreur se produit, renvoie l'erreur
    };

    // Démarre le serveur
    server.run().await
}
