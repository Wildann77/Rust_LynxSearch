pub mod compression;
pub mod cors;
pub mod error_interceptor;
pub mod request_id;
pub mod timeout;
pub mod trace;

use axum::Router;
use std::time::Duration;

pub use compression::create_compression_layer;
pub use cors::create_cors_layer;
pub use error_interceptor::ensure_structured_errors;
pub use request_id::{LynxMakeRequestId, X_REQUEST_ID, create_request_id_layers};
pub use timeout::{DEFAULT_TIMEOUT_DURATION, create_timeout_layer};
pub use trace::create_trace_layer;

pub fn apply_middlewares<S>(router: Router<S>, timeout_duration: Duration) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let (set_request_id, propagate_request_id) = create_request_id_layers();
    let cors = create_cors_layer();
    let trace = create_trace_layer();
    let timeout = create_timeout_layer(timeout_duration);

    router
        .layer(timeout)
        .layer(axum::middleware::from_fn(ensure_structured_errors))
        .layer(trace)
        .layer(propagate_request_id)
        .layer(set_request_id)
        .layer(cors)
}
