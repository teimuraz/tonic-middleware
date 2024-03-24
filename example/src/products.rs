use crate::proto::estore::product_service_server::ProductService;
use crate::proto::estore::{ListProductsRequest, ListProductsResponse, Product};
use tonic::{Request, Response, Status};

#[derive(Default)]
pub struct Products {}

#[tonic::async_trait]
impl ProductService for Products {
    async fn list_products(
        &self,
        _request: Request<ListProductsRequest>,
    ) -> Result<Response<ListProductsResponse>, Status> {
        Ok(Response::new(ListProductsResponse {
            products: vec![
                Product {
                    id: "p001".to_string(),
                    name: "phone".to_string(),
                },
                Product {
                    id: "p002".to_string(),
                    name: "tablet".to_string(),
                },
            ],
        }))
    }
}
