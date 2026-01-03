mod telemetry;

pub use opentelemetry::KeyValue;
pub use telemetry::{CC_BYTES_UPLOADED_COUNTER, CC_FILES_UPLOADED_COUNTER, init_telemetry};
