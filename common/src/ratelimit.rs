use dashmap::DashMap;
use std::future:: Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tonic::body::BoxBody;
use tower::{Layer, Service};
use http::{Request, Response, StatusCode};
use tracing::warn;

#[derive(Clone)]
pub struct RateLimitLayer {
    config: Arc<RateLimitConfig>,
}

impl RateLimitLayer {
    pub fn new(max_requests: u32, window:  Duration) -> Self {
        Self {
            config: Arc::new(RateLimitConfig {
                max_requests,
                window,
                clients: DashMap::new(),
            }),
        }
    }
}

struct RateLimitConfig {
    max_requests: u32,
    window: Duration,
    clients: DashMap<String, ClientState>,
}

struct ClientState {
    count: u32,
    window_start: Instant,
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, service: S) -> Self::Service {
        RateLimitService {
            inner: service,
            config: self.config.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    config: Arc<RateLimitConfig>,
}

impl<S> Service<Request<BoxBody>> for RateLimitService<S>
where
    S: Service<Request<BoxBody>, Response = Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<BoxBody>) -> Self::Future {
        // Extract client identifier (IP address)
        let client_id = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let now = Instant::now();
        let mut allowed = false;

        // Check rate limit
        self.config.clients
            .entry(client_id. clone())
            .and_modify(|state| {
                if now.duration_since(state. window_start) > self.config.window {
                    // Reset window
                    state.count = 1;
                    state.window_start = now;
                    allowed = true;
                } else if state.count < self.config.max_requests {
                    state.count += 1;
                    allowed = true;
                }
            })
            .or_insert_with(|| {
                allowed = true;
                ClientState {
                    count: 1,
                    window_start: now,
                }
            });

        let mut inner = self.inner.clone();

        Box::pin(async move {
            if !allowed {
                warn!("Rate limit exceeded for client: {}", client_id);
                
                // Return 429 Too Many Requests
                let response = Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .body(BoxBody::default())
                    .unwrap();
                
                return Ok(response);
            }

            inner.call(req).await
        })
    }
}