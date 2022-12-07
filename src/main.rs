use zero2prod::run;

#[tokio::main]
async fn main() -> hyper::Result<()> {
    run().await
}
