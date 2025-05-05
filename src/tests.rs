#[cfg(test)]
mod tests {
    use reqwest::StatusCode;
    use anyhow::Result;
    use reqwest::Client;
    use std::time::Duration;
    use tokio::time::Instant;
    use rand::Rng;
    
    const SERVER_URL: &str = "http://157.90.224.91:80";

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

        // Generate 1MB of random data
        let mut rng = rand::thread_rng();
        let mut data = vec![0u8; 1_048_576]; // 1MB = 1,048,576 bytes
        rng.fill(&mut data[..]);
        
        // Create a multipart form with the file data
        let part = reqwest::multipart::Part::bytes(data)
            .file_name("test_file.bin")
            .mime_str("application/octet-stream").unwrap();
        
        let form = reqwest::multipart::Form::new()
            .part("file", part);

        let start = Instant::now();
        let response = client
            .post(format!("{}/upload", SERVER_URL))
            .multipart(form)
            .send()
            .await
            .unwrap();
            
        assert_eq!(response.status(), StatusCode::OK, "Upload endpoint should be accessible");
        let duration = start.elapsed();
        println!("Upload time taken: {:?}", duration);
    }
}
