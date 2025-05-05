use anyhow::Result;
use reqwest::Client;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a reqwest client that stores cookies
    let client = Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let server_url = "http://157.90.224.91:80";
    
    // Step 1: Test the root endpoint
    println!("Testing root endpoint...");
    let response = client.get(format!("{}", server_url)).send().await?;
    println!("Root response: {} {}", response.status(), response.text().await?);
    
    // Step 2: Login to get the JWT token
    println!("\nLogin to get JWT token...");
    let login_response = client.post(format!("{}/login", server_url)).send().await?;
    println!("Login status: {}", login_response.status());
    println!("Login response: {}", login_response.text().await?);
    
    // Step 3: Make 5 requests to the protected endpoint
    println!("\nMaking 5 requests to the protected endpoint...");
    for i in 1..=5 {
        println!("\nRequest #{}", i);
        let fetch_response = client.get(format!("{}/fetch", server_url)).send().await?;
        println!("Status: {}", fetch_response.status());
        println!("Response: {}", fetch_response.text().await?);
        
        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    Ok(())
} 