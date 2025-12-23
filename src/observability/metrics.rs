use metrics_exporter_prometheus::PrometheusBuilder;

pub fn init_metrics() {
    PrometheusBuilder::new()
        .with_http_listener(([127,0,0,1], 9898))
        .install()
        .expect("failed to install Prometheus recorder");
}
