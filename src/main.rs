// Importation des bibliothèques nécessaires
use actix_web::{http::header, web, App, HttpResponse, HttpServer};
use reqwest::Client;
use serde::Serialize;
use std::convert::Infallible;

// Structure représentant la réponse
#[derive(Debug, Serialize)]
struct Response {
    message: String,
}

// Fonction asynchrone qui sera appelée lorsqu'une requête arrive
async fn forward(client: web::Data<Client>) -> Result<HttpResponse, Infallible> {
    // Envoi d'une requête GET à l'adresse du serveur de test
    let _ = client.get("http://127.0.0.1:8080").send().await.unwrap();

    // Interceptation et modification du message
    let intercepted_message = Response {
        message: "I get it".to_string(),
    };

    // Encodage de la réponse interceptée en JSON
    let json = serde_json::to_string(&intercepted_message).unwrap();

    // Création d'une réponse HTTP avec le JSON comme corps
    Ok(HttpResponse::Ok()
        .append_header((header::CONTENT_TYPE, "application/json"))
        .body(json))
}

// Point d'entrée de l'application
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Création d'un nouveau client HTTP
    let client = Client::new();

    // Création d'un nouveau serveur HTTP
    HttpServer::new(move || {
        // Création d'une nouvelle application avec une route
        App::new()
            .app_data(web::Data::new(client.clone()))
            .route("/", web::get().to(forward))
    })
    // Liaison du serveur à une adresse et un port
    .bind("127.0.0.1:8081")?
    // Démarrage du serveur
    .run()
    .await
}
