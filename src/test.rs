use actix_web::{web, App, HttpResponse, HttpServer, Result, ResponseError};
use oauth2::basic::{BasicClient, BasicTokenType};
use oauth2::reqwest::http_client;
use oauth2::{
    AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken, TokenUrl, RedirectUrl, TokenResponse, EmptyExtraTokenFields, AccessToken, RefreshToken, Scope,
};
use serde::{Deserialize, Serialize};
use oauth2::StandardTokenResponse;
use std::fs::File;
use std::io::{self, Read, Write};

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
        .add_scope(Scope::new("https://www.googleapis.com/auth/drive.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.announcements.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.courses.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.coursework.students.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.coursework.me.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.courseworkmaterials.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.rosters.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.profile.emails".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.profile.photos".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.topics.readonly".to_string()))
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
    .map_err(|error| MyError(format!("Failed to exchange code for access token: {}", error)))?
    .unwrap();

    // Save the token response to a JSON file
    save_tokens_to_file(&token_response)?;

    Ok(HttpResponse::Ok().finish())
}

#[derive(Debug, Serialize, Deserialize)]
struct MyTokenResponse {
    scopes: Vec<String>,
    token: TokenInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenInfo {
    access_token: AccessToken,
    refresh_token: Option<RefreshToken>,
    expires_at: i64, // Change this to i64
    id_token: Option<String>,
}

impl From<&StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>> for MyTokenResponse {
    fn from(token_response: &StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>) -> Self {
        MyTokenResponse {
            scopes: token_response.scopes().map_or_else(Vec::new, |scopes| {
                scopes.iter().map(|scope| scope.as_str().to_owned()).collect()
            }),
            token: TokenInfo {
                access_token: token_response.access_token().clone(),
                refresh_token: token_response.refresh_token().cloned(),
                expires_at: token_response.expires_in().map_or(0, |duration| duration.as_secs() as i64),
                id_token: None, // You can add the id_token if it is present in your token response
            },
        }
    }
}

impl From<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>> for MyTokenResponse {
    fn from(token_response: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>) -> Self {
        MyTokenResponse::from(&token_response)
    }
}

impl From<std::io::Error> for MyError {
    fn from(error: std::io::Error) -> Self {
        MyError(format!("IO Error: {}", error))
    }
}

fn save_tokens_to_file(token_response: &StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>) -> io::Result<()> {
    let my_token_response: MyTokenResponse = token_response.into();
    let json = serde_json::to_string(&my_token_response)
        .expect("Failed to serialize token response");
    let mut file = File::create("tokens.json")?;
    file.write_all(json.as_bytes())?;
    Ok(())
}
