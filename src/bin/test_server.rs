use anyhow::Result;
use reqwest::Client;
use std::time::Duration;
use tokio::time::Instant;

const SERVER_URL: &str = "http://157.90.224.91:80";

#[tokio::main]
async fn main() -> Result<()> {
    // Create a reqwest client that stores cookies
    let client = Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(30))
        .build()?;

    println!("Testing root endpoint...");
    let response = client.get(format!("{}", SERVER_URL)).send().await?;
    println!(
        "Root response: {} {}",
        response.status(),
        response.text().await?
    );

    // Step 2: Login to get the JWT token
    println!("\nLogin to get JWT token...");
    let login_response = client.post(format!("{}/login", SERVER_URL)).send().await?;
    println!("Login status: {}", login_response.status());

    let cookies = login_response
        .cookies()
        .map(|c| format!("{}={}", c.name(), c.value()))
        .collect::<Vec<_>>();
    println!("Login cookies: {:?}", cookies);
    println!("Login response: {}", login_response.text().await?);

    println!("\nFetching user info...");
    let fetch_response = client.get(format!("{}/me", SERVER_URL)).send().await?;
    println!("Status: {}", fetch_response.status());
    println!("Response: {}", fetch_response.text().await?);

    // Step 3: Make 5 requests to the protected endpoint
    println!("\nMaking 5 requests to the protected endpoint...");
    for i in 1..=5 {
        println!("\nRequest #{}", i);
        let start = Instant::now();
        let fetch_response = client
            .get(format!("{}/download", SERVER_URL))
            .send()
            .await?;
        println!("Status: {}", fetch_response.status());
        let body = fetch_response.text().await?;
        println!("Body length: {}", body.len());
        let duration = start.elapsed();
        println!("Time taken: {:?}", duration);

        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}

mod tests {
    use reqwest::StatusCode;

    use super::*;

    async fn get_logged_in_client() -> Result<(Client, String)> {
        // Create a reqwest client that stores cookies
        let client = Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        let response = client.get(format!("{}", SERVER_URL)).send().await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Root endpoint should be accessible"
        );

        let login_response = client
            .post(format!("{}/login", SERVER_URL))
            .send()
            .await
            .unwrap();
        assert_eq!(
            login_response.status(),
            StatusCode::OK,
            "Login should be successful"
        );
        // Make sure the JWT cookie is set
        login_response
            .cookies()
            .find(|c| c.name() == "authorization")
            .expect("Authorization JWT cookie not found");

        let user_id = login_response.text().await.unwrap();
        println!("User ID: {}", user_id);

        Ok((client, user_id))
    }

    #[tokio::test]
    async fn test_user_id_decoded() {
        let (client, user_id) = get_logged_in_client().await.unwrap();

        let response = client.get(format!("{}/me", SERVER_URL)).send().await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "User info endpoint should be accessible"
        );

        let body = response.text().await.unwrap();
        println!("Body: {}", body);
        assert!(body.contains(&user_id));
    }
}
