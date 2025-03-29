use crate::proto::test_services::protected_service_server::ProtectedService as GrpcProtectedService;
use crate::proto::test_services::public_service_server::PublicService as GrpcPublicService;
use crate::proto::test_services::{
    ProtectedMethodRequest, ProtectedMethodResponse, PublicMethodRequest, PublicMethodResponse,
};
use std::sync::{Arc, Mutex};
use tonic::body::Body;
use tonic::codegen::http::HeaderValue;
use tonic::{async_trait, Request, Response, Status};
use tonic_middleware::{Middleware, RequestInterceptor, ServiceBound};

pub static USER_ID_HEADER_KEY: &str = "user_id";
pub static USER_ID: &str = "user-1";
pub static AUTHORIZATION_HEADER_KEY: &str = "authorization";
pub static TOKEN: &str = "supersecret";

#[derive(Clone, Default)]
pub struct PublicService;

#[async_trait]
impl GrpcPublicService for PublicService {
    async fn public_method(
        &self,
        _request: Request<PublicMethodRequest>,
    ) -> Result<Response<PublicMethodResponse>, Status> {
        Ok(Response::new(PublicMethodResponse {
            message: "Hello Public!".to_string(),
        }))
    }
}

#[derive(Clone, Default)]
pub struct ProtectedService {}

#[async_trait]
impl GrpcProtectedService for ProtectedService {
    async fn protected_method(
        &self,
        request: Request<ProtectedMethodRequest>,
    ) -> Result<Response<ProtectedMethodResponse>, Status> {
        let user_id = request
            .metadata()
            .get(USER_ID_HEADER_KEY)
            .map(|a| a.to_str().expect("Valid user_id").to_string())
            .expect("Actions to be defined");
        Ok(Response::new(ProtectedMethodResponse {
            message: "Hello Protected!".to_string(),
            user_id,
        }))
    }
}

#[derive(Clone)]
pub struct AuthInterceptor {
    pub flow: Arc<Flow>,
}

#[async_trait]
impl RequestInterceptor for AuthInterceptor {
    async fn intercept(
        &self,
        mut req: tonic::codegen::http::Request<Body>,
    ) -> Result<tonic::codegen::http::Request<Body>, Status> {
        self.flow.add_action(Action::AuthInterceptor);
        match req
            .headers()
            .get(AUTHORIZATION_HEADER_KEY)
            .map(|v| v.to_str())
        {
            Some(Ok(token)) => {
                if token != TOKEN {
                    Err(Status::unauthenticated("Unauthenticated"))
                } else {
                    let user_id = HeaderValue::from_str(USER_ID)
                        .map_err(|_e| Status::internal("Failed set header value"))?;
                    req.headers_mut().insert(USER_ID_HEADER_KEY, user_id);

                    Ok(req)
                }
            }
            _ => Err(Status::unauthenticated("Unauthenticated")),
        }
    }
}

impl AuthInterceptor {
    pub fn new(flow: Arc<Flow>) -> Self {
        Self { flow }
    }
}

#[derive(Clone)]
pub struct Interceptor2 {
    pub flow: Arc<Flow>,
}

#[async_trait]
impl RequestInterceptor for Interceptor2 {
    async fn intercept(
        &self,
        req: tonic::codegen::http::Request<Body>,
    ) -> Result<tonic::codegen::http::Request<Body>, Status> {
        self.flow.add_action(Action::Interceptor2);
        Ok(req)
    }
}

impl Interceptor2 {
    pub fn new(flow: Arc<Flow>) -> Self {
        Self { flow }
    }
}

#[derive(Clone)]
pub struct Middleware1 {
    pub flow: Arc<Flow>,
}

#[async_trait]
impl<S> Middleware<S> for Middleware1
where
    S: ServiceBound,
    S::Future: Send,
{
    async fn call(
        &self,
        req: tonic::codegen::http::Request<Body>,
        mut service: S,
    ) -> Result<tonic::codegen::http::Response<Body>, S::Error> {
        self.flow.add_action(Action::Middleware1Before);
        let result = service.call(req).await?;
        self.flow.add_action(Action::Middleware1After);
        Ok(result)
    }
}

impl Middleware1 {
    pub fn new(flow: Arc<Flow>) -> Self {
        Self { flow }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    AuthInterceptor,
    Interceptor2,
    Middleware1Before,
    Middleware1After,
}

#[derive(Clone, Default)]
pub struct Flow {
    pub actions: Arc<Mutex<Vec<Action>>>,
}

impl Flow {
    pub fn add_action(&self, action: Action) {
        let mut actions = self.actions.lock().unwrap();
        actions.push(action);
    }

    pub fn read_actions(&self) -> Vec<Action> {
        let actions = self.actions.lock().unwrap();
        actions.clone()
    }
}
