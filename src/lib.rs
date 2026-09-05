mod web;

pub async fn run() {
    dotenvy::dotenv().ok();
    web::tracer::init();

    web::run().await;
}
