# promql-compose

A Rust library for building PromQL queries as a typed AST, using a `macro_rules!` DSL that looks like PromQL but accepts runtime values.

Instead of concatenating query strings by hand, you describe selectors, function calls, aggregations, and scalar operations in Rust. The result is an `Expr` tree that renders to canonical PromQL via `Display`.

## Features

- **PromQL-like macro syntax** — write queries that read like PromQL
- **Typed AST** — `Expr`, `Selector`, `LabelMatcher`, and related types in `ast`
- **Runtime values** — label values come from variables via the `PromValue` trait
- **Dynamic splices** — inject arbitrary matchers and aggregation labels with `..(expr)`
- **Full expressions** — scalar binops, function calls (`rate(...)`), and aggregations (`sum by (...)`)

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
| `..(filter.matchers())` | Splice a `Vec<LabelMatcher>` into a selector |
| `(metric.metric_name())` | Dynamic metric name |
| `[(metric.range())]` | Dynamic range duration |
| `(metric.func()) (...)` | Dynamic function name |
| `sum by (..(labels)) (...)` | Splice aggregation label list |
| `(scalar) * ...` | Runtime scalar in a binary expression |

## Integrating with your query types

The integration test in `tests/gateway.rs` shows the intended pattern: define a query struct, implement `PromValue` for your value types, build filter matchers in a helper method, and use `From<&YourQuery> for Vec<Expr>` with the macro:

```rust
promql!(
    (metric.per_minute_scalar()) * sum by (..(group_labels.clone())) (
        (metric.func()) ( (metric.metric_name()) {
            tenant_id = query.tenant_id,
            project_id = query.project_id,
            ..(query.filter.matchers())
        } [(metric.range())] )
    )
)
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
