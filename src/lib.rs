pub use middleware::Middleware;
pub use middleware::MiddlewareFor;
pub use middleware::MiddlewareLayer;
pub use request_interceptor::InterceptorFor;
pub use request_interceptor::RequestInterceptor;
pub use request_interceptor::RequestInterceptorLayer;

use tonic::body::BoxBody;
use tonic::codegen::http::{Request, Response};
use tonic::codegen::Service;

mod middleware;
mod request_interceptor;

pub trait ServiceBound:
    Service<Request<BoxBody>, Response = Response<BoxBody>> + Send + Clone + 'static
{
}

impl<T> ServiceBound for T where
    T: Service<Request<BoxBody>, Response = Response<BoxBody>> + Send + Clone + 'static
{
}
