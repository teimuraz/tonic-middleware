use tonic::async_trait;

#[async_trait]
pub trait AuthService: Send + Sync + 'static {
    async fn verify_token(&self, token: &str) -> Result<String, String>;
}

#[derive(Default, Clone)]
pub struct AuthServiceImpl;

#[async_trait]
impl AuthService for AuthServiceImpl {
    async fn verify_token(&self, token: &str) -> Result<String, String> {
        if token == "supersecret" {
            Ok("user-1".to_string())
        } else {
            Err("Unauthenticated".to_string())
        }
    }
}
