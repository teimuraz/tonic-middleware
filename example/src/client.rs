pub mod proto;

use crate::proto::estore::order_service_client::OrderServiceClient;
use crate::proto::estore::product_service_client::ProductServiceClient;
use crate::proto::estore::{GetMyOrdersRequests, ListProductsRequest};
use tonic::metadata::MetadataValue;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut product_client = ProductServiceClient::connect("http://[::1]:50051").await?;
    let products_request = tonic::Request::new(ListProductsRequest {});
    let list_products_response = product_client.list_products(products_request).await?;
    println!("List products response:\n {:?}\n", list_products_response);

    let mut order_client = OrderServiceClient::connect("http://[::1]:50051").await?;

    let token: MetadataValue<_> = "supersecret".parse()?;
    let mut orders_request_authenticated = tonic::Request::new(GetMyOrdersRequests {});
    orders_request_authenticated
        .metadata_mut()
        .insert("authorization", token);
    let orders_response_authenticated = order_client
        .get_my_orders(orders_request_authenticated)
        .await?;
    println!(
        "Orders response authenticated:\n {:?}\n",
        orders_response_authenticated
    );

    let orders_request_unauthenticated = tonic::Request::new(GetMyOrdersRequests {});
    let orders_response_unauthenticated = order_client
        .get_my_orders(orders_request_unauthenticated)
        .await;
    println!(
        "Orders response unauthenticated:\n {:?}\n",
        orders_response_unauthenticated
    );

    Ok(())
}
