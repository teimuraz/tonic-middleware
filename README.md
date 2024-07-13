# tonic-middleware

[![Crates.io](https://img.shields.io/crates/v/tonic-middleware)](https://crates.io/crates/tonic-middleware)
[![Documentation](https://docs.rs/tonic-middleware/badge.svg)](https://docs.rs/tonic-middleware/latest/tonic_middleware)
[![Crates.io](https://img.shields.io/crates/l/tonic-middleware)](LICENSE)

## Table of Contents
- [Introduction](#introduction)
- [Tonic versions compatability](#tonic-versions-compatability)
- [Usage](#usage)
  - [Define request interceptor and middleware](#define-our-request-interceptor-and-middleware)
  - [Apply request interceptor to individual service](#apply-request-interceptor-to-individual-service)
  - [Apply request interceptor to all services using layer](#apply-request-interceptor-to-all-services-using-layer)
  - [Apply middleware to individual services](#apply-middleware-to-individual-services)
  - [Apply middleware to all services through layer](#apply-middleware-to-all-services-through-layer)
  - [Combine interceptor and middleware for individual services](#combine-interceptor-and-middleware-for-individual-services)
  - [Apply interceptor and middleware to all services through layer](#apply-interceptor-and-middleware-to-all-services-through-layer)
  - Full [example](https://github.com/teimuraz/tonic-middleware/tree/main/example) or check [integration tests](https://github.com/teimuraz/tonic-middleware/blob/main/integration_tests/tests/tests.rs)
- [Motivation](#motivation)

# Introduction

`tonic-middleware` is a Rust library that extends [tonic](https://github.com/hyperium/tonic)-based [gRPC](https://grpc.io/) services, 
enabling **asynchronous** inspection and modification and potentially rejecting of incoming requests.
It also enables the addition of custom logic through middleware, both before and after the actual service call.

The library provides two key tools:

- **Request Interceptor**

  The `RequestInterceptor` trait is designed to enable the interception and processing of
  incoming requests within your service pipeline. This trait is particularly useful for
  performing operations such as authentication, enriching requests with additional metadata,
  or rejecting  requests based on certain criteria before they reach the service logic.


- **Middleware**
 
  If your requirements extend beyond request interception, and you need to interact with both the
 request and response or to perform actions after the service call has been made, you should
 consider implementing `Middleware`.  

Both interceptors and middlewares can be applied to individual service, or to all services
through Tonic's layer.

## Tonic versions compatability

| tonic version | tonic-middleware version | Notes                                                                                                                                                                     |
|---------------|--------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 0.11          | 0.14                     |                                                                                                                                                                           |
| 0.12          | 0.2.0                    | Breaking changes <br/> resulting from breaking changes in tonic. <br/>See [changelog](https://github.com/teimuraz/tonic-middleware/releases/tag/v0.2.0) for more details. |


## Usage

Add to Cargo.toml
```
tonic-middleware = "0.2.0"
```

See full [example](https://github.com/teimuraz/tonic-middleware/tree/main/example) or check [integration tests](https://github.com/teimuraz/tonic-middleware/blob/main/integration_tests/tests/tests.rs)


### Define our request interceptor and middleware

#### To create request interceptor, we need to implement `RequestInterceptor` trait from the library.
### Note:
> Please use `tonic::codegen::http::{Request, Response}` (which are just re-exported from `http` crate by tonic)
> instead of `tonic::{Request, Response}` in  interceptors and middlewares.


Simple request interceptor that uses some custom `AuthService` injected in to perform authentication.
We need to implement `RequestInterceptor` for our custom (`AuthInterceptor`) intercept.
```rust
use tonic::codegen::http::Request; // Use this instead of tonic::Request in Interceptor!
use tonic::codegen::http::Respons; // Use this instead of tonic::Response in Interceptor!
...

#[derive(Clone)]
pub struct AuthInterceptor<A: AuthService> {
    pub auth_service: A,
}

#[async_trait]
impl<A: AuthService> RequestInterceptor for AuthInterceptor<A> {
    async fn intercept(&self, mut req: Request<BoxBody>) -> Result<Request<BoxBody>, Status> {
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
```

#### To create middleware, we need to implement 'Middleware' trait from the library.

Metrics middleware that measures request time and output to stdout.
We need to implement `Middleware` for our custom (`MetricsMiddleware`) middleware.
```rust
use tonic::codegen::http::Request; // Use this instead of tonic::Request in Middleware!
use tonic::codegen::http::Response; // Use this instead of tonic::Response in Middleware!
...

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

```

### Apply request interceptor to individual service
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = "[::1]:50051".parse().unwrap();

    let auth_interceptor = AuthInterceptor {
        auth_service: AuthServiceImpl::default(),
    };

    // Grpc service
    let products_service = Products::default();
    let grpc_products_service = ProductServiceServer::new(products_service);

    // Grpc service
    let orders_service = Orders::default();
    let grpc_orders_service = OrderServiceServer::new(orders_service);

    println!("Grpc server listening on {}", addr);

    Server::builder()
        // No interceptor applied
        .add_service(grpc_products_service)
        // Added interceptor to single service
        .add_service(InterceptorFor::new(grpc_orders_service, auth_interceptor))
        .serve(addr)
        .await?;
 // ...
}
```

### Apply request interceptor to all services using layer
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // ...
    Server::builder()
        // Interceptor can be added as a layer so all services will be intercepted
        .layer(RequestInterceptorLayer::new(auth_interceptor.clone()))
        .add_service(grpc_products_service)
        .add_service(grpc_orders_service)
        .serve(addr)
        .await?;
    // ...
}
```

### Apply middleware to individual services
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

 // ...
 Server::builder()
         // Middleware can be added to individual service
         .add_service(MiddlewareFor::new(
            grpc_products_service,
            metrics_middleware,
         ))
         // No middleware applied
         .add_service(grpc_orders_service)

         .serve(addr)
         .await?;
 // ...
}
```

### Apply middleware to all services through layer
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

 // ...
 Server::builder()
         // Middleware can also be added as a layer, so it will apply to 
         // all services
         .layer(MiddlewareLayer::new(metrics_middleware))
         
         .add_service(grpc_products_service)
         .add_service(grpc_orders_service)
         .serve(addr)
         .await?;
 // ...
}
```

### Combine interceptor and middleware for individual services

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // ...
    Server::builder()
        // Middlewares and interceptors can be combined, in any order.
        // Outermost will be executed first
        .add_service(
            MiddlewareFor::new(
                InterceptorFor::new(grpc_orders_service.clone(), auth_interceptor.clone()),
                metrics_middleware.clone(),
            ))
        .add_service(grpc_products_service)    
        .await?;
    // ...
}
```

### Apply interceptor and middleware to all services through layer
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

 // ...
 Server::builder()
         // Interceptor can be added as a layer so all services will be intercepted
         .layer(RequestInterceptorLayer::new(auth_interceptor.clone()))
         // Middleware can also be added as a layer, so it will apply to all services
         .layer(MiddlewareLayer::new(metrics_middleware))
         
         .add_service(grpc_products_service)
         .add_service(grpc_orders_service)
         .await?;
 // ...
}
```



## Motivation
Tonic provides a solid foundation for developing gRPC services in Rust, and while it offers a range of features, extending it with asynchronous interceptors and middleware requires a bit more effort. That's where `tonic-middleware` comes in,
this library simplifies adding custom asynchronous processing to the [tonic](https://github.com/hyperium/tonic) service stack.