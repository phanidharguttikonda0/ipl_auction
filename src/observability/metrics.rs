use metrics_exporter_prometheus::PrometheusBuilder;

pub fn init_metrics() {
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9898))
        .install()
        .expect("failed to install Prometheus recorder");
}
