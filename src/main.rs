use axum::{
    http::{StatusCode, HeaderMap, header},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
    extract::{State, multipart::Multipart},
    body::Bytes,
};
use cookie::Cookie;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_cookies::{CookieManagerLayer, Cookies};
use chrono;
use tempfile::TempDir;
use tokio;
use uuid::Uuid;

// Constants
const JWT_SECRET: &[u8] = b"my_super_secret_key"; // In production, use a secure randomly generated key
const COOKIE_NAME: &str = "authorization";

#[tokio::main]
async fn main() {
    // Create a temporary directory for file uploads
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to create temporary directory: {}", e);
            std::process::exit(1);
        }
    };
    
    println!("Temporary upload directory created at: {}", temp_dir.path().display());

    // Create a shared state for the application
    let state = Arc::new(AppState {
        upload_dir: Arc::new(temp_dir),
    });

    // Set up the router with our routes
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/login", post(login_handler))
        .route("/fetch", get(fetch_handler))
        .route("/upload", post(upload_handler))
        .route("/download", get(download_handler))
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
    // Temporary directory for file uploads
    upload_dir: Arc<TempDir>,
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

// File upload handler - authenticated endpoint
async fn upload_handler(
    cookies: Cookies,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Verify authentication
    let token = match cookies.get(COOKIE_NAME) {
        Some(cookie) => cookie.value().to_string(),
        None => return (StatusCode::UNAUTHORIZED, "No session cookie found".to_string()),
    };
    
    // Validate the token
    let token_data = match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid token".to_string()),
    };
    
    // Process the uploaded file
    let user_id = token_data.claims.sub;
    
    // Process the file upload
    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(e) => {
            eprintln!("Error getting next field: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to process upload".to_string());
        }
    } {
        // Get field name, file name and content type
        let _name = field.name().unwrap_or("").to_string();
        let file_name = match field.file_name() {
            Some(name) => name.to_string(),
            None => continue, // Skip fields without a file name
        };
        
        // Generate a unique filename to prevent conflicts
        let unique_filename = format!("{}-{}", Uuid::new_v4(), file_name);
        let file_path = state.upload_dir.path().join(&unique_filename);
        
        // Get the file data
        let data = match field.bytes().await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to read file data: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read uploaded file".to_string());
            }
        };
        
        // Save the file
        if let Err(e) = tokio::fs::write(&file_path, &data).await {
            eprintln!("Failed to save file: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save file".to_string());
        }
        
        println!("User {} uploaded file: {} to {}", user_id, unique_filename, file_path.display());
        
        // Return success after the first file (we'll only handle one file for simplicity)
        return (StatusCode::OK, format!("File uploaded successfully: {}", unique_filename));
    }
    
    // If we get here, no valid file was found in the request
    (StatusCode::BAD_REQUEST, "No file found in request".to_string())
}

// Generate and return 512KB of pseudo data
async fn download_handler(cookies: Cookies) -> Response {
    // Extract the JWT token from the session cookie
    let token = match cookies.get(COOKIE_NAME) {
        Some(cookie) => cookie.value().to_string(),
        None => {
            let body = "No session cookie found".to_string();
            let mut response = Response::new(body.into());
            *response.status_mut() = StatusCode::UNAUTHORIZED;
            return response;
        }
    };
    
    // Validate the token
    let token_data = match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(_) => {
            let body = "Invalid token".to_string();
            let mut response = Response::new(body.into());
            *response.status_mut() = StatusCode::UNAUTHORIZED;
            return response;
        }
    };
    
    // Log the user ID
    let user_id = token_data.claims.sub;
    println!("Download request from user ID: {}", user_id);
    
    // Generate 512KB of pseudo data
    let data_size = 512 * 1024; // 512KB
    let mut rng = rand::thread_rng();
    let data: Vec<u8> = (0..data_size).map(|_| rng.gen::<u8>()).collect();
    
    // Create a response with headers and body
    let mut response = Response::new(data.into());
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap()
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        "attachment; filename=\"random_data.bin\"".parse().unwrap()
    );
    *response.status_mut() = StatusCode::OK;
    
    response
}
