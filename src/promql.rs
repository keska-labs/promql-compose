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
//! assert_eq!(expr.to_string(), "60 * sum by (http_method) (rate(http_requests_total{method=\"GET\"}[5m]))");
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

use crate::ast::{Expr, LabelMatcher, MatchOp};

/// Build a scalar [`Expr`] from a numeric Rust value (e.g. a call argument).
pub fn scalar_arg(v: impl Into<f64>) -> Expr {
    Expr::Scalar(v.into())
}

/// Append a required label matcher to `out`.
pub fn push_matcher<T: PromValue>(out: &mut Vec<LabelMatcher>, name: &str, op: MatchOp, val: &T) {
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

    (@expr ( $($inner:tt)+ )) => {{
        $crate::promql!(@expr $($inner)*)
    }};

    (@expr $($tokens:tt)*) => {{
        $crate::promql!(@expr_unit $($tokens)*)
    }};

    // ---------------------------------------------------------------------
    // @expr_unit — primary expressions, then optional binop tail
    // ---------------------------------------------------------------------
    (@expr_unit $op:ident by ( $($labels:tt)* ) ( $($inner:tt)* ) $($rest:tt)*) => {{
        let mut __labels: ::std::vec::Vec<String> = ::std::vec::Vec::new();
        $crate::promql!(@labels __labels; $($labels)*);
        let __node = $crate::ast::Expr::Aggregation {
            op: stringify!($op).to_string(),
            modifier: ::core::option::Option::Some($crate::ast::AggrMod::By(__labels)),
            arg: ::std::boxed::Box::new($crate::promql!(@expr $($inner)*)),
        };
        $crate::promql!(@expr_tail __node; $($rest)*)
    }};
    (@expr_unit $op:ident without ( $($labels:tt)* ) ( $($inner:tt)* ) $($rest:tt)*) => {{
        let mut __labels: ::std::vec::Vec<String> = ::std::vec::Vec::new();
        $crate::promql!(@labels __labels; $($labels)*);
        let __node = $crate::ast::Expr::Aggregation {
            op: stringify!($op).to_string(),
            modifier: ::core::option::Option::Some($crate::ast::AggrMod::Without(__labels)),
            arg: ::std::boxed::Box::new($crate::promql!(@expr $($inner)*)),
        };
        $crate::promql!(@expr_tail __node; $($rest)*)
    }};

    (@expr_unit ( $func:expr ) ( $a:literal , $b:literal $(, $rest:literal)* $(,)? ) $($tail:tt)*) => {{
        let __node = $crate::ast::Expr::Call {
            func: ::std::string::ToString::to_string(&$func),
            args: ::std::vec![$crate::scalar_arg($a), $crate::scalar_arg($b), $($crate::scalar_arg($rest),)*],
        };
        $crate::promql!(@expr_tail __node; $($tail)*)
    }};

    (@expr_unit ( $func:expr ) ( ( $first:expr ) , $($rest:tt)+ ) $($tail:tt)*) => {{
        let __node = $crate::ast::Expr::Call {
            func: ::std::string::ToString::to_string(&$func),
            args: ::std::vec![
                $crate::scalar_arg($first),
                $crate::promql! { @call_arg $($rest)* },
            ],
        };
        $crate::promql!(@expr_tail __node; $($tail)*)
    }};

    (@expr_unit ( $func:expr ) ( $first:literal , $($rest:tt)+ ) $($tail:tt)*) => {{
        let __node = $crate::ast::Expr::Call {
            func: ::std::string::ToString::to_string(&$func),
            args: ::std::vec![
                $crate::scalar_arg($first),
                $crate::promql! { @call_arg $($rest)* },
            ],
        };
        $crate::promql!(@expr_tail __node; $($tail)*)
    }};

    (@expr_unit ( $func:expr ) ( $first:ident , $($rest:tt)+ ) $($tail:tt)*) => {{
        let __node = $crate::ast::Expr::Call {
            func: ::std::string::ToString::to_string(&$func),
            args: ::std::vec![
                $crate::scalar_arg($first),
                $crate::promql! { @call_arg $($rest)* },
            ],
        };
        $crate::promql!(@expr_tail __node; $($tail)*)
    }};

    (@expr_unit ( $func:expr ) ( $($inner:tt)* ) $($tail:tt)*) => {{
        let mut __args: ::std::vec::Vec<$crate::ast::Expr> = ::std::vec::Vec::new();
        $crate::promql! { @call_args __args; $($inner)* };
        let __node = $crate::ast::Expr::Call {
            func: ::std::string::ToString::to_string(&$func),
            args: __args,
        };
        $crate::promql!(@expr_tail __node; $($tail)*)
    }};

    (@expr_unit $func:ident ( $a:literal , $b:literal $(, $rest:literal)* $(,)? ) $($tail:tt)*) => {{
        let __node = $crate::ast::Expr::Call {
            func: stringify!($func).to_string(),
            args: ::std::vec![$crate::scalar_arg($a), $crate::scalar_arg($b), $($crate::scalar_arg($rest),)*],
        };
        $crate::promql!(@expr_tail __node; $($tail)*)
    }};

    (@expr_unit $func:ident ( ( $first:expr ) , $($rest:tt)+ ) $($tail:tt)*) => {{
        let __node = $crate::ast::Expr::Call {
            func: stringify!($func).to_string(),
            args: ::std::vec![
                $crate::scalar_arg($first),
                $crate::promql! { @call_arg $($rest)* },
            ],
        };
        $crate::promql!(@expr_tail __node; $($tail)*)
    }};

    (@expr_unit $func:ident ( $first:literal , $($rest:tt)+ ) $($tail:tt)*) => {{
        let __node = $crate::ast::Expr::Call {
            func: stringify!($func).to_string(),
            args: ::std::vec![
                $crate::scalar_arg($first),
                $crate::promql! { @call_arg $($rest)* },
            ],
        };
        $crate::promql!(@expr_tail __node; $($tail)*)
    }};

    (@expr_unit $func:ident ( $first:ident , $($rest:tt)+ ) $($tail:tt)*) => {{
        let __node = $crate::ast::Expr::Call {
            func: stringify!($func).to_string(),
            args: ::std::vec![
                $crate::scalar_arg($first),
                $crate::promql! { @call_arg $($rest)* },
            ],
        };
        $crate::promql!(@expr_tail __node; $($tail)*)
    }};

    (@expr_unit $func:ident ( $($inner:tt)* ) $($tail:tt)*) => {{
        let mut __args: ::std::vec::Vec<$crate::ast::Expr> = ::std::vec::Vec::new();
        $crate::promql! { @call_args __args; $($inner)* };
        let __node = $crate::ast::Expr::Call {
            func: stringify!($func).to_string(),
            args: __args,
        };
        $crate::promql!(@expr_tail __node; $($tail)*)
    }};

    (@expr_unit $metric:literal { $($m:tt)* } $($rest:tt)*) => {{
        let mut __sel = $crate::ast::Selector::default();
        __sel.metric = ::core::option::Option::Some($metric.to_string());
        $crate::promql!(@matchers __sel; $($m)*);
        $crate::promql!(@mods __sel; $($rest)*);
        let __node = $crate::ast::Expr::Selector(__sel);
        $crate::promql!(@expr_tail __node;)
    }};
    (@expr_unit $metric:literal [ ( $d:expr ) ] $($rest:tt)*) => {{
        let mut __sel = $crate::ast::Selector::default();
        __sel.metric = ::core::option::Option::Some($metric.to_string());
        __sel.range = ::core::option::Option::Some(::std::string::ToString::to_string(&$d));
        $crate::promql!(@mods __sel; $($rest)*);
        let __node = $crate::ast::Expr::Selector(__sel);
        $crate::promql!(@expr_tail __node;)
    }};
    (@expr_unit $metric:literal [ $d:tt ] $($rest:tt)*) => {{
        let mut __sel = $crate::ast::Selector::default();
        __sel.metric = ::core::option::Option::Some($metric.to_string());
        __sel.range = ::core::option::Option::Some(stringify!($d).to_string());
        $crate::promql!(@mods __sel; $($rest)*);
        let __node = $crate::ast::Expr::Selector(__sel);
        $crate::promql!(@expr_tail __node;)
    }};

    (@expr_unit $lit:literal $($rest:tt)*) => {{
        let __node = $crate::ast::Expr::Scalar($lit as f64);
        $crate::promql!(@expr_tail __node; $($rest)*)
    }};

    (@expr_unit $($tokens:tt)+) => {{
        $crate::promql!(@expr_unit_split []; $($tokens)*)
    }};

    (@expr_unit) => {{
        compile_error!("promql!: empty expression")
    }};

    // ---------------------------------------------------------------------
    // @expr_unit_split — selector fallback; split at first top-level binop
    // ---------------------------------------------------------------------
    (@expr_unit_split [$($acc:tt)*];) => {{
        let mut __sel = $crate::ast::Selector::default();
        $crate::promql!(@parse __sel; $($acc)*);
        $crate::promql!(@expr_tail $crate::ast::Expr::Selector(__sel);)
    }};
    (@expr_unit_split [$($acc:tt)*]; * $($rest:tt)*) => {{
        let mut __sel = $crate::ast::Selector::default();
        $crate::promql!(@parse __sel; $($acc)*);
        let __node = $crate::ast::Expr::Selector(__sel);
        $crate::promql!(@expr_tail __node; * $($rest)*)
    }};
    (@expr_unit_split [$($acc:tt)*]; / $($rest:tt)*) => {{
        let mut __sel = $crate::ast::Selector::default();
        $crate::promql!(@parse __sel; $($acc)*);
        let __node = $crate::ast::Expr::Selector(__sel);
        $crate::promql!(@expr_tail __node; / $($rest)*)
    }};
    (@expr_unit_split [$($acc:tt)*]; + $($rest:tt)*) => {{
        let mut __sel = $crate::ast::Selector::default();
        $crate::promql!(@parse __sel; $($acc)*);
        let __node = $crate::ast::Expr::Selector(__sel);
        $crate::promql!(@expr_tail __node; + $($rest)*)
    }};
    (@expr_unit_split [$($acc:tt)*]; - $($rest:tt)*) => {{
        let mut __sel = $crate::ast::Selector::default();
        $crate::promql!(@parse __sel; $($acc)*);
        let __node = $crate::ast::Expr::Selector(__sel);
        $crate::promql!(@expr_tail __node; - $($rest)*)
    }};
    (@expr_unit_split [$($acc:tt)*]; $tok:tt $($rest:tt)*) => {{
        $crate::promql!(@expr_unit_split [$($acc)* $tok]; $($rest)*)
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
    // @call_args / @call_arg — function argument parsing
    // ---------------------------------------------------------------------
    (@call_args $out:ident;) => {};
    (@call_args $out:ident; $($arg:tt)*) => {
        $out.push($crate::promql! { @call_arg $($arg)* });
    };

    (@call_arg $lit:literal) => {
        $crate::scalar_arg($lit)
    };
    (@call_arg $id:ident) => {
        $crate::scalar_arg($id)
    };
    (@call_arg ( $e:expr )) => {
        $crate::scalar_arg($e)
    };
    (@call_arg $($tt:tt)+) => {
        $crate::promql! { @expr $($tt)* }
    };

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
    (@parse $sel:ident; $metric:literal { $($m:tt)* } $($rest:tt)*) => {
        $sel.metric = ::core::option::Option::Some($metric.to_string());
        $crate::promql!(@matchers $sel; $($m)*);
        $crate::promql!(@mods $sel; $($rest)*);
    };
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
    (@parse $sel:ident; $metric:literal $($rest:tt)*) => {
        $sel.metric = ::core::option::Option::Some($metric.to_string());
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
    (@matchers_into $out:expr; ? $label:literal = ~ $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_opt_matcher(
            $out,
            $label,
            $crate::ast::MatchOp::Re,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; ? $label:literal ! ~ $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_opt_matcher(
            $out,
            $label,
            $crate::ast::MatchOp::Nre,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; ? $label:literal != $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_opt_matcher(
            $out,
            $label,
            $crate::ast::MatchOp::Ne,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; $label:literal = ~ $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_matcher(
            $out,
            $label,
            $crate::ast::MatchOp::Re,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; $label:literal ! ~ $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_matcher(
            $out,
            $label,
            $crate::ast::MatchOp::Nre,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; ? $label:literal = $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_opt_matcher(
            $out,
            $label,
            $crate::ast::MatchOp::Eq,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; $label:literal != $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_matcher(
            $out,
            $label,
            $crate::ast::MatchOp::Ne,
            &$val,
        );
        $( $crate::promql!(@matchers_into $out; $($rest)*); )?
    };
    (@matchers_into $out:expr; $label:literal = $val:expr $(, $($rest:tt)*)?) => {
        $crate::push_matcher(
            $out,
            $label,
            $crate::ast::MatchOp::Eq,
            &$val,
        );
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
                        ..Default::default()
                    })],
                }),
            }),
        }
    }

    use crate::ast::{AggrMod, Expr, LabelMatcher, MatchOp, MetricNameSyntax, Selector};

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
            sel.matchers
                .iter()
                .map(|m| m.op.clone())
                .collect::<Vec<_>>(),
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

    #[test]
    fn multi_arg_histogram_quantile() {
        let expr = promql!(histogram_quantile(
            0.99,
            sum by (le, http_method) (
                rate(http_request_duration_seconds_bucket { job = "api" }[5m])
            )
        ));
        let call = match &expr {
            Expr::Call { func, args } => {
                assert_eq!(func, "histogram_quantile");
                args
            }
            other => panic!("expected call, got {other:?}"),
        };
        assert_eq!(call.len(), 2);
        assert_eq!(call[0], Expr::Scalar(0.99));
        assert_eq!(
            expr.to_string(),
            "histogram_quantile(0.99, sum by (le, http_method) (rate(http_request_duration_seconds_bucket{job=\"api\"}[5m])))"
        );
    }

    #[test]
    fn multi_arg_runtime_quantile_variable() {
        let quantile = 0.95;
        let expr = promql!(histogram_quantile(
            quantile,
            sum by (le) (rate(buckets[5m]))
        ));
        let call = match &expr {
            Expr::Call { func, args } => {
                assert_eq!(func, "histogram_quantile");
                args
            }
            other => panic!("expected call, got {other:?}"),
        };
        assert_eq!(call.len(), 2);
        assert_eq!(call[0], Expr::Scalar(0.95));
    }

    #[test]
    fn multi_arg_nested_commas_stay_in_one_arg() {
        let expr = promql!(histogram_quantile(
            0.99,
            rate(http_requests { environment = "staging", method = "GET" }[5m])
        ));
        let call = match &expr {
            Expr::Call { args, .. } => args,
            other => panic!("expected call, got {other:?}"),
        };
        assert_eq!(call.len(), 2);
        assert!(matches!(&call[1], Expr::Call { func, .. } if func == "rate"));
    }

    #[test]
    fn multi_arg_single_arg_regression() {
        let expr = promql!(rate(http_requests_total { method = "GET" }[5m]));
        let call = match &expr {
            Expr::Call { func, args } => {
                assert_eq!(func, "rate");
                args
            }
            other => panic!("expected call, got {other:?}"),
        };
        assert_eq!(call.len(), 1);
    }

    #[test]
    fn multi_arg_dynamic_func() {
        let func = "histogram_quantile";
        let expr = promql!((func)(0.99, sum by (le) (rate(buckets[5m]))));
        let call = match &expr {
            Expr::Call { func, args } => {
                assert_eq!(func, "histogram_quantile");
                args
            }
            other => panic!("expected call, got {other:?}"),
        };
        assert_eq!(call.len(), 2);
    }

    #[test]
    fn vector_binary_division() {
        let groups = vec!["http_method".to_string()];
        let window = "5m";
        let result: Option<&str> = None;
        let metric = "requests";
        let expr = promql!(
            sum by (..(groups.clone())) (
                rate((metric) {
                    tenant_id = "t1",
                    result = "err",
                } [(window)])
            )
            /
            sum by (..(groups)) (
                rate((metric) {
                    tenant_id = "t1",
                    ?result = result,
                } [(window)])
            )
        );
        assert_eq!(
            expr.to_string(),
            concat!(
                "sum by (http_method) (rate(requests{tenant_id=\"t1\", result=\"err\"}[5m]))",
                " / sum by (http_method) (rate(requests{tenant_id=\"t1\"}[5m]))"
            )
        );
    }

    #[test]
    fn multi_arg_three_literal_scalars() {
        let expr = promql!(clamp_min(0.5, 0.0, 1.0));
        let call = match &expr {
            Expr::Call { func, args } => {
                assert_eq!(func, "clamp_min");
                args
            }
            other => panic!("expected call, got {other:?}"),
        };
        assert_eq!(call.len(), 3);
        assert_eq!(
            call,
            &[
                Expr::Scalar(0.5),
                Expr::Scalar(0.0),
                Expr::Scalar(1.0),
            ]
        );
    }

    #[test]
    fn dotted_metric_name_via_expr_normal_syntax() {
        let name = "http.server.duration_seconds";
        let expr = promql!((name) { tenant_id = "t1" }[5m]);
        assert_eq!(
            expr.to_string(),
            "{__name__=\"http.server.duration_seconds\", tenant_id=\"t1\"}[5m]"
        );
    }

    #[test]
    fn dotted_metric_name_via_literal_normal_syntax() {
        let expr = promql!("http.server.duration_seconds" { tenant_id = "t1" }[5m]);
        let s = sel(&expr);
        assert_eq!(s.metric.as_deref(), Some("http.server.duration_seconds"));
        assert_eq!(
            expr.to_string(),
            "{__name__=\"http.server.duration_seconds\", tenant_id=\"t1\"}[5m]"
        );
    }

    #[test]
    fn dotted_metric_name_literal_inside_call() {
        let expr = promql!(rate("http.server.duration_seconds" { tenant_id = "t1" }[5m]));
        assert_eq!(
            expr.to_string(),
            "rate({__name__=\"http.server.duration_seconds\", tenant_id=\"t1\"}[5m])"
        );
    }

    #[test]
    fn name_label_syntax_bare_metric() {
        let expr = promql!(up);
        let mut sel = sel(&expr).clone();
        sel.metric_name_syntax = MetricNameSyntax::NameLabel;
        assert_eq!(sel.to_string(), "{__name__=\"up\"}");
    }

    #[test]
    fn quoted_brace_syntax_bare_metric_with_matchers() {
        let expr = promql!(http_requests_total { method = "GET" });
        let mut sel = sel(&expr).clone();
        sel.metric_name_syntax = MetricNameSyntax::QuotedBrace;
        assert_eq!(
            sel.to_string(),
            "{\"http_requests_total\", \"method\"=\"GET\"}"
        );
    }

    #[test]
    fn quoted_brace_syntax_dotted_metric() {
        let expr = promql!("http.server.duration_seconds" { tenant_id = "t1" }[5m]);
        let mut sel = sel(&expr).clone();
        sel.metric_name_syntax = MetricNameSyntax::QuotedBrace;
        assert_eq!(
            sel.to_string(),
            "{\"http.server.duration_seconds\", \"tenant_id\"=\"t1\"}[5m]"
        );
    }

    #[test]
    fn name_matcher_via_macro_normal_syntax() {
        let name = "http.server.duration_seconds";
        let expr = promql!({ __name__ = name, tenant_id = "t1" });
        assert_eq!(
            expr.to_string(),
            "{__name__=\"http.server.duration_seconds\", tenant_id=\"t1\"}"
        );
    }

    #[test]
    fn dedup_metric_field_and_name_matcher() {
        let sel = Selector {
            metric: Some("foo.bar".to_string()),
            matchers: vec![
                LabelMatcher {
                    name: "__name__".to_string(),
                    op: MatchOp::Eq,
                    value: "ignored".to_string(),
                },
                LabelMatcher {
                    name: "tenant_id".to_string(),
                    op: MatchOp::Eq,
                    value: "t1".to_string(),
                },
            ],
            ..Default::default()
        };
        assert_eq!(
            sel.to_string(),
            "{__name__=\"foo.bar\", tenant_id=\"t1\"}"
        );
    }

    #[test]
    fn promql_string_escaping_in_values() {
        let expr = promql!(m { msg = "say \"hi\"" });
        assert_eq!(expr.to_string(), "m{msg=\"say \\\"hi\\\"\"}");
    }

    #[test]
    fn label_literal_name_normal_syntax() {
        let expr = promql!(m { "service.name" = "frontend", tenant_id = "t1" });
        assert_eq!(
            expr.to_string(),
            "m{\"service.name\"=\"frontend\", tenant_id=\"t1\"}"
        );
    }

    #[test]
    fn label_literal_name_quoted_brace_syntax() {
        let expr = promql!(m { "service.name" = "frontend", tenant_id = "t1" });
        let mut sel = sel(&expr).clone();
        sel.metric_name_syntax = MetricNameSyntax::QuotedBrace;
        assert_eq!(
            sel.to_string(),
            "{\"m\", \"service.name\"=\"frontend\", \"tenant_id\"=\"t1\"}"
        );
    }

    #[test]
    fn promql_match_label_literal() {
        let env = "production";
        let matchers = promql_match!("deployment.environment" = env);
        assert_eq!(matchers.len(), 1);
        assert_eq!(matchers[0].name, "deployment.environment");
        assert_eq!(matchers[0].value, "production");
    }

    #[test]
    fn combined_otel_selector_normal_syntax() {
        let service_name = "frontend";
        let expr = promql!("http.server.duration_seconds" {
            "service.name" = service_name,
            tenant_id = "t1",
        });
        assert_eq!(
            expr.to_string(),
            "{__name__=\"http.server.duration_seconds\", \"service.name\"=\"frontend\", tenant_id=\"t1\"}"
        );
    }

    #[test]
    fn with_metric_name_syntax_builder() {
        let sel = Selector {
            metric: Some("up".to_string()),
            ..Default::default()
        }
        .with_metric_name_syntax(MetricNameSyntax::NameLabel);
        assert_eq!(sel.to_string(), "{__name__=\"up\"}");
    }
}
