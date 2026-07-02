//! PromQL abstract syntax tree and rendering.

use std::fmt;

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

/// A single `name <op> "value"` label matcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelMatcher {
    pub name: String,
    pub op: MatchOp,
    pub value: String,
}

/// A parsed PromQL vector selector.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selector {
    pub metric: Option<String>,
    pub matchers: Vec<LabelMatcher>,
    pub range: Option<String>,
    pub offset: Option<String>,
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(metric) = &self.metric {
            f.write_str(metric)?;
        }

        if !self.matchers.is_empty() {
            f.write_str("{")?;
            for (idx, matcher) in self.matchers.iter().enumerate() {
                if idx > 0 {
                    f.write_str(", ")?;
                }
                write!(f, "{}{}{:?}", matcher.name, matcher.op, matcher.value)?;
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
