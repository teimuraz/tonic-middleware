pub mod common;

use integration_tests::proto;

use crate::common::{grpc_server_addr, mk_protected_request, mk_public_request, sleep, Services};
use crate::proto::test_services::ProtectedMethodRequest;
use integration_tests::services::{Action, USER_ID};
use serial_test::serial;
use tokio::sync::oneshot;
use tonic::transport::Server;
use tonic::Code;
use tonic_middleware::{InterceptorFor, MiddlewareFor, MiddlewareLayer, RequestInterceptorLayer};

#[tokio::test]
#[serial]
async fn test_interceptor_applies_to_individual_service_rejecting_request() {
    let services = Services::new();
    let public_server = services.public_server.as_ref().clone();
    let protected_server = services.protected_server.as_ref().clone();
    let auth_interceptor = services.auth_interceptor.as_ref().clone();
    let flow = services.flow;

    let (tx, rx) = oneshot::channel();
    let jh = tokio::spawn(async move {
        Server::builder()
            .add_service(public_server)
            .add_service(InterceptorFor::new(protected_server, auth_interceptor))
            .serve_with_shutdown(grpc_server_addr().parse().unwrap(), async {
                drop(rx.await)
            })
            .await
            .unwrap()
    });

    sleep().await;

    let mut public_service_client = services.public_service_client.as_ref().clone();
    public_service_client
        .public_method(mk_public_request())
        .await
        .expect("Public method response");

    let result = services
        .protected_service_client
        .as_ref()
        .clone()
        .protected_method(ProtectedMethodRequest {
            message: "Hello!".to_string(),
        })
        .await;

    assert!(result.is_err_and(|e| e.code() == Code::Unauthenticated));

    let actions: Vec<Action> = flow.read_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::AuthInterceptor);

    tx.send(()).unwrap();
    jh.await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_interceptor_applies_to_individual_service_and_sets_request_header() {
    let services = Services::new();
    let public_server = services.public_server.as_ref().clone();
    let protected_server = services.protected_server.as_ref().clone();
    let auth_interceptor = services.auth_interceptor.as_ref().clone();
    let flow = services.flow;

    let (tx, rx) = oneshot::channel();
    let jh = tokio::spawn(async move {
        Server::builder()
            .add_service(public_server)
            .add_service(InterceptorFor::new(protected_server, auth_interceptor))
            .serve_with_shutdown(grpc_server_addr().parse().unwrap(), async {
                drop(rx.await)
            })
            .await
            .unwrap()
    });

    let mut protected_service_client = services.protected_service_client.as_ref().clone();

    sleep().await;

    let request = mk_protected_request();
    let result = protected_service_client
        .protected_method(request)
        .await
        .expect("Method response");

    assert_eq!(result.get_ref().user_id, USER_ID);

    let actions: Vec<Action> = flow.read_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::AuthInterceptor);

    tx.send(()).unwrap();
    jh.await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_interceptor_applies_to_all_services() {
    let services = Services::new();
    let public_server = services.public_server.as_ref().clone();
    let protected_server = services.protected_server.as_ref().clone();
    let auth_interceptor = services.auth_interceptor.as_ref().clone();
    let interceptor2 = services.interceptor2.as_ref().clone();
    let flow = services.flow;

    let (tx, rx) = oneshot::channel();
    let jh = tokio::spawn(async move {
        Server::builder()
            .layer(RequestInterceptorLayer::new(interceptor2))
            .add_service(public_server)
            .add_service(InterceptorFor::new(protected_server, auth_interceptor))
            .serve_with_shutdown(grpc_server_addr().parse().unwrap(), async {
                drop(rx.await)
            })
            .await
            .unwrap()
    });

    let mut protected_service_client = services.protected_service_client.as_ref().clone();

    sleep().await;

    let request = mk_protected_request();
    let result = protected_service_client
        .protected_method(request)
        .await
        .expect("Method response");

    assert_eq!(result.get_ref().user_id, USER_ID);

    let mut public_service_client = services.public_service_client.as_ref().clone();
    public_service_client
        .public_method(mk_public_request())
        .await
        .expect("Public method response");

    let actions: Vec<Action> = flow.read_actions();
    assert_eq!(actions.len(), 3);
    // protected_service_client call -> interceptor2 through layer
    assert_eq!(actions[0], Action::Interceptor2);
    // protected_service_client call -> auth_interceptor through layer
    assert_eq!(actions[1], Action::AuthInterceptor);
    // public_service_client call -> interceptor2 through layer
    assert_eq!(actions[2], Action::Interceptor2);

    tx.send(()).unwrap();
    jh.await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_interceptors_can_be_combined() {
    let services = Services::new();
    let public_server = services.public_server.as_ref().clone();
    let protected_server = services.protected_server.as_ref().clone();
    let auth_interceptor = services.auth_interceptor.as_ref().clone();
    let interceptor2 = services.interceptor2.as_ref().clone();
    let flow = services.flow;

    let (tx, rx) = oneshot::channel();
    let jh = tokio::spawn(async move {
        Server::builder()
            .add_service(public_server)
            .add_service(InterceptorFor::new(
                InterceptorFor::new(protected_server, auth_interceptor),
                interceptor2,
            ))
            .serve_with_shutdown(grpc_server_addr().parse().unwrap(), async {
                drop(rx.await)
            })
            .await
            .unwrap()
    });

    let mut protected_service_client = services.protected_service_client.as_ref().clone();

    sleep().await;

    let request = mk_protected_request();
    let result = protected_service_client
        .protected_method(request)
        .await
        .expect("Method response");

    assert_eq!(result.get_ref().user_id, USER_ID);

    let actions: Vec<Action> = flow.read_actions();
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0], Action::Interceptor2);
    assert_eq!(actions[1], Action::AuthInterceptor);

    tx.send(()).unwrap();
    jh.await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_middleware_applied_to_individual_service() {
    let services = Services::new();
    let public_server = services.public_server.as_ref().clone();
    let protected_server = services.protected_server.as_ref().clone();
    let middleware1 = services.middleware1.as_ref().clone();
    let auth_interceptor = services.auth_interceptor.as_ref().clone();
    let flow = services.flow;

    let (tx, rx) = oneshot::channel();
    let jh = tokio::spawn(async move {
        Server::builder()
            .add_service(MiddlewareFor::new(public_server, middleware1))
            .add_service(InterceptorFor::new(protected_server, auth_interceptor))
            .serve_with_shutdown(grpc_server_addr().parse().unwrap(), async {
                drop(rx.await)
            })
            .await
            .unwrap()
    });

    let mut protected_service_client = services.protected_service_client.as_ref().clone();

    sleep().await;

    let request = mk_protected_request();
    let result = protected_service_client
        .protected_method(request)
        .await
        .expect("Method response");

    assert_eq!(result.get_ref().user_id, USER_ID);

    services
        .public_service_client
        .as_ref()
        .clone()
        .public_method(mk_public_request())
        .await
        .expect("Method response");

    let actions: Vec<Action> = flow.read_actions();
    assert_eq!(actions.len(), 3);
    assert_eq!(actions[0], Action::AuthInterceptor);
    assert_eq!(actions[1], Action::Middleware1Before);
    assert_eq!(actions[2], Action::Middleware1After);

    tx.send(()).unwrap();
    jh.await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_middleware_applied_to_all_services_through_layer() {
    let services = Services::new();
    let public_server = services.public_server.as_ref().clone();
    let protected_server = services.protected_server.as_ref().clone();
    let middleware1 = services.middleware1.as_ref().clone();
    let auth_interceptor = services.auth_interceptor.as_ref().clone();
    let flow = services.flow;

    let (tx, rx) = oneshot::channel();
    let jh = tokio::spawn(async move {
        Server::builder()
            .layer(MiddlewareLayer::new(middleware1))
            .add_service(public_server)
            .add_service(InterceptorFor::new(protected_server, auth_interceptor))
            .serve_with_shutdown(grpc_server_addr().parse().unwrap(), async {
                drop(rx.await)
            })
            .await
            .unwrap()
    });

    let mut protected_service_client = services.protected_service_client.as_ref().clone();

    sleep().await;

    let request = mk_protected_request();
    let result = protected_service_client
        .protected_method(request)
        .await
        .expect("Method response");

    assert_eq!(result.get_ref().user_id, USER_ID);

    services
        .public_service_client
        .as_ref()
        .clone()
        .public_method(mk_public_request())
        .await
        .expect("Method response");

    let actions: Vec<Action> = flow.read_actions();
    assert_eq!(actions.len(), 5);
    assert_eq!(actions[0], Action::Middleware1Before);
    assert_eq!(actions[1], Action::AuthInterceptor);
    assert_eq!(actions[2], Action::Middleware1After);
    assert_eq!(actions[3], Action::Middleware1Before);
    assert_eq!(actions[4], Action::Middleware1After);

    tx.send(()).unwrap();
    jh.await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_interceptor_and_middleware_combined_for_individual_service() {
    let services = Services::new();
    let public_server = services.public_server.as_ref().clone();
    let protected_server = services.protected_server.as_ref().clone();
    let middleware1 = services.middleware1.as_ref().clone();
    let auth_interceptor = services.auth_interceptor.as_ref().clone();
    let flow = services.flow;

    let (tx, rx) = oneshot::channel();
    let jh = tokio::spawn(async move {
        Server::builder()
            .add_service(public_server)
            .add_service(MiddlewareFor::new(
                InterceptorFor::new(protected_server, auth_interceptor),
                middleware1,
            ))
            .serve_with_shutdown(grpc_server_addr().parse().unwrap(), async {
                drop(rx.await)
            })
            .await
            .unwrap()
    });

    let mut protected_service_client = services.protected_service_client.as_ref().clone();

    sleep().await;

    let request = mk_protected_request();
    let result = protected_service_client
        .protected_method(request)
        .await
        .expect("Method response");

    assert_eq!(result.get_ref().user_id, USER_ID);

    services
        .public_service_client
        .as_ref()
        .clone()
        .public_method(mk_public_request())
        .await
        .expect("Method response");

    let actions: Vec<Action> = flow.read_actions();
    assert_eq!(actions.len(), 5);
    assert_eq!(actions[0], Action::Middleware1Before);
    assert_eq!(actions[1], Action::AuthInterceptor);
    assert_eq!(actions[2], Action::Middleware1After);

    tx.send(()).unwrap();
    jh.await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_interceptor_and_middleware_combined_for_all_services_through_layer() {
    let services = Services::new();
    let public_server = services.public_server.as_ref().clone();
    let protected_server = services.protected_server.as_ref().clone();
    let middleware1 = services.middleware1.as_ref().clone();
    let auth_interceptor = services.auth_interceptor.as_ref().clone();
    let interceptor2 = services.interceptor2.as_ref().clone();
    let flow = services.flow;

    let (tx, rx) = oneshot::channel();
    let jh = tokio::spawn(async move {
        Server::builder()
            .layer(MiddlewareLayer::new(middleware1))
            .layer(RequestInterceptorLayer::new(interceptor2))
            .add_service(public_server)
            .add_service(InterceptorFor::new(protected_server, auth_interceptor))
            .serve_with_shutdown(grpc_server_addr().parse().unwrap(), async {
                drop(rx.await)
            })
            .await
            .unwrap()
    });

    let mut protected_service_client = services.protected_service_client.as_ref().clone();

    sleep().await;

    let request = mk_protected_request();
    protected_service_client
        .protected_method(request)
        .await
        .expect("Method response");

    services
        .public_service_client
        .as_ref()
        .clone()
        .public_method(mk_public_request())
        .await
        .expect("Method response");

    let actions: Vec<Action> = flow.read_actions();
    assert_eq!(actions.len(), 7);
    assert_eq!(actions[0], Action::Middleware1Before);
    assert_eq!(actions[1], Action::Interceptor2);
    assert_eq!(actions[2], Action::AuthInterceptor);
    assert_eq!(actions[3], Action::Middleware1After);
    assert_eq!(actions[4], Action::Middleware1Before);
    assert_eq!(actions[5], Action::Interceptor2);
    assert_eq!(actions[6], Action::Middleware1After);

    tx.send(()).unwrap();
    jh.await.unwrap();
}
