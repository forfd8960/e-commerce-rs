use anyhow::Result;
use bcrypt::{DEFAULT_COST, hash, verify};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use proto::user::{
    GetUserProfileRequest, GetUserProfileResponse, LoginRequest, LoginResponse, RegisterRequest,
    RegisterResponse, UpdateUserProfileRequest, UpdateUserProfileResponse, User, VerifyRequest,
    VerifyResponse, user_service_server::UserService,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tonic::{Request, Response, Status};
use tracing::{error, info, warn};
use uuid::Uuid;

const JWT_SECRET: &str = "your-secret-key-change-in-production";
const TOKEN_EXPIRATION_HOURS: i64 = 24;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // user_id
    exp: i64,    // expiration time
    iat: i64,    // issued at
}

#[derive(Debug, sqlx::FromRow)]
struct DbUser {
    id: String,
    username: String,
    email: String,
    password_hash: String,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

pub struct UserServiceImpl {
    db: PgPool,
}

impl UserServiceImpl {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    fn generate_token(&self, user_id: &str) -> Result<String> {
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            sub: user_id.to_string(),
            exp: now + (TOKEN_EXPIRATION_HOURS * 3600),
            iat: now,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
        )?;

        Ok(token)
    }

    fn verify_token(&self, token: &str) -> Result<String> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
            &Validation::default(),
        )?;

        Ok(token_data.claims.sub)
    }

    fn db_user_to_proto(&self, db_user: &DbUser) -> User {
        User {
            user_id: db_user.id.clone(),
            username: db_user.username.clone(),
            email: db_user.email.clone(),
            full_name: String::new(),    // Not stored in current schema
            phone_number: String::new(), // Not stored in current schema
            created_at: db_user.created_at.and_utc().timestamp(),
            updated_at: db_user.updated_at.and_utc().timestamp(),
        }
    }
}

#[tonic::async_trait]
impl UserService for UserServiceImpl {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();

        // Validate input
        if req.username.is_empty() || req.email.is_empty() || req.password.is_empty() {
            warn!("Register validation failed: missing required fields");
            return Ok(Response::new(RegisterResponse {
                success: false,
                message: "Username, email, and password are required".to_string(),
                user_id: String::new(),
            }));
        }

        // Hash password
        let password_hash = hash(&req.password, DEFAULT_COST).map_err(|e| {
            error!("Failed to hash password: {}", e);
            Status::internal(format!("Failed to hash password: {}", e))
        })?;

        let user_id = Uuid::new_v4().to_string();

        // Insert user into database
        let result = sqlx::query(
            "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        )
        .bind(&user_id)
        .bind(&req.username)
        .bind(&req.email)
        .bind(&password_hash)
        .execute(&self.db)
        .await;

        match result {
            Ok(_) => {
                info!(
                    "User registered successfully: {} ({})",
                    req.username, user_id
                );
                Ok(Response::new(RegisterResponse {
                    success: true,
                    message: "User registered successfully".to_string(),
                    user_id,
                }))
            }
            Err(e) => {
                if e.to_string().contains("duplicate key") {
                    warn!(
                        "Registration failed: username or email already exists: {}",
                        req.username
                    );
                    Ok(Response::new(RegisterResponse {
                        success: false,
                        message: "Username or email already exists".to_string(),
                        user_id: String::new(),
                    }))
                } else {
                    error!("Database error during registration: {}", e);
                    Err(Status::internal(format!("Database error: {}", e)))
                }
            }
        }
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();

        // Fetch user from database
        let user_result = sqlx::query_as::<_, DbUser>(
            "SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE username = $1",
        )
        .bind(&req.username)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            error!("Database error during login: {}", e);
            Status::internal(format!("Database error: {}", e))
        })?;

        let user = match user_result {
            Some(u) => u,
            None => {
                warn!("Login failed: user not found: {}", req.username);
                return Ok(Response::new(LoginResponse {
                    success: false,
                    message: "Invalid username or password".to_string(),
                    token: String::new(),
                    user: None,
                }));
            }
        };

        // Verify password
        let password_valid = verify(&req.password, &user.password_hash).map_err(|e| {
            error!("Password verification error: {}", e);
            Status::internal(format!("Password verification error: {}", e))
        })?;

        if !password_valid {
            warn!("Login failed: invalid password for user: {}", req.username);
            return Ok(Response::new(LoginResponse {
                success: false,
                message: "Invalid username or password".to_string(),
                token: String::new(),
                user: None,
            }));
        }

        // Generate JWT token
        let token = self.generate_token(&user.id).map_err(|e| {
            error!("Token generation error: {}", e);
            Status::internal(format!("Token generation error: {}", e))
        })?;

        info!(
            "User logged in successfully: {} ({})",
            req.username, user.id
        );
        Ok(Response::new(LoginResponse {
            success: true,
            message: "Login successful".to_string(),
            token,
            user: Some(self.db_user_to_proto(&user)),
        }))
    }

    async fn verify(
        &self,
        request: Request<VerifyRequest>,
    ) -> Result<Response<VerifyResponse>, Status> {
        let req = request.into_inner();

        let user = self
            .get_user_profile(Request::new(GetUserProfileRequest {
                user_id: req.user_id.clone(),
            }))
            .await?;
        let user_result = user.into_inner();

        if user_result.success && user_result.user.is_some() {
            info!("User verified successfully: {}", req.user_id);
            Ok(Response::new(VerifyResponse {
                valid: true,
                user_id: user_result
                    .user
                    .as_ref()
                    .map(|u| u.user_id.clone())
                    .unwrap_or_default(),
                message: "User is valid".to_string(),
            }))
        } else {
            warn!("User verification failed: {}", req.user_id);
            Ok(Response::new(VerifyResponse {
                valid: false,
                user_id: String::new(),
                message: "Invalid user".to_string(),
            }))
        }
    }

    async fn get_user_profile(
        &self,
        request: Request<GetUserProfileRequest>,
    ) -> Result<Response<GetUserProfileResponse>, Status> {
        let req = request.into_inner();
        info!(
            "Get user profile request received for user_id: {}",
            req.user_id
        );

        let user_result = sqlx::query_as::<_, DbUser>(
            "SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE id = $1",
        )
        .bind(&req.user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            error!("Database error while fetching user profile: {}", e);
            Status::internal(format!("Database error: {}", e))
        })?;

        match user_result {
            Some(user) => {
                info!("User profile retrieved successfully: {}", req.user_id);
                Ok(Response::new(GetUserProfileResponse {
                    success: true,
                    message: "User profile retrieved successfully".to_string(),
                    user: Some(self.db_user_to_proto(&user)),
                }))
            }
            None => {
                warn!("User profile not found: {}", req.user_id);
                Ok(Response::new(GetUserProfileResponse {
                    success: false,
                    message: "User not found".to_string(),
                    user: None,
                }))
            }
        }
    }

    async fn update_user_profile(
        &self,
        request: Request<UpdateUserProfileRequest>,
    ) -> Result<Response<UpdateUserProfileResponse>, Status> {
        let req = request.into_inner();
        info!(
            "Update user profile request received for user_id: {}",
            req.user_id
        );

        // Update user in database
        let result = sqlx::query(
            "UPDATE users SET email = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(&req.email)
        .bind(&req.user_id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            error!("Database error during profile update: {}", e);
            Status::internal(format!("Database error: {}", e))
        })?;

        if result.rows_affected() == 0 {
            warn!(
                "User profile update failed: user not found: {}",
                req.user_id
            );
            return Ok(Response::new(UpdateUserProfileResponse {
                success: false,
                message: "User not found".to_string(),
                user: None,
            }));
        }

        // Fetch updated user
        let user = sqlx::query_as::<_, DbUser>(
            "SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE id = $1",
        )
        .bind(&req.user_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| {
            error!("Database error fetching updated user: {}", e);
            Status::internal(format!("Database error: {}", e))
        })?;

        info!("User profile updated successfully: {}", req.user_id);
        Ok(Response::new(UpdateUserProfileResponse {
            success: true,
            message: "User profile updated successfully".to_string(),
            user: Some(self.db_user_to_proto(&user)),
        }))
    }
}
