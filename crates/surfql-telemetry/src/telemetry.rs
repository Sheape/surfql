use std::sync::LazyLock;

use opentelemetry::{
    KeyValue, global,
    metrics::{Counter, Gauge},
};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{Resource, logs, metrics, resource::ResourceBuilder};
use surfql_core::load_config;
use tracing_subscriber::{EnvFilter, Registry, prelude::*};

const SERVICE_NAME: &str = "surfql-cc-downloader";
pub static CC_BYTES_UPLOADED_COUNTER: LazyLock<Counter<u64>> = LazyLock::new(|| {
    global::meter(SERVICE_NAME)
        .u64_counter("upload_bytes_total")
        .with_description("Total cumulative bytes uploaded")
        .with_unit("By")
        .build()
});
pub static CC_FILES_UPLOADED_COUNTER: LazyLock<Counter<u64>> = LazyLock::new(|| {
    global::meter(SERVICE_NAME)
        .u64_counter("files_uploaded_total")
        .with_description("Total number of files uploaded")
        .build()
});

pub fn init_telemetry() {
    let collector_url = load_config().OTLP_COLLECTOR_URL.as_str();
    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", "surfql-worker"),
            KeyValue::new("worker.role", "downloader"),
        ])
        .build();

    let metrics_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(collector_url)
        .build()
        .expect("Failed to build metrics exporter");

    let reader = metrics::PeriodicReader::builder(metrics_exporter)
        .with_interval(std::time::Duration::from_secs(5))
        .build();

    let meter_provider = metrics::SdkMeterProvider::builder()
        .with_resource(resource.clone())
        .with_reader(reader)
        .build();

    global::set_meter_provider(meter_provider);

    let log_exporter = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .with_endpoint(collector_url)
        .build()
        .expect("Failed to build log exporter");

    let log_provider = logs::SdkLoggerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(log_exporter)
        .build();

    // Well this is kind of necessary in order to avoid recursive callback loop error if something
    // does go off.
    let filter = EnvFilter::new("info")
        .add_directive("opentelemetry=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap());

    let otel_log_layer =
        opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&log_provider);

    let console_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_level(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
        .compact();

    Registry::default()
        .with(filter)
        .with(otel_log_layer)
        .with(console_layer)
        .init();
}
