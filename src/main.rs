use std::path::PathBuf;

use clap::Parser;
use zero2prod::{
    settings::Settings,
    trace::{get_subscriber, init_subscriber, stdout, TraceSettings},
    Application,
};

#[derive(Debug, Parser)]
#[command(version)]
struct Args {
    /// Configuration file path
    #[arg(short = 'f')]
    configuration: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> hyper::Result<()> {
    let args = Args::parse();
    let configuration = args.configuration.as_ref().map(|c| c.to_str().unwrap());
    let settings = Settings::load(configuration).expect("failed to load configuration");

    let subscriber = get_subscriber(TraceSettings {
        level: settings.log.level,
        writer: stdout(),
        endpoint: settings.log.endpoint.as_deref(),
        namespace: settings.log.namespace.as_deref(),
    });
    init_subscriber(subscriber);

    let server = Application::build(settings)?;
    server.await
}
