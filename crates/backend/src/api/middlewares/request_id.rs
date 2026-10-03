use axum::http::{HeaderName, Request};
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use uuid::Uuid;

pub static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Clone, Copy, Debug, Default)]
pub struct LynxMakeRequestId;

impl MakeRequestId for LynxMakeRequestId {
    fn make_request_id<B>(&mut self, request: &Request<B>) -> Option<RequestId> {
        if let Some(req_id) = request
            .headers()
            .get(&X_REQUEST_ID)
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok())
        {
            return Some(RequestId::new(req_id));
        }

        let new_id = Uuid::new_v4().to_string();
        new_id.parse().ok().map(RequestId::new)
    }
}

pub fn create_request_id_layers() -> (
    SetRequestIdLayer<LynxMakeRequestId>,
    PropagateRequestIdLayer,
) {
    (
        SetRequestIdLayer::new(X_REQUEST_ID.clone(), LynxMakeRequestId),
        PropagateRequestIdLayer::new(X_REQUEST_ID.clone()),
    )
}
