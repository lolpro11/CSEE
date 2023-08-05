use oauth2::{
    AuthorizationCode, AuthorizationCodeUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl,
    TokenResponse,
};
use rocket::http::Cookies;
use rocket::request::Form;
use rocket::response::Redirect;
use rocket::State;
use std::env;

const REDIRECT_URI: &str = "http://localhost:8000/auth/callback"; // Update this URL to match your setup

// Initialize the OAuth2 client
lazy_static::lazy_static! {
    static ref CLIENT: oauth2::Client = {
        let client_id =
            ClientId::new(env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| CLIENT_ID.to_string()));
        let client_secret =
            ClientSecret::new(env::var("GOOGLE_CLIENT_SECRET").unwrap_or_else(|_| CLIENT_SECRET.to_string()));
        oauth2::Client::new(client_id, Some(client_secret), oauth2::AuthUrl::new("https://accounts.google.com/o/oauth2/auth".to_string()), oauth2::TokenUrl::new("https://accounts.google.com/o/oauth2/token".to_string()))
    };
}

#[get("/login")]
fn login() -> Redirect {
    // Generate the authorization URL
    let (authorize_url, csrf_state) = CLIENT
        .authorize_url(CsrfToken::new_random)
        .add_scope(oauth2::Scope::new("email".to_string()))
        .add_scope(oauth2::Scope::new("profile".to_string()))
        .url();

    // Save the CSRF token to the session (in a real app, you should use a proper session store)
    // This is just for simplicity and might not be secure in a production setup.
    // Make sure to use a proper session management library in a real-world scenario.
    let csrf_token = csrf_state.secret();
    println!("CSRF token: {}", csrf_token);

    Redirect::to(authorize_url.to_string())
}

#[derive(FromForm)]
struct AuthCallbackParams {
    code: String,
    state: String,
}

#[get("/auth/callback?<params..>")]
fn auth_callback(params: Form<AuthCallbackParams>, cookies: &Cookies) -> Redirect {
    // Verify the CSRF token (in a real app, you should use a proper session store)
    let csrf_state = CsrfToken::new(params.state.clone());
    let _ = csrf_state.verify_secret(&params.state);

    // Exchange the authorization code for an access token
    let code = AuthorizationCode::new(params.code.clone());
    let token_response = CLIENT.exchange_code(code).request(oauth2::GrantType::AuthorizationCode);

    match token_response {
        Ok(token) => {
            // Save the access token to the session (in a real app, you should use a proper session store)
            // This is just for simplicity and might not be secure in a production setup.
            // Make sure to use a proper session management library in a real-world scenario.
            cookies.add(oauth2::AccessToken::cookie(&token.access_token().secret()));
            Redirect::to("/")
        }
        Err(_) => Redirect::to("/error"),
    }
}

fn main() {
    rocket::ignite().mount("/", routes![login, auth_callback]).launch();
}
