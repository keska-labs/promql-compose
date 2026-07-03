//! PromQL abstract syntax tree and rendering.

use std::fmt;
use std::fmt::Write;

const NAME_LABEL: &str = "__name__";

/// The comparison operator used by a label matcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchOp {
    Eq,
    Ne,
    Re,
    Nre,
}

impl fmt::Display for MatchOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatchOp::Eq => f.write_str("="),
            MatchOp::Ne => f.write_str("!="),
            MatchOp::Re => f.write_str("=~"),
            MatchOp::Nre => f.write_str("!~"),
        }
    }
}

/// How to serialize metric names (and label name quoting) in a [`Selector`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetricNameSyntax {
    /// Bare prefix when legacy-compatible; `{__name__="..."}` for non-legacy names.
    #[default]
    Normal,
    /// Always `{__name__="..."}`.
    NameLabel,
    /// Always Prometheus 3.0 UTF-8 brace syntax `{"..."}`; quote all label names.
    QuotedBrace,
}

/// Returns true when `name` matches legacy Prometheus `[a-zA-Z_:][a-zA-Z0-9_:]*`.
pub fn is_legacy_promql_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == ':' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

fn write_promql_string(f: &mut fmt::Formatter<'_>, s: &str) -> fmt::Result {
    f.write_str("\"")?;
    for c in s.chars() {
        match c {
            '\\' | '"' => {
                f.write_str("\\")?;
                f.write_char(c)?;
            }
            _ => f.write_char(c)?,
        }
    }
    f.write_str("\"")
}

fn write_label_name(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    syntax: MetricNameSyntax,
) -> fmt::Result {
    let quote = match syntax {
        MetricNameSyntax::QuotedBrace => true,
        MetricNameSyntax::Normal | MetricNameSyntax::NameLabel => !is_legacy_promql_ident(name),
    };
    if quote {
        write_promql_string(f, name)
    } else {
        f.write_str(name)
    }
}

fn write_label_matcher(
    f: &mut fmt::Formatter<'_>,
    matcher: &LabelMatcher,
    syntax: MetricNameSyntax,
) -> fmt::Result {
    write_label_name(f, &matcher.name, syntax)?;
    write!(f, "{}", matcher.op)?;
    write_promql_string(f, &matcher.value)
}

/// A single `name <op> "value"` label matcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelMatcher {
    pub name: String,
    pub op: MatchOp,
    pub value: String,
}

/// A parsed PromQL vector selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    pub metric: Option<String>,
    pub matchers: Vec<LabelMatcher>,
    pub range: Option<String>,
    pub offset: Option<String>,
    pub metric_name_syntax: MetricNameSyntax,
}

impl Default for Selector {
    fn default() -> Self {
        Self {
            metric: None,
            matchers: Vec::new(),
            range: None,
            offset: None,
            metric_name_syntax: MetricNameSyntax::Normal,
        }
    }
}

impl Selector {
    /// Set the metric/label serialization syntax and return `self`.
    pub fn with_metric_name_syntax(mut self, syntax: MetricNameSyntax) -> Self {
        self.metric_name_syntax = syntax;
        self
    }

    fn effective_metric_name(&self) -> Option<&str> {
        if let Some(metric) = &self.metric {
            return Some(metric.as_str());
        }
        self.matchers.iter().find_map(|m| {
            if m.name == NAME_LABEL && m.op == MatchOp::Eq {
                Some(m.value.as_str())
            } else {
                None
            }
        })
    }

    fn should_skip_name_matcher(&self, rendering_metric_in_braces: bool) -> impl Fn(&LabelMatcher) -> bool + '_ {
        move |m: &LabelMatcher| {
            m.name == NAME_LABEL
                && m.op == MatchOp::Eq
                && (self.metric.is_some() || rendering_metric_in_braces)
        }
    }
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let syntax = self.metric_name_syntax;
        let metric_name = self.effective_metric_name();

        let use_bare = matches!(syntax, MetricNameSyntax::Normal)
            && metric_name.is_some_and(is_legacy_promql_ident);

        let use_quoted_metric = matches!(syntax, MetricNameSyntax::QuotedBrace)
            && metric_name.is_some();

        let use_name_label = metric_name.is_some()
            && (matches!(syntax, MetricNameSyntax::NameLabel)
                || (matches!(syntax, MetricNameSyntax::Normal)
                    && !metric_name.is_some_and(is_legacy_promql_ident)));

        let rendering_metric_in_braces = use_quoted_metric || use_name_label;
        let skip_name = self.should_skip_name_matcher(rendering_metric_in_braces);

        if use_bare {
            f.write_str(metric_name.unwrap())?;
        }

        let visible_matchers: Vec<_> = self
            .matchers
            .iter()
            .filter(|m| !skip_name(m))
            .collect();

        let needs_braces = rendering_metric_in_braces || !visible_matchers.is_empty();

        if needs_braces {
            f.write_str("{")?;
            let mut first = true;

            if use_quoted_metric {
                write_promql_string(f, metric_name.unwrap())?;
                first = false;
            } else if use_name_label {
                f.write_str(NAME_LABEL)?;
                f.write_str("=")?;
                write_promql_string(f, metric_name.unwrap())?;
                first = false;
            }

            for matcher in visible_matchers {
                if !first {
                    f.write_str(", ")?;
                }
                write_label_matcher(f, matcher, syntax)?;
                first = false;
            }

            f.write_str("}")?;
        }

        if let Some(range) = &self.range {
            write!(f, "[{range}]")?;
        }

        if let Some(offset) = &self.offset {
            write!(f, " offset {offset}")?;
        }

        Ok(())
    }
}

/// Aggregation modifier: `by (...)` or `without (...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggrMod {
    By(Vec<String>),
    Without(Vec<String>),
}

/// A PromQL expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Scalar(f64),
    Selector(Selector),
    Call {
        func: String,
        args: Vec<Expr>,
    },
    Aggregation {
        op: String,
        modifier: Option<AggrMod>,
        arg: Box<Expr>,
    },
    Binary {
        op: String,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

impl Expr {
    pub fn as_selector(&self) -> Option<&Selector> {
        match self {
            Expr::Selector(sel) => Some(sel),
            _ => None,
        }
    }
}

fn fmt_scalar(f: &mut fmt::Formatter<'_>, value: f64) -> fmt::Result {
    if value.fract() == 0.0 && value.is_finite() {
        write!(f, "{}", value as i64)
    } else {
        write!(f, "{value}")
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Scalar(v) => fmt_scalar(f, *v),
            Expr::Selector(sel) => write!(f, "{sel}"),
            Expr::Call { func, args } => {
                f.write_str(func)?;
                f.write_str("(")?;
                for (idx, arg) in args.iter().enumerate() {
                    if idx > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                f.write_str(")")
            }
            Expr::Aggregation { op, modifier, arg } => {
                f.write_str(op)?;
                if let Some(modifier) = modifier {
                    match modifier {
                        AggrMod::By(labels) => {
                            f.write_str(" by (")?;
                            for (idx, label) in labels.iter().enumerate() {
                                if idx > 0 {
                                    f.write_str(", ")?;
                                }
                                f.write_str(label)?;
                            }
                            f.write_str(")")?;
                        }
                        AggrMod::Without(labels) => {
                            f.write_str(" without (")?;
                            for (idx, label) in labels.iter().enumerate() {
                                if idx > 0 {
                                    f.write_str(", ")?;
                                }
                                f.write_str(label)?;
                            }
                            f.write_str(")")?;
                        }
                    }
                }
                write!(f, " ({arg})")
            }
            Expr::Binary { op, lhs, rhs } => write!(f, "{lhs} {op} {rhs}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_legacy_promql_ident_examples() {
        assert!(is_legacy_promql_ident("http_requests_total"));
        assert!(is_legacy_promql_ident("up"));
        assert!(is_legacy_promql_ident("_private"));
        assert!(!is_legacy_promql_ident("http.server.duration_seconds"));
        assert!(!is_legacy_promql_ident("service.name"));
        assert!(!is_legacy_promql_ident("1foo"));
    }
}
