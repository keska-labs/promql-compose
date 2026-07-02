//! PromQL expression builder via `macro_rules!`.
//!
//! Builds a typed [`crate::ast::Expr`] AST from PromQL-like syntax, including vector
//! selectors, function calls, aggregations, and scalar binary ops. Values in
//! label matchers are converted through the [`PromValue`] trait.
//!
//! ```
//! # use promql_compose::promql;
//! let expr = promql!(60 * sum by (http_method) (
//!     rate(http_requests_total { method = "GET" }[5m])
//! ));
//! assert!(expr.to_string().contains("sum by (http_method)"));
//! ```

/// Converts a Rust value into the raw (unquoted) content of a PromQL label value.
pub trait PromValue {
    fn to_prom_value(&self) -> String;
}

impl<T: PromValue + ?Sized> PromValue for &T {
    fn to_prom_value(&self) -> String {
        (*self).to_prom_value()
    }
}

impl PromValue for str {
    fn to_prom_value(&self) -> String {
        self.to_string()
    }
}

impl PromValue for String {
    fn to_prom_value(&self) -> String {
        self.clone()
    }
}

impl PromValue for bool {
    fn to_prom_value(&self) -> String {
        self.to_string()
    }
}

macro_rules! prom_value_int {
    ($($ty:ty),+ $(,)?) => {
        $(impl PromValue for $ty {
            fn to_prom_value(&self) -> String {
                self.to_string()
            }
        })+
    };
}

prom_value_int!(
    i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, isize, usize
);

impl PromValue for uuid::Uuid {
    fn to_prom_value(&self) -> String {
        self.to_string()
    }
}

use crate::ast::{LabelMatcher, MatchOp};

/// Append a required label matcher to `out`.
pub fn push_matcher<T: PromValue>(
    out: &mut Vec<LabelMatcher>,
    name: &str,
    op: MatchOp,
    val: &T,
) {
    out.push(LabelMatcher {
        name: name.to_string(),
        op,
        value: val.to_prom_value(),
    });
}

/// Append a label matcher when `val` is `Some`; no-op when `None`.
pub fn push_opt_matcher<T: PromValue>(
    out: &mut Vec<LabelMatcher>,
    name: &str,
    op: MatchOp,
    val: &Option<T>,
) {
    if let Some(v) = val {
        push_matcher(out, name, op, v);
    }
}

/// Parse/build a PromQL expression into an [`crate::ast::Expr`].
#[macro_export]
macro_rules! promql {
    // ---------------------------------------------------------------------
    // @expr — full expression grammar
    // ---------------------------------------------------------------------
    (@expr # ( $e:expr ) $($rest:tt)*) => {{
        let __node = $e;
        $crate::promql!(@expr_tail __node; $($rest)*)
    }};

    (@expr ( $lhs:expr ) * $($rest:tt)*) => {{
        let __lhs = $crate::ast::Expr::Scalar($lhs);
        let __rhs = $crate::promql!(@expr $($rest)*);
        $crate::ast::Expr::Binary {
            op: "*".to_string(),
            lhs: ::std::boxed::Box::new(__lhs),
            rhs: ::std::boxed::Box::new(__rhs),
        }
    }};
    (@expr ( $lhs:expr ) + $($rest:tt)*) => {{
        let __lhs = $crate::ast::Expr::Scalar($lhs);
        let __rhs = $crate::promql!(@expr $($rest)*);
        $crate::ast::Expr::Binary {
            op: "+".to_string(),
            lhs: ::std::boxed::Box::new(__lhs),
            rhs: ::std::boxed::Box::new(__rhs),
        }
    }};
    (@expr ( $lhs:expr ) - $($rest:tt)*) => {{
        let __lhs = $crate::ast::Expr::Scalar($lhs);
        let __rhs = $crate::promql!(@expr $($rest)*);
        $crate::ast::Expr::Binary {
            op: "-".to_string(),
            lhs: ::std::boxed::Box::new(__lhs),
            rhs: ::std::boxed::Box::new(__rhs),
        }
    }};
    (@expr ( $lhs:expr ) / $($rest:tt)*) => {{
        let __lhs = $crate::ast::Expr::Scalar($lhs);
        let __rhs = $crate::promql!(@expr $($rest)*);
        $crate::ast::Expr::Binary {
            op: "/".to_string(),
            lhs: ::std::boxed::Box::new(__lhs),
            rhs: ::std::boxed::Box::new(__rhs),
        }
    }};

    (@expr $lhs:literal * $($rest:tt)*) => {{
        let __lhs = $crate::ast::Expr::Scalar($lhs as f64);
        let __rhs = $crate::promql!(@expr $($rest)*);
        $crate::ast::Expr::Binary {
            op: "*".to_string(),
            lhs: ::std::boxed::Box::new(__lhs),
            rhs: ::std::boxed::Box::new(__rhs),
        }
    }};
    (@expr $lhs:literal + $($rest:tt)*) => {{
        let __lhs = $crate::ast::Expr::Scalar($lhs as f64);
        let __rhs = $crate::promql!(@expr $($rest)*);
        $crate::ast::Expr::Binary {
            op: "+".to_string(),
            lhs: ::std::boxed::Box::new(__lhs),
            rhs: ::std::boxed::Box::new(__rhs),
        }
    }};
    (@expr $lhs:literal - $($rest:tt)*) => {{
        let __lhs = $crate::ast::Expr::Scalar($lhs as f64);
        let __rhs = $crate::promql!(@expr $($rest)*);
        $crate::ast::Expr::Binary {
            op: "-".to_string(),
            lhs: ::std::boxed::Box::new(__lhs),
            rhs: ::std::boxed::Box::new(__rhs),
        }
    }};
    (@expr $lhs:literal / $($rest:tt)*) => {{
        let __lhs = $crate::ast::Expr::Scalar($lhs as f64);
        let __rhs = $crate::promql!(@expr $($rest)*);
        $crate::ast::Expr::Binary {
            op: "/".to_string(),
            lhs: ::std::boxed::Box::new(__lhs),
            rhs: ::std::boxed::Box::new(__rhs),
        }
    }};

    (@expr $op:ident by ( $($labels:tt)* ) ( $($inner:tt)* )) => {{
        let mut __labels: ::std::vec::Vec<String> = ::std::vec::Vec::new();
        $crate::promql!(@labels __labels; $($labels)*);
        $crate::ast::Expr::Aggregation {
            op: stringify!($op).to_string(),
            modifier: ::core::option::Option::Some($crate::ast::AggrMod::By(__labels)),
            arg: ::std::boxed::Box::new($crate::promql!(@expr $($inner)*)),
        }
    }};
    (@expr $op:ident without ( $($labels:tt)* ) ( $($inner:tt)* )) => {{
        let mut __labels: ::std::vec::Vec<String> = ::std::vec::Vec::new();
        $crate::promql!(@labels __labels; $($labels)*);
        $crate::ast::Expr::Aggregation {
            op: stringify!($op).to_string(),
            modifier: ::core::option::Option::Some($crate::ast::AggrMod::Without(__labels)),
            arg: ::std::boxed::Box::new($crate::promql!(@expr $($inner)*)),
        }
    }};

    (@expr ( $func:expr ) ( $($inner:tt)* )) => {{
        $crate::ast::Expr::Call {
            func: ::std::string::ToString::to_string(&$func),
            args: ::std::vec![$crate::promql!(@expr $($inner)*)],
        }
    }};

    (@expr $func:ident ( $($inner:tt)* )) => {{
        $crate::ast::Expr::Call {
            func: stringify!($func).to_string(),
            args: ::std::vec![$crate::promql!(@expr $($inner)*)],
        }
    }};

    (@expr ( $($inner:tt)+ )) => {{
        $crate::promql!(@expr $($inner)*)
    }};

    (@expr $($tokens:tt)+) => {{
        let mut __sel = $crate::ast::Selector::default();
        $crate::promql!(@parse __sel; $($tokens)*);
        $crate::ast::Expr::Selector(__sel)
    }};

    (@expr) => {{
        compile_error!("promql!: empty expression")
    }};

    (@expr_tail $node:expr;) => { $node };
    (@expr_tail $node:expr; * $($rest:tt)*) => {{
        $crate::ast::Expr::Binary {
            op: "*".to_string(),
            lhs: ::std::boxed::Box::new($node),
            rhs: ::std::boxed::Box::new($crate::promql!(@expr $($rest)*)),
        }
    }};
    (@expr_tail $node:expr; + $($rest:tt)*) => {{
        $crate::ast::Expr::Binary {
            op: "+".to_string(),
            lhs: ::std::boxed::Box::new($node),
            rhs: ::std::boxed::Box::new($crate::promql!(@expr $($rest)*)),
        }
    }};
    (@expr_tail $node:expr; - $($rest:tt)*) => {{
        $crate::ast::Expr::Binary {
            op: "-".to_string(),
            lhs: ::std::boxed::Box::new($node),
            rhs: ::std::boxed::Box::new($crate::promql!(@expr $($rest)*)),
        }
    }};
    (@expr_tail $node:expr; / $($rest:tt)*) => {{
        $crate::ast::Expr::Binary {
            op: "/".to_string(),
            lhs: ::std::boxed::Box::new($node),
            rhs: ::std::boxed::Box::new($crate::promql!(@expr $($rest)*)),
        }
    }};

    // ---------------------------------------------------------------------
    // @labels — aggregation label lists, with optional splice
    // ---------------------------------------------------------------------
    (@labels $out:ident;) => {};
    (@labels $out:ident; .. ( $e:expr ) $(, $($rest:tt)*)?) => {
        $out.extend($e);
        $( $crate::promql!(@labels $out; $($rest)*); )?
    };
    (@labels $out:ident; $label:ident $(, $($rest:tt)*)?) => {
        $out.push(stringify!($label).to_string());
        $( $crate::promql!(@labels $out; $($rest)*); )?
    };

    // ---------------------------------------------------------------------
    // @parse — metric name, brace block and following modifiers
    // ---------------------------------------------------------------------
    (@parse $sel:ident; ( $metric:expr ) { $($m:tt)* } $($rest:tt)*) => {
        $sel.metric = ::core::option::Option::Some(::std::string::ToString::to_string(&$metric));
        $crate::promql!(@matchers $sel; $($m)*);
        $crate::promql!(@mods $sel; $($rest)*);
    };
    (@parse $sel:ident; $name:ident { $($m:tt)* } $($rest:tt)*) => {
        $sel.metric = ::core::option::Option::Some(stringify!($name).to_string());
        $crate::promql!(@matchers $sel; $($m)*);
        $crate::promql!(@mods $sel; $($rest)*);
    };
    (@parse $sel:ident; { $($m:tt)* } $($rest:tt)*) => {
        $crate::promql!(@matchers $sel; $($m)*);
        $crate::promql!(@mods $sel; $($rest)*);
    };
    (@parse $sel:ident; ( $metric:expr ) $($rest:tt)*) => {
        $sel.metric = ::core::option::Option::Some(::std::string::ToString::to_string(&$metric));
        $crate::promql!(@mods $sel; $($rest)*);
    };
    (@parse $sel:ident; $name:ident $($rest:tt)*) => {
        $sel.metric = ::core::option::Option::Some(stringify!($name).to_string());
        $crate::promql!(@mods $sel; $($rest)*);
    };

    // ---------------------------------------------------------------------
    // @mods — `[range]` and `offset <duration>` modifiers
    // ---------------------------------------------------------------------
    (@mods $sel:ident;) => {};
    (@mods $sel:ident; [ ( $d:expr ) ] $($rest:tt)*) => {
        $sel.range = ::core::option::Option::Some(::std::string::ToString::to_string(&$d));
        $crate::promql!(@mods $sel; $($rest)*);
    };
    (@mods $sel:ident; [ $d:tt ] $($rest:tt)*) => {
        $sel.range = ::core::option::Option::Some(stringify!($d).to_string());
        $crate::promql!(@mods $sel; $($rest)*);
    };
    (@mods $sel:ident; offset $d:tt $($rest:tt)*) => {
        $sel.offset = ::core::option::Option::Some(stringify!($d).to_string());
        $crate::promql!(@mods $sel; $($rest)*);
    };

    // ---------------------------------------------------------------------
    // @matchers — comma separated `name <op> value` list
    // ---------------------------------------------------------------------
    (@matchers $sel:ident; $($rest:tt)*) => {
        $crate::promql!(@matchers_into &mut $sel.matchers; $($rest)*);
    };

    (@matchers_into $out:expr;) => {};
    (@matchers_into $out:expr; .. ( $e:expr ) $(, $($rest:tt)*)?) => {
        $out.extend($e);
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; ? $name:ident = ~ $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_opt_matcher(
            $out,
            stringify!($name),
            $crate::ast::MatchOp::Re,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; ? $name:ident ! ~ $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_opt_matcher(
            $out,
            stringify!($name),
            $crate::ast::MatchOp::Nre,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; ? $name:ident != $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_opt_matcher(
            $out,
            stringify!($name),
            $crate::ast::MatchOp::Ne,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; $name:ident = ~ $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_matcher(
            $out,
            stringify!($name),
            $crate::ast::MatchOp::Re,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; $name:ident ! ~ $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_matcher(
            $out,
            stringify!($name),
            $crate::ast::MatchOp::Nre,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; ? $name:ident = $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_opt_matcher(
            $out,
            stringify!($name),
            $crate::ast::MatchOp::Eq,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; $name:ident != $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_matcher(
            $out,
            stringify!($name),
            $crate::ast::MatchOp::Ne,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; $name:ident = $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_matcher(
            $out,
            stringify!($name),
            $crate::ast::MatchOp::Eq,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };

    // ---------------------------------------------------------------------
    // Public entry point (kept last)
    // ---------------------------------------------------------------------
    ($($tokens:tt)*) => {{
        $crate::promql!(@expr $($tokens)*)
    }};
}

/// Build a standalone list of label matchers.
///
/// Uses the same matcher syntax as selector braces in [`promql!`], including
/// optional matchers (`?name = expr`) and splices (`..(vec)`).
#[macro_export]
macro_rules! promql_match {
    ($($tokens:tt)*) => {{
        let mut __matchers: ::std::vec::Vec<$crate::ast::LabelMatcher> = ::std::vec::Vec::new();
        $crate::promql!(@matchers_into &mut __matchers; $($tokens)*);
        __matchers
    }};
}

#[cfg(test)]
mod tests {
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

    use crate::ast::{AggrMod, Expr, LabelMatcher, MatchOp, Selector};

    use super::PromValue;

    fn sel(expr: &Expr) -> &Selector {
        expr.as_selector().expect("expected selector expression")
    }

    #[test]
    fn parses_the_readme_example() {
        let expr = promql!(http_requests_total {
            environment =~ "staging|testing|development",
            method != "GET"
        } offset 5m);
        let sel = sel(&expr);

        assert_eq!(sel.metric.as_deref(), Some("http_requests_total"));
        assert_eq!(sel.offset.as_deref(), Some("5m"));
        assert_eq!(sel.range, None);
        assert_eq!(
            sel.matchers,
            vec![
                LabelMatcher {
                    name: "environment".to_string(),
                    op: MatchOp::Re,
                    value: "staging|testing|development".to_string(),
                },
                LabelMatcher {
                    name: "method".to_string(),
                    op: MatchOp::Ne,
                    value: "GET".to_string(),
                },
            ]
        );
    }

    #[test]
    fn all_operators() {
        let expr = promql!(m { a = "1", b != "2", c =~ "3", d !~ "4" });
        let sel = sel(&expr);
        assert_eq!(
            sel.matchers
                .iter()
                .map(|m| m.op.clone())
                .collect::<Vec<_>>(),
            vec![MatchOp::Eq, MatchOp::Ne, MatchOp::Re, MatchOp::Nre]
        );
    }

    #[test]
    fn metric_only() {
        let expr = promql!(up);
        let sel = sel(&expr);
        assert_eq!(sel.metric.as_deref(), Some("up"));
        assert!(sel.matchers.is_empty());
    }

    #[test]
    fn labels_without_metric() {
        let expr = promql!({ job = "node" });
        let sel = sel(&expr);
        assert_eq!(sel.metric, None);
        assert_eq!(sel.matchers[0].value, "node");
    }

    #[test]
    fn range_and_offset() {
        let expr = promql!(rate_me[5m] offset 1h);
        let sel = sel(&expr);
        assert_eq!(sel.metric.as_deref(), Some("rate_me"));
        assert_eq!(sel.range.as_deref(), Some("5m"));
        assert_eq!(sel.offset.as_deref(), Some("1h"));
    }

    #[test]
    fn trailing_comma() {
        let expr = promql!(x { a = "1", });
        let sel = sel(&expr);
        assert_eq!(sel.matchers.len(), 1);
    }

    #[test]
    fn serialize_hand_built_example_ast() {
        let expr = example_http_stats_expr();
        assert_eq!(expr.to_string(), EXAMPLE_HTTP_STATS_QUERY);
    }

    #[test]
    fn macro_renders_example_string() {
        let expr = promql!(
            60 * sum by (http_method) (
                rate(kie_http_gateway_fetch_requests {
                    tenant_id = "00000000-0000-4000-8000-000000000100",
                    project_id = "00000000-0000-4000-8000-000000000200",
                    integration_id = "int-1"
                }[5m])
            )
        );
        assert_eq!(expr.to_string(), EXAMPLE_HTTP_STATS_QUERY);
        assert_eq!(expr, example_http_stats_expr());
    }

    #[test]
    fn prom_value_custom_type() {
        struct Region(&'static str);
        impl PromValue for Region {
            fn to_prom_value(&self) -> String {
                self.0.to_string()
            }
        }

        let region = Region("eu-west");
        let expr = promql!(requests { region = region });
        let sel = sel(&expr);
        assert_eq!(sel.matchers[0].value, "eu-west");
    }

    #[test]
    fn matcher_splice_and_dynamic_range() {
        let extra = vec![LabelMatcher {
            name: "integration_id".to_string(),
            op: MatchOp::Eq,
            value: "int-1".to_string(),
        }];
        let range = "5m";
        let expr = promql!(metric {
            tenant_id = "t1",
            ..(extra)
        } [(range)]);
        let sel = sel(&expr);
        assert_eq!(sel.matchers.len(), 2);
        assert_eq!(sel.range.as_deref(), Some("5m"));
    }

    #[test]
    fn aggregation_label_splice() {
        let labels = vec!["http_method".to_string(), "status".to_string()];
        let expr = promql!(sum by (..(labels.clone())) (up));
        assert_eq!(
            expr,
            Expr::Aggregation {
                op: "sum".to_string(),
                modifier: Some(AggrMod::By(labels)),
                arg: Box::new(Expr::Selector(Selector {
                    metric: Some("up".to_string()),
                    ..Default::default()
                })),
            }
        );
    }

    #[test]
    fn promql_match_required_and_optional() {
        let integration_id: Option<&str> = Some("int-1");
        let http_method: Option<&str> = None;
        let matchers = promql_match!(
            tenant_id = "t1",
            ?integration_id = integration_id,
            ?http_method = http_method,
        );
        assert_eq!(matchers.len(), 2);
        assert_eq!(matchers[0].name, "tenant_id");
        assert_eq!(matchers[1].name, "integration_id");
    }

    #[test]
    fn promql_match_parity_with_selector() {
        let expr = promql!(m { a = "1", b != "2" });
        let from_match = promql_match!(a = "1", b != "2");
        assert_eq!(sel(&expr).matchers, from_match);
    }

    #[test]
    fn promql_match_splice() {
        let extra = vec![LabelMatcher {
            name: "b".to_string(),
            op: MatchOp::Eq,
            value: "2".to_string(),
        }];
        let matchers = promql_match!(a = "1", ..(extra));
        assert_eq!(matchers.len(), 2);
    }

    #[test]
    fn promql_selector_optional_matchers() {
        let integration_id: Option<&str> = Some("int-1");
        let http_method: Option<&str> = None;
        let expr = promql!(metric {
            tenant_id = "t1",
            ?integration_id = integration_id,
            ?http_method = http_method,
        });
        let sel = sel(&expr);
        assert_eq!(sel.matchers.len(), 2);
        assert_eq!(sel.matchers[0].name, "tenant_id");
        assert_eq!(sel.matchers[1].name, "integration_id");
    }

    #[test]
    fn promql_selector_optional_all_operators() {
        let a: Option<&str> = Some("1");
        let b: Option<&str> = None;
        let c: Option<&str> = Some("3");
        let d: Option<&str> = Some("4");
        let expr = promql!(m {
            ?a = a,
            ?b != b,
            ?c =~ c,
            ?d !~ d,
        });
        let sel = sel(&expr);
        assert_eq!(sel.matchers.len(), 3);
        assert_eq!(
            sel.matchers.iter().map(|m| m.op.clone()).collect::<Vec<_>>(),
            vec![MatchOp::Eq, MatchOp::Re, MatchOp::Nre]
        );
    }

    #[test]
    fn promql_selector_splice_with_optional() {
        let extra = vec![LabelMatcher {
            name: "b".to_string(),
            op: MatchOp::Eq,
            value: "2".to_string(),
        }];
        let optional: Option<&str> = Some("x");
        let expr = promql!(m {
            ..(extra),
            ?c = optional,
        });
        assert_eq!(sel(&expr).matchers.len(), 2);
    }
}
