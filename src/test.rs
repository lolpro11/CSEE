use actix_web::error::ResponseError;
use actix_web::{web, App, HttpServer, HttpResponse, Result};
use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, TokenUrl, RedirectUrl, TokenResponse,
};
use oauth2::Scope;
use oauth2::AuthorizationCode;
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read};

// AuthCallbackParams struct for deserialization of query parameters
#[derive(Deserialize)]
struct AuthCallbackParams {
    code: String,
    state: String,
}

// Credentials struct to deserialize from JSON
#[derive(Debug, Deserialize)]
struct Credentials {
    installed: Installed,
}

#[derive(Debug, Deserialize)]
struct Installed {
    client_id: String,
    client_secret: String,
}

lazy_static::lazy_static! {
    static ref CLIENT: BasicClient = {
        // Read credentials from the JSON file
        let credentials = get_credentials().expect("Failed to read credentials from JSON");

        // Google OAuth2 credentials
        let client_id = ClientId::new(credentials.installed.client_id);
        let client_secret = ClientSecret::new(credentials.installed.client_secret);
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/auth".to_string())
            .expect("Failed to parse Auth URL");
        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .expect("Failed to parse Token URL");
        let redirect_url =
            RedirectUrl::new("http://localhost:8080/auth/callback".to_string())
                .expect("Failed to parse Redirect URL");

        // Scopes requested from Google (you can add more if needed)
        let scopes = vec![
            Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()),
            Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()),
        ];

        // Create an OAuth2 client
        BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
            .set_redirect_uri(redirect_url)
    };
}

fn get_credentials() -> io::Result<Credentials> {
    let mut file = File::open("credentials.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let credentials: Credentials = serde_json::from_str(&contents)?;
    Ok(credentials)
}

// Custom error type that implements ResponseError
#[derive(Debug)]
struct MyError(String);

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "An error occurred: {}", self.0)
    }
}

impl ResponseError for MyError {}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/login", web::get().to(login))
            .route("/auth/callback", web::get().to(auth_callback))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

async fn login() -> HttpResponse {
    // Redirect the user to the Google OAuth2 authorization URL
    let (auth_url, csrf_state) = CLIENT
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();
    
    HttpResponse::Found()
        .append_header(("Location", auth_url.to_string()))
        .finish()
}

async fn auth_callback(params: web::Query<AuthCallbackParams>) -> Result<HttpResponse, MyError> {
    // Verify the CSRF token (in a real app, you should use a proper session store)
    let csrf_state = CsrfToken::new(params.state.clone());
    let csrf_secret = csrf_state.secret();

    if &params.state != csrf_secret {
        return Ok(HttpResponse::InternalServerError().finish());
    }

    // Exchange the authorization code for an access token
    let code = AuthorizationCode::new(params.code.clone());

    // Use actix_web::block to run the blocking code asynchronously
    let token_response = actix_web::web::block(move || {
        CLIENT.exchange_code(code).request(http_client)
    })
    .await
    .map_err(|error| MyError(format!("Failed to exchange code for access token: {}", error)))?.unwrap();


    // You can now use the access token as needed
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(format!("Access token: {:?}", token_response.access_token().secret())))
}
