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

#[cfg(test)]
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
        assert!(body.contains(&user_id));
    }


    #[tokio::test]
    async fn test_request_rate_limiting() {
        let (client, _user_id) = get_logged_in_client().await.unwrap();

        let mut count = 0;
        for _i in 0..5 {
            let response = client.get(format!("{}/fetch", SERVER_URL)).send().await.unwrap();
            println!("Response: {}", response.status());
            if response.status() == StatusCode::SERVICE_UNAVAILABLE {
                count += 1;
            }
        }
        assert!(count >= 2, "Rate limit should be exceeded at least 2 times");
    }

    #[tokio::test]
    async fn test_file_download() {
        let (client, _user_id) = get_logged_in_client().await.unwrap();

        let start = Instant::now();
        let response = client.get(format!("{}/download", SERVER_URL)).send().await.unwrap(); // Download 512KB of data
        assert_eq!(response.status(), StatusCode::OK, "Download endpoint should be accessible");
        let _ = response.text().await.unwrap();
        let duration = start.elapsed();
        println!("Time taken: {:?}", duration);
        assert!(duration > Duration::from_secs(3), "Download is limited to 100KB/s and should take at least 3 seconds");
    }

    #[tokio::test]
    async fn test_file_upload() {
        let (client, _user_id) = get_logged_in_client().await.unwrap();

        let response = client.post(format!("{}/upload", SERVER_URL)).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "Upload endpoint should be accessible");
    }
}
