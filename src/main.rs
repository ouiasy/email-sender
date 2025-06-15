use email_sender::errors;
use email_sender::run;

#[tokio::main]
async fn main() -> Result<(), errors::AppError> {
    run().await
}
