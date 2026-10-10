use axum::extract::Request;
use axum::response::Response;
use std::time::Duration;
use tower_http::classify::{ServerErrorsAsFailures, ServerErrorsFailureClass, SharedClassifier};
use tower_http::trace::TraceLayer;
use tracing::{Span, info_span};

#[allow(clippy::type_complexity)]
pub fn create_trace_layer() -> TraceLayer<
    SharedClassifier<ServerErrorsAsFailures>,
    impl Fn(&Request) -> Span + Clone,
    impl Fn(&Request, &Span) + Clone,
    impl Fn(&Response, Duration, &Span) + Clone,
    tower_http::trace::DefaultOnBodyChunk,
    tower_http::trace::DefaultOnEos,
    impl Fn(ServerErrorsFailureClass, Duration, &Span) + Clone,
> {
    TraceLayer::new_for_http()
        .make_span_with(|request: &Request| {
            let req_id = request
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");

            info_span!(
                "http_request",
                request_id = %req_id,
                method = %request.method(),
                uri = %request.uri().path(),
                status_code = tracing::field::Empty,
                took_ms = tracing::field::Empty,
                latency_ms = tracing::field::Empty,
            )
        })
        .on_request(|_request: &Request, _span: &Span| {
            tracing::debug!("Started processing HTTP request");
        })
        .on_response(|response: &Response, latency: Duration, span: &Span| {
            let ms = latency.as_millis() as u64;
            let status = response.status().as_u16();
            span.record("status_code", status);
            span.record("took_ms", ms);
            span.record("latency_ms", ms);
            tracing::info!(
                status = status,
                took_ms = ms,
                latency_ms = ms,
                "Finished processing HTTP request"
            );
        })
        .on_failure(
            |error: ServerErrorsFailureClass, latency: Duration, span: &Span| {
                let ms = latency.as_millis() as u64;
                span.record("took_ms", ms);
                span.record("latency_ms", ms);
                tracing::error!(
                    took_ms = ms,
                    latency_ms = ms,
                    error = %error,
                    "HTTP request failed"
                );
            },
        )
}
