use proto::user::{user_service_client::UserServiceClient, LoginRequest};
use tonic::Request;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting concurrent login requests test...");
    println!("Sending 1 login request per second\n");

    let url = "http://127.0.0.1:50051";
    let total_requests = 60; // Adjust this number as needed

    // Spawn concurrent tasks
    let mut handles = vec![];

    for i in 0..total_requests {
        let handle = tokio::spawn(async move {
            // Wait i seconds before sending request (1 request per second)
            sleep(Duration::from_secs(i)).await;

            // Create new client for each request
            let mut client = match UserServiceClient::connect(url).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Request {}: Failed to connect: {}", i + 1, e);
                    return;
                }
            };

            let mut login_request = Request::new(LoginRequest {
                username: "john_doe".to_string(),
                password: "securepassword123".to_string(),
            });

            login_request.metadata_mut().insert("x-forwarded-for", "127.0.0.1".parse().unwrap());  

            println!("Request {}: Sending login request at {:?}", i + 1, std::time::Instant::now());

            match client.login(login_request).await {
                Ok(response) => {
                    let result = response.into_inner();
                    println!(
                        "Request {}: Success={}, Message={}, Token={:?}",
                        i + 1,
                        result.success,
                        result.message,
                        if result.token.is_empty() { "None" } else { "Received" }
                    );
                }
                Err(e) => {
                    eprintln!("Request {}: Error: {}", i + 1, e);
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let _ = handle.await;
    }

    println!("\nAll requests completed!");
    Ok(())
}