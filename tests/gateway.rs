//! Stub HTTP gateway query types demonstrating `From<HttpStatsQuery>`.

use promql_compose::{AggrMod, Expr, LabelMatcher, MatchOp, PromValue, Selector, promql};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpStatsQuery {
    pub tenant_id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub filter: HttpGatewayStatsFilter,
    pub group_by: Vec<HttpGatewayGroupBy>,
    pub metrics: Vec<GatewayMetric>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HttpGatewayStatsFilter {
    pub integration_id: Option<String>,
    pub upstream_host: Option<String>,
    pub http_method: Option<HttpMethod>,
    pub http_status: Option<String>,
    pub status_class: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpGatewayGroupBy {
    HttpMethod,
    IntegrationId,
    StatusClass,
}

impl HttpGatewayGroupBy {
    pub fn label(&self) -> &'static str {
        match self {
            HttpGatewayGroupBy::HttpMethod => "http_method",
            HttpGatewayGroupBy::IntegrationId => "integration_id",
            HttpGatewayGroupBy::StatusClass => "status_class",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl PromValue for HttpMethod {
    fn to_prom_value(&self) -> String {
        match self {
            HttpMethod::Get => "GET".to_string(),
            HttpMethod::Post => "POST".to_string(),
            HttpMethod::Put => "PUT".to_string(),
            HttpMethod::Delete => "DELETE".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayMetric {
    FetchRequests,
    ErrorResponses,
}

impl GatewayMetric {
    pub fn metric_name(&self) -> &'static str {
        match self {
            GatewayMetric::FetchRequests => "kie_http_gateway_fetch_requests",
            GatewayMetric::ErrorResponses => "kie_http_gateway_error_responses",
        }
    }

    pub fn range(&self) -> &'static str {
        "5m"
    }

    pub fn per_minute_scalar(&self) -> f64 {
        60.0
    }

    pub fn func(&self) -> &'static str {
        "rate"
    }
}

impl From<&HttpStatsQuery> for Vec<Expr> {
    fn from(query: &HttpStatsQuery) -> Self {
        let group_labels: Vec<String> = query
            .group_by
            .iter()
            .map(|group| group.label().to_string())
            .collect();

        query
            .metrics
            .iter()
            .map(|metric| {
                promql!(
                    (metric.per_minute_scalar()) * sum by (..(group_labels.clone())) (
                        (metric.func()) ( (metric.metric_name()) {
                            tenant_id = query.tenant_id,
                            project_id = query.project_id,
                            ?integration_id = query.filter.integration_id,
                            ?http_method = query.filter.http_method,
                            ?status_class = query.filter.status_class,
                        } [(metric.range())] )
                    )
                )
            })
            .collect()
    }
}

pub fn error_rate_expr(query: &HttpStatsQuery, metric: &GatewayMetric) -> Expr {
    let group_labels: Vec<String> = query
        .group_by
        .iter()
        .map(|group| group.label().to_string())
        .collect();

    promql!(
        sum by (..(group_labels.clone())) (
            (metric.func()) ( (metric.metric_name()) {
                tenant_id = query.tenant_id,
                project_id = query.project_id,
                ?integration_id = query.filter.integration_id,
                ?upstream_host = query.filter.upstream_host,
                ?http_method = query.filter.http_method,
                ?http_status = query.filter.http_status,
                result = "err",
            } [(metric.range())] )
        )
        /
        sum by (..(group_labels)) (
            (metric.func()) ( (metric.metric_name()) {
                tenant_id = query.tenant_id,
                project_id = query.project_id,
                ?integration_id = query.filter.integration_id,
                ?upstream_host = query.filter.upstream_host,
                ?http_method = query.filter.http_method,
                ?http_status = query.filter.http_status,
                ?result = query.filter.result,
            } [(metric.range())] )
        )
    )
}

/// Canonical error-rate query string used in tests.
pub const EXAMPLE_ERROR_RATE_QUERY: &str = concat!(
    "sum by (http_method) (rate(kie_http_gateway_fetch_requests{",
    "tenant_id=\"00000000-0000-4000-8000-000000000100\", ",
    "project_id=\"00000000-0000-4000-8000-000000000200\", ",
    "integration_id=\"int-1\", http_method=\"GET\", result=\"err\"}[5m]))",
    " / sum by (http_method) (rate(kie_http_gateway_fetch_requests{",
    "tenant_id=\"00000000-0000-4000-8000-000000000100\", ",
    "project_id=\"00000000-0000-4000-8000-000000000200\", ",
    "integration_id=\"int-1\", http_method=\"GET\"}[5m]))"
);

/// Canonical example query string used in tests.
pub const EXAMPLE_HTTP_STATS_QUERY: &str = concat!(
    "60 * sum by (http_method) (rate(kie_http_gateway_fetch_requests{",
    "tenant_id=\"00000000-0000-4000-8000-000000000100\", ",
    "project_id=\"00000000-0000-4000-8000-000000000200\", ",
    "integration_id=\"int-1\"}[5m]))"
);

/// Hand-built AST for [`EXAMPLE_HTTP_STATS_QUERY`].
pub fn example_http_stats_expr() -> Expr {
    Expr::Binary {
        op: "*".to_string(),
        lhs: Box::new(Expr::Scalar(60.0)),
        rhs: Box::new(Expr::Aggregation {
            op: "sum".to_string(),
            modifier: Some(AggrMod::By(vec!["http_method".to_string()])),
            arg: Box::new(Expr::Call {
                func: "rate".to_string(),
                args: vec![Expr::Selector(Selector {
                    metric: Some("kie_http_gateway_fetch_requests".to_string()),
                    matchers: vec![
                        LabelMatcher {
                            name: "tenant_id".to_string(),
                            op: MatchOp::Eq,
                            value: "00000000-0000-4000-8000-000000000100".to_string(),
                        },
                        LabelMatcher {
                            name: "project_id".to_string(),
                            op: MatchOp::Eq,
                            value: "00000000-0000-4000-8000-000000000200".to_string(),
                        },
                        LabelMatcher {
                            name: "integration_id".to_string(),
                            op: MatchOp::Eq,
                            value: "int-1".to_string(),
                        },
                    ],
                    range: Some("5m".to_string()),
                    offset: None,
                })],
            }),
        }),
    }
}

#[test]
fn from_http_stats_query_renders_example_string() {
    let query = HttpStatsQuery {
        tenant_id: uuid::Uuid::parse_str("00000000-0000-4000-8000-000000000100").unwrap(),
        project_id: uuid::Uuid::parse_str("00000000-0000-4000-8000-000000000200").unwrap(),
        filter: HttpGatewayStatsFilter {
            integration_id: Some("int-1".to_string()),
            ..Default::default()
        },
        group_by: vec![HttpGatewayGroupBy::HttpMethod],
        metrics: vec![GatewayMetric::FetchRequests],
    };

    let exprs: Vec<Expr> = (&query).into();
    assert_eq!(exprs.len(), 1);
    let expr = &exprs[0];
    assert_eq!(*expr, example_http_stats_expr());
    assert_eq!(expr.to_string(), EXAMPLE_HTTP_STATS_QUERY);
}

#[test]
fn multiple_metrics_and_group_by_labels() {
    let query = HttpStatsQuery {
        tenant_id: uuid::Uuid::nil(),
        project_id: uuid::Uuid::nil(),
        filter: HttpGatewayStatsFilter::default(),
        group_by: vec![
            HttpGatewayGroupBy::HttpMethod,
            HttpGatewayGroupBy::StatusClass,
        ],
        metrics: vec![GatewayMetric::FetchRequests, GatewayMetric::ErrorResponses],
    };

    let exprs: Vec<Expr> = (&query).into();
    assert_eq!(exprs.len(), 2);
    assert_eq!(
        exprs[0].to_string(),
        concat!(
            "60 * sum by (http_method, status_class) (rate(kie_http_gateway_fetch_requests{",
            "tenant_id=\"00000000-0000-0000-0000-000000000000\", ",
            "project_id=\"00000000-0000-0000-0000-000000000000\"}[5m]))"
        )
    );
    assert_eq!(
        exprs[1].to_string(),
        concat!(
            "60 * sum by (http_method, status_class) (rate(kie_http_gateway_error_responses{",
            "tenant_id=\"00000000-0000-0000-0000-000000000000\", ",
            "project_id=\"00000000-0000-0000-0000-000000000000\"}[5m]))"
        )
    );
}

#[test]
fn filter_with_custom_prom_value_method() {
    let query = HttpStatsQuery {
        tenant_id: uuid::Uuid::nil(),
        project_id: uuid::Uuid::nil(),
        filter: HttpGatewayStatsFilter {
            http_method: Some(HttpMethod::Post),
            ..Default::default()
        },
        group_by: vec![],
        metrics: vec![GatewayMetric::FetchRequests],
    };

    let exprs: Vec<Expr> = (&query).into();
    assert_eq!(
        exprs[0].to_string(),
        concat!(
            "60 * sum by () (rate(kie_http_gateway_fetch_requests{",
            "tenant_id=\"00000000-0000-0000-0000-000000000000\", ",
            "project_id=\"00000000-0000-0000-0000-000000000000\", ",
            "http_method=\"POST\"}[5m]))"
        )
    );
}

#[test]
fn error_rate_query_renders() {
    let query = HttpStatsQuery {
        tenant_id: uuid::Uuid::parse_str("00000000-0000-4000-8000-000000000100").unwrap(),
        project_id: uuid::Uuid::parse_str("00000000-0000-4000-8000-000000000200").unwrap(),
        filter: HttpGatewayStatsFilter {
            integration_id: Some("int-1".to_string()),
            http_method: Some(HttpMethod::Get),
            ..Default::default()
        },
        group_by: vec![HttpGatewayGroupBy::HttpMethod],
        metrics: vec![GatewayMetric::FetchRequests],
    };

    let expr = error_rate_expr(&query, &GatewayMetric::FetchRequests);
    assert_eq!(expr.to_string(), EXAMPLE_ERROR_RATE_QUERY);
}
