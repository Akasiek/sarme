mod web;

pub async fn run() {
    dotenvy::dotenv().ok();
    web::run().await;
}
