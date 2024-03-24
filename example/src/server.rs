pub mod auth;
pub mod orders;
pub mod products;
pub mod proto;

use crate::auth::{AuthService, AuthServiceImpl};
use crate::orders::Orders;
use crate::products::Products;
use crate::proto::estore::order_service_server::OrderServiceServer;
use crate::proto::estore::product_service_server::ProductServiceServer;
use std::net::SocketAddr;
use std::time::Instant;
use tonic::body::BoxBody;
use tonic::codegen::http::{HeaderValue, Request, Response};
use tonic::transport::{Body, Server};
use tonic::{async_trait, Status};
use tonic_middleware::{
    InterceptorFor, Middleware, MiddlewareFor, MiddlewareLayer, RequestInterceptor,
    RequestInterceptorLayer, ServiceBound,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = "[::1]:50051".parse().unwrap();

    let auth_interceptor = AuthInterceptor {
        auth_service: AuthServiceImpl::default(),
    };

    let metrics_middleware = MetricsMiddleware::default();

    let products_service = Products::default();
    let grpc_products_service = ProductServiceServer::new(products_service);

    let orders_service = Orders::default();
    let grpc_orders_service = OrderServiceServer::new(orders_service);

    println!("Grpc server listening on {}", addr);

    Server::builder()
        // Interceptor can be added as a layer so all services will be intercepted
        // .layer(RequestInterceptorLayer::new(auth_interceptor.clone()))

        // Middleware can also be added as a layer, so it will apply to all services
        // .layer(MiddlewareLayer::new(metrics_middleware))

        // Middleware can be added to individual service
        .add_service(MiddlewareFor::new(
            grpc_products_service,
            metrics_middleware,
        ))

        // Interceptor can be added to individual service as well
        .add_service(InterceptorFor::new(grpc_orders_service, auth_interceptor))

        // Middlewares and interceptors can be combined, in any order.
        // Outermost will be executed first
        // .add_service(MiddlewareFor::new(InterceptorFor::new(grpc_orders_service.clone(), auth_interceptor.clone()), metrics_middleware.clone()))

        .serve(addr)
        .await?;
    Ok(())
}

#[derive(Clone)]
pub struct AuthInterceptor<A: AuthService> {
    pub auth_service: A,
}

#[async_trait]
impl<A: AuthService> RequestInterceptor for AuthInterceptor<A> {
    async fn intercept(&self, mut req: Request<Body>) -> Result<Request<Body>, Status> {
        match req.headers().get("authorization").map(|v| v.to_str()) {
            Some(Ok(token)) => {
                // Get user id from the token
                let user_id = self
                    .auth_service
                    .verify_token(token)
                    .await
                    .map_err(Status::unauthenticated)?;

                // Set user id in header, so it can be used in grpc services through tonic::Request::metadata()
                let user_id_header_value = HeaderValue::from_str(&user_id.to_string())
                    .map_err(|_e| Status::internal("Failed to convert user_id to header value"))?;
                req.headers_mut().insert("user_id", user_id_header_value);
                Ok(req)
            }
            _ => Err(Status::unauthenticated("Unauthenticated")),
        }
    }
}

#[derive(Default, Clone)]
pub struct MetricsMiddleware;

#[async_trait]
impl<S> Middleware<S> for MetricsMiddleware
where
    S: ServiceBound,
    S::Future: Send,
{
    async fn call(
        &self,
        req: Request<Body>,
        mut service: S,
    ) -> Result<Response<BoxBody>, S::Error> {
        let start_time = Instant::now();
        // Call the service. You can also intercept request from middleware.
        let result = service.call(req).await?;

        let elapsed_time = start_time.elapsed();
        println!("Request processed in {:?}", elapsed_time);

        Ok(result)
    }
}
