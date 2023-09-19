// Importation des bibliothèques nécessaires pour créer un serveur web et un client HTTP,
// manipuler des données JSON, et formater des chaînes de caractères.
use actix_web::{
    http::StatusCode, web, App, Error, HttpRequest, HttpResponse, HttpServer, ResponseError,
};
use awc::Client;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;

// Définition de la structure de données pour la réponse HTTP.
// Elle contient un seul champ, "message", qui est une chaîne de caractères.
#[derive(Debug, Serialize, Deserialize)]
struct Response {
    message: String,
}

// Définition de la structure de données pour les erreurs personnalisées.
// Elle contient un seul champ, "name", qui est une chaîne de caractères.
#[derive(Debug)]
pub struct MyError {
    name: String,
}

// Implémentation de la méthode d'affichage pour MyError.
// Cette méthode est utilisée pour convertir l'erreur en une chaîne de caractères lisible.
impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// Implémentation de ResponseError pour MyError.
// Cette méthode est utilisée pour convertir l'erreur en une réponse HTTP avec un code d'erreur 500.
impl ResponseError for MyError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// Fonction pour transférer les requêtes HTTP.
// Cette fonction prend en entrée une requête HTTP, le corps de la requête et un client HTTP,
// et renvoie une réponse HTTP ou une erreur.
async fn forward(
    req: HttpRequest,
    // La requête HTTP entrante
    body: web::Bytes,          // Le corps de la requête
    client: web::Data<Client>, // Le client HTTP
) -> Result<HttpResponse, Error> {
    println!("Début de la fonction forward");

    // URL du serveur de test
    let server_test_url = "http://127.0.0.1:8081".to_string();

    // Conversion du corps de la requête en une chaîne de caractères
    let mut body_str = String::from_utf8(body.to_vec()).unwrap();
    println!("Corps de la requête initiale : {}", body_str);

    // Création d'un nouvel objet JSON pour la requête modifiée
    let request_data = json!({
        "message": "Ceci est une requête modifiée par le proxy"
    });

    // Conversion de l'objet JSON en une chaîne de caractères
    body_str = request_data.to_string();
    println!("Corps de la requête modifiée : {}", body_str);

    // Conversion de la chaîne de caractères en Bytes pour l'envoyer dans la requête
    let new_body = web::Bytes::from(body_str);

    // Création d'une nouvelle requête à partir de la requête entrante et envoi de la requête
    let forwarded_req = client
        .request_from(server_test_url, req.head())
        .no_decompress();
    println!("Nouvelle requête créée et envoyée");

    // Gestion des erreurs lors de l'envoi de la requête
    let res = forwarded_req.send_body(new_body).await.map_err(|e| {
        let my_error = MyError {
            name: format!("La requête de transfert a échoué : {}", e.to_string()),
        };
        println!("Erreur lors de l'envoi de la requête : {}", my_error);
        actix_web::Error::from(my_error)
    })?;

    // Construction de la réponse à renvoyer au client
    let mut client_resp = HttpResponse::build(res.status());
    println!("Réponse du serveur reçue et en cours de construction");

    // Ajout des en-têtes de la réponse
    for (header_name, header_value) in res.headers().iter() {
        client_resp.append_header((header_name.clone(), header_value.clone()));
    }

    // Ajout d'un en-tête spécifique au proxy
    client_resp.append_header(("X-Forwarded-By", "My Proxy"));

    // Création d'un nouvel objet JSON pour la réponse modifiée
    let response_data = json!({
        "message": "Ceci est une réponse modifiée par le proxy"
    });

    // Conversion de l'objet JSON en une chaîne de caractères
    let new_body = response_data.to_string();
    println!("Corps de la réponse modifiée : {}", new_body);

    // Ajout du type de contenu à la réponse
    client_resp.append_header(("Content-Type", "application/json"));

    println!("Réponse prête à être renvoyée au client");
    // Renvoi de la réponse au client
    Ok(client_resp.body(new_body))
}

// Point d'entrée de l'application
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Création d'un nouveau client HTTP avec la bibliothèque Reqwest
    let reqwest_client = ReqwestClient::new();

    // Création d'un nouveau serveur HTTP avec la bibliothèque Actix-web
    let server = match HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Client::default())) // Ajout du client HTTP à l'application
            .app_data(web::Data::new(reqwest_client.clone())) // Ajout du client Reqwest à l'application
            .route("/", web::route().to(forward)) // Ajout de la route pour la fonction forward
    })
    .bind("127.0.0.1:8080") // Liaison du serveur à l'adresse et au port spécifiés
    {
        Ok(server) => server, // Si le serveur est créé avec succès, renvoie le serveur
        Err(e) => return Err(e), // Si une erreur se produit, renvoie l'erreur
    };

    // Démarrage du serveur
    server.run().await
}
