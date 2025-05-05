use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use cookie::Cookie;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_cookies::{CookieManagerLayer, Cookies};
use chrono;

// Constants
const JWT_SECRET: &[u8] = b"my_super_secret_key"; // In production, use a secure randomly generated key
const COOKIE_NAME: &str = "jwt_session";

#[tokio::main]
async fn main() {
    // Create a shared state for the application
    let state = Arc::new(AppState {
        // Add any shared state here if needed
    });

    // Set up the router with our routes
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/login", post(login_handler))
        .route("/fetch", get(fetch_handler))
        .layer(CookieManagerLayer::new())
        .with_state(state);

    // Start the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Server started on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}

// Application state (can be expanded as needed)
#[derive(Clone)]
struct AppState {
    // Add fields as needed
}

// Root handler - no authentication required
async fn root_handler() -> impl IntoResponse {
    (StatusCode::OK, "jwt test")
}

// Define the JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,  // Subject (user ID)
    exp: usize,   // Expiration time
}

// Login handler - will generate a JWT token and set it as a cookie
async fn login_handler(cookies: Cookies) -> impl IntoResponse {
    // Generate a random user ID between 1 and 10000
    let user_id = rand::thread_rng().gen_range(1..=10000).to_string();
    
    // Set expiration time to 60 minutes from now
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::minutes(60))
        .expect("valid timestamp")
        .timestamp() as usize;
    
    // Create the claims
    let claims = Claims {
        sub: user_id.clone(),
        exp: expiration,
    };
    
    // Create the JWT token
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .expect("JWT token creation failed");
    
    // Create a cookie with the token
    let mut cookie = Cookie::new(COOKIE_NAME, token);
    cookie.set_http_only(true);
    cookie.set_path("/");
    
    cookies.add(cookie);
    
    // Return the user ID in the response for demonstration purposes
    format!("Login successful. User ID: {}", user_id)
}

// Protected fetch handler - will validate the JWT token
async fn fetch_handler(cookies: Cookies) -> impl IntoResponse {
    // Extract the JWT token from the session cookie
    let token = match cookies.get(COOKIE_NAME) {
        Some(cookie) => cookie.value().to_string(),
        None => return (StatusCode::UNAUTHORIZED, "No session cookie found"),
    };
    
    // Validate the token
    let token_data = match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid token"),
    };
    
    // Log the user ID (useful for testing Nginx rate limiting)
    let user_id = token_data.claims.sub;
    println!("Request from user ID: {}", user_id);
    
    // Return "Hello, world!" if the token is valid
    (StatusCode::OK, "Hello, world!")
}
