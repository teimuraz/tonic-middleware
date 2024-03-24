use integration_tests::proto::test_services::protected_service_client::ProtectedServiceClient;
use integration_tests::proto::test_services::protected_service_server::ProtectedServiceServer;
use integration_tests::proto::test_services::public_service_client::PublicServiceClient;
use integration_tests::proto::test_services::public_service_server::PublicServiceServer;
use integration_tests::proto::test_services::{ProtectedMethodRequest, PublicMethodRequest};
use integration_tests::services::{
    AuthInterceptor, Flow, Interceptor2, Middleware1, ProtectedService, PublicService,
    AUTHORIZATION_HEADER_KEY, TOKEN,
};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, Endpoint};
use tonic::Request;

pub static GRPC_SERVER_ADDRS: &str = "[::1]:50051";

pub fn grpc_server_addr() -> String {
    "[::1]:50051".to_string()
}

pub fn grpc_client_connection_url() -> String {
    format!("http://{}", grpc_server_addr())
}

#[derive(Clone)]
pub struct Services {
    pub public_server: Arc<PublicServiceServer<PublicService>>,
    pub public_service_client: Arc<PublicServiceClient<Channel>>,
    pub protected_server: Arc<ProtectedServiceServer<ProtectedService>>,
    pub protected_service_client: Arc<ProtectedServiceClient<Channel>>,
    pub auth_interceptor: Arc<AuthInterceptor>,
    pub interceptor2: Arc<Interceptor2>,
    pub middleware1: Arc<Middleware1>,
    pub flow: Arc<Flow>,
    pub channel: Arc<Channel>,
}

impl Services {
    pub fn new() -> Self {
        let flow = Arc::new(Flow::default());
        let channel = Arc::new(
            Endpoint::from_str(grpc_client_connection_url().as_str())
                .expect("endpoint")
                .connect_lazy(),
        );
        Self {
            public_server: Arc::new(PublicServiceServer::new(PublicService::default())),
            public_service_client: Arc::new(PublicServiceClient::new(channel.as_ref().clone())),
            protected_server: Arc::new(ProtectedServiceServer::new(ProtectedService::default())),
            protected_service_client: Arc::new(ProtectedServiceClient::new(
                channel.as_ref().clone(),
            )),
            auth_interceptor: Arc::new(AuthInterceptor::new(flow.clone())),
            interceptor2: Arc::new(Interceptor2::new(flow.clone())),
            middleware1: Arc::new(Middleware1::new(flow.clone())),
            flow,
            channel,
        }
    }
}

pub fn mk_protected_request() -> Request<ProtectedMethodRequest> {
    let mut request = Request::new(ProtectedMethodRequest {
        message: "Hello!".to_string(),
    });
    let token: MetadataValue<_> = TOKEN.parse().expect("token");
    request
        .metadata_mut()
        .insert(AUTHORIZATION_HEADER_KEY, token);

    request
}

pub fn mk_public_request() -> Request<PublicMethodRequest> {
    Request::new(PublicMethodRequest {
        message: "Hello!".to_string(),
    })
}

pub async fn sleep() {
    tokio::time::sleep(Duration::from_millis(100)).await;
}
