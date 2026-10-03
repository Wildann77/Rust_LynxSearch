use tower_http::compression::CompressionLayer;

pub fn create_compression_layer() -> CompressionLayer {
    CompressionLayer::new().gzip(true)
}
