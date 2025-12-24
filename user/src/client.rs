use proto::user::{
    GetUserProfileRequest, LoginRequest, RegisterRequest, UpdateUserProfileRequest, VerifyRequest,
    user_service_client::UserServiceClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = UserServiceClient::connect("http://127.0.0.1:50051").await?;

    println!("Connected to User Service");
    println!("=========================\n");

    // Test 1: Register a new user
    println!("1. Testing User Registration");
    let register_request = RegisterRequest {
        username: "john_doe".to_string(),
        email: "john@example.com".to_string(),
        password: "securepassword123".to_string(),
        full_name: "John Doe".to_string(),
        phone_number: "+1234567890".to_string(),
    };

    let register_response = client.register(register_request).await?;
    let register_result = register_response.into_inner();
    println!("Register Response:");
    println!("  Success: {}", register_result.success);
    println!("  Message: {}", register_result.message);
    println!("  User ID: {}\n", register_result.user_id);

    // Test 2: Login with the registered user
    println!("2. Testing User Login");
    let login_request = LoginRequest {
        username: "john_doe".to_string(),
        password: "securepassword123".to_string(),
    };

    let login_response = client.login(login_request).await?;
    let login_result = login_response.into_inner();
    println!("Login Response:");
    println!("  Success: {}", login_result.success);
    println!("  Message: {}", login_result.message);
    println!("  Token: {}", login_result.token);
    if let Some(user) = &login_result.user {
        println!("  User ID: {}", user.user_id);
        println!("  Username: {}", user.username);
        println!("  Email: {}\n", user.email);
    }

    let user_id = login_result
        .user
        .as_ref()
        .map(|u| u.user_id.clone())
        .unwrap_or_default();

    // Test 3: Verify the token
    println!("3. Testing Token Verification");
    let verify_request = VerifyRequest {
        user_id: user_id.clone(),
    };

    let verify_response = client.verify(verify_request).await?;
    let verify_result = verify_response.into_inner();
    println!("Verify Response:");
    println!("  Valid: {}", verify_result.valid);
    println!("  User ID: {}", verify_result.user_id);
    println!("  Message: {}\n", verify_result.message);

    // Test 4: Get user profile
    println!("4. Testing Get User Profile");
    let profile_request = GetUserProfileRequest {
        user_id: user_id.clone(),
    };

    let profile_response = client.get_user_profile(profile_request).await?;
    let profile_result = profile_response.into_inner();
    println!("Get Profile Response:");
    println!("  Success: {}", profile_result.success);
    println!("  Message: {}", profile_result.message);
    if let Some(user) = &profile_result.user {
        println!("  Username: {}", user.username);
        println!("  Email: {}", user.email);
        println!("  Created At: {}", user.created_at);
        println!("  Updated At: {}\n", user.updated_at);
    }

    // Test 5: Update user profile
    println!("5. Testing Update User Profile");
    let update_request = UpdateUserProfileRequest {
        user_id: user_id.clone(),
        email: "john.doe@example.com".to_string(),
        full_name: "John Updated Doe".to_string(),
        phone_number: "+0987654321".to_string(),
    };

    let update_response = client.update_user_profile(update_request).await?;
    let update_result = update_response.into_inner();
    println!("Update Profile Response:");
    println!("  Success: {}", update_result.success);
    println!("  Message: {}", update_result.message);
    if let Some(user) = &update_result.user {
        println!("  Updated Email: {}", user.email);
        println!("  Updated At: {}\n", user.updated_at);
    }

    // Test 6: Try to login with wrong password
    println!("6. Testing Login with Wrong Password");
    let wrong_login_request = LoginRequest {
        username: "john_doe".to_string(),
        password: "wrongpassword".to_string(),
    };

    let wrong_login_response = client.login(wrong_login_request).await?;
    let wrong_login_result = wrong_login_response.into_inner();
    println!("Wrong Login Response:");
    println!("  Success: {}", wrong_login_result.success);
    println!("  Message: {}\n", wrong_login_result.message);

    // Test 7: Verify an invalid token
    println!("7. Testing Invalid Token Verification");
    let invalid_verify_request = VerifyRequest {
        user_id: "invalid_user_id".to_string(),
    };

    let invalid_verify_response = client.verify(invalid_verify_request).await?;
    let invalid_verify_result = invalid_verify_response.into_inner();
    println!("Invalid Verify Response:");
    println!("  Valid: {}", invalid_verify_result.valid);
    println!("  Message: {}\n", invalid_verify_result.message);

    println!("=========================");
    println!("All tests completed!");

    Ok(())
}
