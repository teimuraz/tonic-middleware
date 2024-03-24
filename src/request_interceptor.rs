use std::task::{Context, Poll};

use crate::ServiceBound;
use async_trait::async_trait;
use futures_util::future::BoxFuture;
use tonic::codegen::http::Request;
use tonic::codegen::Service;
use tonic::server::NamedService;
use tonic::transport::Body;
use tonic::Status;
use tower::Layer;

/// The `RequestInterceptor` trait is designed to enable the interception and processing of
/// incoming requests within your service pipeline. This trait is particularly useful for
/// performing operations such as authentication, enriching requests with additional metadata,
/// or rejecting  requests based on certain criteria before they reach the service logic.

/// If your requirements extend beyond request interception, and you need to interact with both the
/// request and response or to perform actions after the service call has been made, you should
/// consider implementing `Middleware`.

#[async_trait]
pub trait RequestInterceptor {
    /// Intercepts an incoming request, allowing for inspection, modification, or early rejection
    /// with a `Status` error.
    ///
    /// # Parameters
    ///
    /// * `req`: The incoming `Request` to be intercepted.
    ///
    /// # Returns
    ///
    /// Returns either the potentially modified request for further processing, or a `Status`
    /// error to halt processing with a specific error response.
    async fn intercept(&self, req: Request<Body>) -> Result<Request<Body>, Status>;
}

/// `InterceptorFor` wraps a service with a `RequestInterceptor`, enabling request-level
/// interception before
/// the request reaches the service logic.
/// # Type Parameters
///
/// * `S`: The service being wrapped.
/// * `I`: The `RequestInterceptor` that will preprocess the requests.
#[derive(Debug, Clone)]
pub struct InterceptorFor<S, I>
where
    I: RequestInterceptor,
{
    pub inner: S,
    pub interceptor: I,
}

impl<S, I> InterceptorFor<S, I>
where
    I: RequestInterceptor,
{
    /// Creates a new `InterceptorFor` with the provided service and interceptor.
    ///
    /// # Parameters
    ///
    /// * `inner`: The service being wrapped.
    /// * `interceptor`: The interceptor that will preprocess the requests.
    pub fn new(inner: S, interceptor: I) -> Self {
        InterceptorFor { inner, interceptor }
    }
}

impl<S, I> Service<Request<Body>> for InterceptorFor<S, I>
where
    S: ServiceBound,
    S::Future: Send,
    I: RequestInterceptor + Send + Clone + 'static + Sync,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let interceptor = self.interceptor.clone();
        let mut inner = self.inner.clone();
        Box::pin(async move {
            match interceptor.intercept(req).await {
                Ok(req) => inner.call(req).await,
                Err(status) => {
                    let response = status.to_http();
                    Ok(response)
                }
            }
        })
    }
}

impl<S, I> NamedService for InterceptorFor<S, I>
where
    S: NamedService,
    I: RequestInterceptor,
{
    const NAME: &'static str = S::NAME;
}

/// `RequestInterceptorLayer` provides a way to wrap services with a specific interceptor using the tower `Layer` trait
///
/// # Type Parameters
///
/// * `I`: The `RequestInterceptor` implementation.
#[derive(Debug, Clone)]
pub struct RequestInterceptorLayer<I> {
    interceptor: I,
}

impl<I> RequestInterceptorLayer<I> {
    /// Creates a new `RequestInterceptorLayer` with the given interceptor.
    ///
    /// # Parameters
    ///
    /// * `interceptor`: The interceptor to apply to services.
    pub fn new(interceptor: I) -> Self {
        RequestInterceptorLayer { interceptor }
    }
}

impl<S, I> Layer<S> for RequestInterceptorLayer<I>
where
    I: RequestInterceptor + Clone,
{
    type Service = InterceptorFor<S, I>;

    fn layer(&self, inner: S) -> Self::Service {
        InterceptorFor::new(inner, self.interceptor.clone())
    }
}
