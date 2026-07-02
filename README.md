# promql-compose

A Rust library for building PromQL queries as a typed AST, using a `macro_rules!` DSL that looks like PromQL but accepts runtime values.

Instead of concatenating query strings by hand, you describe selectors, function calls, aggregations, and scalar operations in Rust. The result is an `Expr` tree that renders to canonical PromQL via `Display`.

## Features

- **PromQL-like macro syntax** — write queries that read like PromQL
- **Typed AST** — `Expr`, `Selector`, `LabelMatcher`, and related types in `ast`
- **Runtime values** — label values come from variables via the `PromValue` trait
- **Dynamic splices** — inject arbitrary matchers and aggregation labels with `..(expr)`
- **Optional matchers** — skip label matchers when a value is `None` with `?name = expr`
- **Standalone matcher lists** — build `Vec<LabelMatcher>` with `promql_match!`
- **Full expressions** — scalar binops, function calls (`rate(...)`, `histogram_quantile(q, ...)`), and aggregations (`sum by (...)`)

## Quick start

Add the dependency:

```toml
[dependencies]
promql-compose = "0.1"
uuid = "1"
```

Build a selector:

```rust
use promql_compose::promql;

let expr = promql!(http_requests_total {
    environment =~ "staging|testing|development",
    method != "GET"
} offset 5m);

println!("{}", expr);
// http_requests_total{environment=~"staging|testing|development", method!="GET"} offset 5m
```

Build a full expression:

```rust
use promql_compose::promql;

let expr = promql!(
    60 * sum by (http_method) (
        rate(http_requests_total { method = "GET" }[5m])
    )
);

println!("{}", expr);
// 60 * sum by (http_method) (rate(http_requests_total{method="GET"}[5m]))
```

Multi-argument function calls accept a numeric first argument (literal, `f64` variable, or parenthesized Rust expression), then a PromQL expression for the remaining argument(s). Commas inside matchers or label lists in the tail stay part of that argument:

```rust
use promql_compose::promql;

let quantile = 0.99;
let expr = promql!(histogram_quantile(
    quantile,
    sum by (le, http_method) (
        rate(http_request_duration_seconds_bucket { job = "api" }[5m])
    )
));

println!("{}", expr);
// histogram_quantile(0.99, sum by (le, http_method) (rate(...)))
```

## Runtime values with `PromValue`

Implement `PromValue` for your domain types to control how they appear in label matchers:

```rust
use promql_compose::{PromValue, promql};

enum HttpMethod { Get, Post }

impl PromValue for HttpMethod {
    fn to_prom_value(&self) -> String {
        match self {
            HttpMethod::Get => "GET".into(),
            HttpMethod::Post => "POST".into(),
        }
    }
}

let method = HttpMethod::Get;
let expr = promql!(requests { http_method = method });
```

Built-in implementations are provided for `str`, `String`, `bool`, integers, and `uuid::Uuid`.

## Dynamic parts

The macro supports runtime data beyond literals:

| Syntax | Purpose |
|--------|---------|
| `tenant_id = query.tenant_id` | Runtime label value via `PromValue` |
| `?integration_id = query.filter.integration_id` | Optional matcher — omitted when `None` |
| `..(extra_matchers)` | Splice a `Vec<LabelMatcher>` into a selector |
| `promql_match!(?a = x, b = y)` | Build a standalone `Vec<LabelMatcher>` |
| `(metric.metric_name())` | Dynamic metric name |
| `[(metric.range())]` | Dynamic range duration |
| `(metric.func()) (...)` | Dynamic function name |
| `sum by (..(labels)) (...)` | Splice aggregation label list |
| `(scalar) * ...` | Runtime scalar in a binary expression |
| `histogram_quantile(q, sum by (le) (...))` | Multi-arg function call (scalar first arg + PromQL tail) |

## Integrating with your query types

The integration test in `tests/gateway.rs` shows the intended pattern: define a query struct, implement `PromValue` for your value types, and use `From<&YourQuery> for Vec<Expr>` with optional matchers inlined in the selector:

```rust
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
```

When you need a matcher list outside a full expression, use `promql_match!`:

```rust
use promql_compose::promql_match;

let matchers = promql_match!(
    ?integration_id = filter.integration_id,
    ?http_method = filter.http_method,
);
```

This produces queries like:

```
60 * sum by (http_method) (rate(kie_http_gateway_fetch_requests{tenant_id="00000000-0000-4000-8000-000000000100", project_id="00000000-0000-4000-8000-000000000200", integration_id="int-1"}[5m]))
```

## AST types

| Type | Description |
|------|-------------|
| `Expr` | Top-level expression node (scalar, selector, call, aggregation, binary) |
| `Selector` | Vector selector with metric, matchers, range, and offset |
| `LabelMatcher` | A single `name op "value"` matcher |
| `MatchOp` | `=`, `!=`, `=~`, `!~` |
| `AggrMod` | `by (...)` or `without (...)` aggregation modifier |

Render any expression with `.to_string()` or `{}` in a format string.

## Limitations

This is a pragmatic builder, not a full PromQL parser:

- Operator precedence is intentionally limited (scalar factor → aggregation → call → selector)
- Multi-arg comma syntax splits on the first comma only (scalar + PromQL tail, or all-literal scalars)
- A bare identifier in a call argument position is treated as a Rust `f64` variable, not a PromQL metric name
- No `@` timestamp modifier, `bool` modifier, or subquery support yet

## Development

```bash
cargo test
```

The test suite includes:

- Selector and expression macro tests in `src/promql.rs`
- Serialize tests (hand-built AST → query string)
- Gateway integration tests in `tests/gateway.rs`

## License

MIT OR Apache-2.0
