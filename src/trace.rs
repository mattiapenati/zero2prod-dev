use std::net::SocketAddr;

use axum::extract::ConnectInfo;
use hyper::Request;
use once_cell::sync::Lazy;
use opentelemetry::{
    sdk::{
        trace::{self, Sampler},
        Resource,
    },
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tracing::{field::Empty, subscriber::set_global_default, Level, Subscriber};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{filter::Targets, fmt::MakeWriter, layer::SubscriberExt, Registry};

const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

static STDOUT: Lazy<(NonBlocking, WorkerGuard)> =
    Lazy::new(|| tracing_appender::non_blocking(std::io::stdout()));

pub fn stdout() -> impl for<'a> MakeWriter<'a> + Send + Sync + 'static {
    STDOUT.0.clone()
}

pub struct TraceSettings<'a, W>
where
    W: for<'b> MakeWriter<'b>,
{
    pub level: Level,
    pub writer: W,
    pub endpoint: Option<&'a str>,
    pub namespace: Option<&'a str>,
}

pub fn get_subscriber<'a, W>(settings: TraceSettings<'a, W>) -> Box<dyn Subscriber + Send + Sync>
where
    W: for<'b> MakeWriter<'b> + Send + Sync + 'static,
{
    let filter = Targets::new().with_target(PKG_NAME, settings.level);
    let formatting_layer = BunyanFormattingLayer::new(PKG_NAME.to_string(), settings.writer);

    let subscriber = Registry::default()
        .with(filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    if let Some(endpoint) = settings.endpoint {
        let resources = {
            let mut resources = vec![
                KeyValue::new("service.name", PKG_NAME),
                KeyValue::new("service.version", PKG_VERSION),
            ];
            if let Some(name) = settings.namespace {
                resources.push(KeyValue::new("service.namespace", name.to_string()))
            }
            resources
        };
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint),
            )
            .with_trace_config(
                trace::config()
                    .with_sampler(Sampler::AlwaysOn)
                    .with_resource(Resource::new(resources)),
            )
            .install_batch(opentelemetry::runtime::Tokio)
            .expect("failed to initialize tracer exporter");
        let otel = tracing_opentelemetry::layer().with_tracer(tracer);

        Box::new(subscriber.with(otel))
    } else {
        Box::new(subscriber)
    }
}

pub fn init_subscriber<S>(subscriber: S)
where
    S: Subscriber + Send + Sync,
{
    LogTracer::init().expect("failed to set log tracer");
    set_global_default(subscriber).expect("failed to set subscriber");
}

#[derive(Clone, Copy, Debug)]
pub struct MakeSpan;

impl<B> tower_http::trace::MakeSpan<B> for MakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> tracing::Span {
        let ConnectInfo(socket_addr) = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .unwrap();

        let request_id = request
            .headers()
            .get("x-request-id")
            .unwrap()
            .to_str()
            .unwrap();

        let http_target = match request.uri().path_and_query() {
            Some(http_target) => http_target.as_str(),
            None => request.uri().path(),
        };

        tracing::info_span!(
            "HTTP request",
            http.method = %request.method(),
            http.target = %http_target,
            http.status_code = Empty,
            net.sock.host.addr = %socket_addr.ip(),
            net.sock.host.port = socket_addr.port(),
            request_id = %request_id,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OnResponse;

impl<B> tower_http::trace::OnResponse<B> for OnResponse {
    fn on_response(
        self,
        response: &hyper::Response<B>,
        _latency: std::time::Duration,
        span: &tracing::Span,
    ) {
        span.record("http.status_code", response.status().as_u16());
    }
}
