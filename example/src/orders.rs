use crate::proto::estore::order_service_server::OrderService;
use crate::proto::estore::{GetMyOrdersRequests, GetMyOrdersResponse, Order};
use tonic::{Request, Response, Status};

#[derive(Default)]
pub struct Orders {}

#[tonic::async_trait]
impl OrderService for Orders {
    async fn get_my_orders(
        &self,
        request: Request<GetMyOrdersRequests>,
    ) -> Result<Response<GetMyOrdersResponse>, Status> {
        // user_id that is set within request interceptor
        let user_id = request.metadata().get("user_id");
        println!("User Id {}", user_id.unwrap().to_str().unwrap());
        Ok(Response::new(GetMyOrdersResponse {
            orders: vec![
                Order {
                    id: "ord001".to_string(),
                    label: "Christmas gifts".to_string(),
                    amount: 350,
                },
                Order {
                    id: "ord002".to_string(),
                    label: "Home office equipment".to_string(),
                    amount: 1150,
                },
            ],
        }))
    }
}
