use std::task::{Context, Poll};

use crate::ServiceBound;
use async_trait::async_trait;
use futures_util::future::BoxFuture;
use tonic::body::BoxBody;
use tonic::codegen::http::Request;
use tonic::codegen::http::Response;
use tonic::codegen::Service;
use tonic::server::NamedService;
use tonic::transport::Body;
use tower::Layer;

/// The `Middleware` trait defines a generic interface for middleware components
/// in a grpc service chain.
/// Implementors of this trait can modify, observe, or otherwise interact with requests and
/// responses in the service pipeline
///
/// If you need just intercept requests, pls can [RequestInterceptor]
///
/// # Type Parameters
///
/// * `S`: A service bound that defines the requirements for the service being wrapped by
/// the middleware.
///
/// See [examples on GitHub](https://github.com/teimuraz/tonic-middleware/tree/main/example)
#[async_trait]
pub trait Middleware<S>
where
    S: ServiceBound,
{
    /// Processes an incoming request and forwards it to the given service.
    ///
    /// Implementations may perform operations before or after forwarding the request,
    /// such as logging, metrics collection, or request modification.
    ///
    /// # Parameters
    ///
    /// * `req`: The incoming request to process.
    /// * `service`: The service to forward the processed request to.
    ///
    /// # Returns
    ///
    /// A `Result` containing the response from the service or an error if one occurred
    /// during processing.
    async fn call(&self, req: Request<Body>, service: S) -> Result<Response<BoxBody>, S::Error>;
}

/// `MiddlewareFor` is a service wrapper that pairs a middleware with its target service.
///
/// # Type Parameters
///
/// * `S`: The service that this middleware is wrapping.
/// * `M`: The middleware that is being applied to the service.
#[derive(Clone)]
pub struct MiddlewareFor<S, M>
where
    S: ServiceBound,
    M: Middleware<S>,
{
    pub inner: S,
    pub middleware: M,
}

impl<S, M> MiddlewareFor<S, M>
where
    S: ServiceBound,
    M: Middleware<S>,
{
    /// Constructs a new `MiddlewareFor` with the given service and middleware.
    ///
    /// # Parameters
    ///
    /// * `inner`: The service that this middleware is wrapping.
    /// * `middleware`: The middleware that is being applied to the service.
    pub fn new(inner: S, middleware: M) -> Self {
        MiddlewareFor { inner, middleware }
    }
}

impl<S, M> Service<Request<Body>> for MiddlewareFor<S, M>
where
    S: ServiceBound,
    S::Future: Send,
    M: Middleware<S> + Send + Clone + 'static + Sync,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let middleware = self.middleware.clone();
        let inner = self.inner.clone();
        Box::pin(async move { middleware.call(req, inner).await })
    }
}

impl<S, M> NamedService for MiddlewareFor<S, M>
where
    S: NamedService + ServiceBound,
    M: Middleware<S>,
{
    const NAME: &'static str = S::NAME;
}

/// `MiddlewareLayer` provides a way to wrap services with a specific middleware using
/// the tower `Layer` trait
#[derive(Clone)]
pub struct MiddlewareLayer<M> {
    middleware: M,
}

impl<M> MiddlewareLayer<M> {
    /// Creates a new `MiddlewareLayer` with the given middleware.
    ///
    /// # Parameters
    ///
    /// * `middleware`: The middleware to apply to services.
    pub fn new(middleware: M) -> Self {
        MiddlewareLayer { middleware }
    }
}

impl<S, M> Layer<S> for MiddlewareLayer<M>
where
    S: ServiceBound,
    M: Middleware<S> + Clone,
{
    type Service = MiddlewareFor<S, M>;

    fn layer(&self, inner: S) -> Self::Service {
        MiddlewareFor::new(inner, self.middleware.clone())
    }
}
