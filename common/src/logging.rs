use http::{Request, Response};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;
use tonic::body::BoxBody;
use tower::{Layer, Service};
use tracing::{error, info};

#[derive(Clone)]
pub struct LoggingLayer;

impl<S> Layer<S> for LoggingLayer {
    type Service = LoggingService<S>;

    fn layer(&self, service: S) -> Self::Service {
        LoggingService { inner: service }
    }
}

#[derive(Clone)]
pub struct LoggingService<S> {
    inner: S,
}

impl<S> Service<Request<BoxBody>> for LoggingService<S>
where
    S: Service<Request<BoxBody>, Response = Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<BoxBody>) -> Self::Future {
        let start = Instant::now();
        let path = req.uri().path().to_owned();
        let method = req.method().clone();

        info!(
            method = %method,
            path = %path,
            "gRPC request started"
        );

        let future = self.inner.call(req);

        ResponseFuture {
            future,
            start,
            path,
            method: method.to_string(),
        }
    }
}

#[pin_project]
pub struct ResponseFuture<F> {
    #[pin]
    future: F,
    start: Instant,
    path: String,
    method: String,
}

impl<F, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<BoxBody>, E>>,
{
    type Output = Result<Response<BoxBody>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        match this.future.poll(cx) {
            Poll::Ready(result) => {
                let duration = this.start.elapsed();

                match &result {
                    Ok(response) => {
                        info!(
                            method = %this.method,
                            path = %this.path,
                            status = %response.status(),
                            duration_ms = %duration.as_millis(),
                            "gRPC request completed"
                        );
                    }
                    Err(_) => {
                        error!(
                            method = %this. method,
                            path = %this.path,
                            duration_ms = %duration.as_millis(),
                            "gRPC request failed"
                        );
                    }
                }

                Poll::Ready(result)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
