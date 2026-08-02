//! Public-boundary tests for the statements formula DSL.

use finstack_quant_core::expr::{BinOp as CoreBinOp, ExprNode, Function, UnaryOp as CoreUnaryOp};
use finstack_quant_statements::dsl::{
    parse_and_compile, parse_formula, BinOp as StmtBinOp, StmtExpr, UnaryOp as StmtUnaryOp,
};

#[test]
fn operators_precedence_and_conditionals_parse_and_compile_consistently() {
    let binary_cases = [
        ("lhs + rhs", StmtBinOp::Add, CoreBinOp::Add),
        ("lhs - rhs", StmtBinOp::Sub, CoreBinOp::Sub),
        ("lhs * rhs", StmtBinOp::Mul, CoreBinOp::Mul),
        ("lhs / rhs", StmtBinOp::Div, CoreBinOp::Div),
        ("lhs % rhs", StmtBinOp::Mod, CoreBinOp::Mod),
        ("lhs == rhs", StmtBinOp::Eq, CoreBinOp::Eq),
        ("lhs != rhs", StmtBinOp::Ne, CoreBinOp::Ne),
        ("lhs < rhs", StmtBinOp::Lt, CoreBinOp::Lt),
        ("lhs <= rhs", StmtBinOp::Le, CoreBinOp::Le),
        ("lhs > rhs", StmtBinOp::Gt, CoreBinOp::Gt),
        ("lhs >= rhs", StmtBinOp::Ge, CoreBinOp::Ge),
        ("lhs and rhs", StmtBinOp::And, CoreBinOp::And),
        ("lhs or rhs", StmtBinOp::Or, CoreBinOp::Or),
    ];

    for (formula, statement_op, core_op) in binary_cases {
        let parsed = parse_formula(formula).expect("binary expression should parse");
        assert!(
            matches!(parsed, StmtExpr::BinOp { op, .. } if op == statement_op),
            "statement operator mapping changed for {formula}"
        );

        let compiled = parse_and_compile(formula).expect("binary expression should compile");
        assert!(
            matches!(compiled.node, ExprNode::BinOp { op, .. } if op == core_op),
            "core operator mapping changed for {formula}"
        );
    }

    let unary_cases = [
        ("-revenue", StmtUnaryOp::Neg, CoreUnaryOp::Neg),
        ("!flag", StmtUnaryOp::Not, CoreUnaryOp::Not),
        ("not revenue > 0", StmtUnaryOp::Not, CoreUnaryOp::Not),
    ];

    for (formula, statement_op, core_op) in unary_cases {
        let parsed = parse_formula(formula).expect("unary expression should parse");
        assert!(
            matches!(parsed, StmtExpr::UnaryOp { op, .. } if op == statement_op),
            "statement unary mapping changed for {formula}"
        );

        let compiled = parse_and_compile(formula).expect("unary expression should compile");
        assert!(
            matches!(compiled.node, ExprNode::UnaryOp { op, .. } if op == core_op),
            "core unary mapping changed for {formula}"
        );
    }

    let precedence = parse_formula("1 + 2 * 3").expect("precedence expression should parse");
    let StmtExpr::BinOp {
        op: StmtBinOp::Add,
        left,
        right,
    } = precedence
    else {
        panic!("addition should be the top-level precedence node");
    };
    assert_eq!(*left, StmtExpr::Literal(1.0));
    assert!(matches!(
        *right,
        StmtExpr::BinOp {
            op: StmtBinOp::Mul,
            ..
        }
    ));

    let grouped = parse_formula("(1 + 2) * 3").expect("grouped expression should parse");
    let StmtExpr::BinOp {
        op: StmtBinOp::Mul,
        left,
        ..
    } = grouped
    else {
        panic!("multiplication should be the top-level grouped node");
    };
    assert!(matches!(
        *left,
        StmtExpr::BinOp {
            op: StmtBinOp::Add,
            ..
        }
    ));

    let nested_grouped = parse_formula("((1 + 2) * 3) / 4").expect("nested grouping should parse");
    assert!(matches!(
        nested_grouped,
        StmtExpr::BinOp {
            op: StmtBinOp::Div,
            ..
        }
    ));

    let compact = parse_formula("revenue-cogs").expect("compact expression should parse");
    let spaced = parse_formula("revenue - cogs").expect("spaced expression should parse");
    let padded = parse_formula("revenue  -  cogs").expect("padded expression should parse");
    assert_eq!(compact, spaced);
    assert_eq!(spaced, padded);

    let conditional = "if(revenue > 1000000, revenue * 0.1, 0)";
    let parsed = parse_formula(conditional).expect("conditional should parse");
    let StmtExpr::IfThenElse {
        condition,
        then_expr,
        else_expr,
    } = parsed
    else {
        panic!("if() should lower to the statements conditional node");
    };
    assert!(matches!(
        *condition,
        StmtExpr::BinOp {
            op: StmtBinOp::Gt,
            ..
        }
    ));
    assert!(matches!(
        *then_expr,
        StmtExpr::BinOp {
            op: StmtBinOp::Mul,
            ..
        }
    ));
    assert_eq!(*else_expr, StmtExpr::Literal(0.0));

    let compiled = parse_and_compile(conditional).expect("conditional should compile");
    let ExprNode::IfThenElse {
        condition,
        then_expr,
        else_expr,
    } = compiled.node
    else {
        panic!("if() should lower to the core conditional node");
    };
    assert!(matches!(
        condition.node,
        ExprNode::BinOp {
            op: CoreBinOp::Gt,
            ..
        }
    ));
    assert!(matches!(
        then_expr.node,
        ExprNode::BinOp {
            op: CoreBinOp::Mul,
            ..
        }
    ));
    assert!(matches!(else_expr.node, ExprNode::Literal(value) if value == 0.0));
}

#[test]
fn function_names_arities_and_core_mappings_stay_in_sync() {
    let cases = [
        ("lag(revenue, 1)", "lag", 2, Function::Lag),
        ("diff(revenue, 1)", "diff", 2, Function::Diff),
        (
            "pct_change(revenue, 1)",
            "pct_change",
            2,
            Function::PctChange,
        ),
        ("cumsum(revenue)", "cumsum", 1, Function::CumSum),
        ("cumprod(revenue)", "cumprod", 1, Function::CumProd),
        ("cummin(revenue)", "cummin", 1, Function::CumMin),
        ("cummax(revenue)", "cummax", 1, Function::CumMax),
        (
            "rolling_mean(revenue, 4)",
            "rolling_mean",
            2,
            Function::RollingMean,
        ),
        (
            "rolling_sum(revenue, 4)",
            "rolling_sum",
            2,
            Function::RollingSum,
        ),
        (
            "rolling_std(revenue, 4)",
            "rolling_std",
            2,
            Function::RollingStd,
        ),
        (
            "rolling_var(revenue, 4)",
            "rolling_var",
            2,
            Function::RollingVar,
        ),
        (
            "rolling_median(revenue, 4)",
            "rolling_median",
            2,
            Function::RollingMedian,
        ),
        (
            "rolling_min(revenue, 4)",
            "rolling_min",
            2,
            Function::RollingMin,
        ),
        (
            "rolling_max(revenue, 4)",
            "rolling_max",
            2,
            Function::RollingMax,
        ),
        (
            "rolling_count(revenue, 4)",
            "rolling_count",
            2,
            Function::RollingCount,
        ),
        ("ewm_mean(revenue, 0.5)", "ewm_mean", 2, Function::EwmMean),
        ("ewm_std(revenue, 0.5)", "ewm_std", 2, Function::EwmStd),
        ("ewm_var(revenue, 0.5)", "ewm_var", 2, Function::EwmVar),
        ("std(revenue)", "std", 1, Function::Std),
        ("var(revenue)", "var", 1, Function::Var),
        ("median(revenue)", "median", 1, Function::Median),
        ("shift(revenue, 1)", "shift", 2, Function::Shift),
        ("rank(revenue)", "rank", 1, Function::Rank),
        ("quantile(revenue, 0.5)", "quantile", 2, Function::Quantile),
        ("sum(revenue, cogs)", "sum", 2, Function::Sum),
        ("mean(revenue, cogs)", "mean", 2, Function::Mean),
        ("min(revenue, cogs)", "min", 2, Function::Min),
        ("max(revenue, cogs)", "max", 2, Function::Max),
        ("annualize(revenue, 4)", "annualize", 2, Function::Annualize),
        (
            "annualize_rate(rate, 4, 1)",
            "annualize_rate",
            3,
            Function::AnnualizeRate,
        ),
        ("ttm(ebitda)", "ttm", 1, Function::Ttm),
        ("ltm(ebitda)", "ltm", 1, Function::Ttm),
        ("ytd(revenue)", "ytd", 1, Function::Ytd),
        ("qtd(revenue)", "qtd", 1, Function::Qtd),
        (
            "fiscal_ytd(revenue, 4)",
            "fiscal_ytd",
            2,
            Function::FiscalYtd,
        ),
        ("coalesce(revenue, 0)", "coalesce", 2, Function::Coalesce),
        ("abs(revenue)", "abs", 1, Function::Abs),
        ("sign(revenue)", "sign", 1, Function::Sign),
        (
            "growth_rate(revenue, 4)",
            "growth_rate",
            2,
            Function::GrowthRate,
        ),
    ];

    for (formula, expected_name, expected_arity, expected_function) in cases {
        let parsed = parse_formula(formula).expect("supported function should parse");
        let StmtExpr::Call { func, args } = parsed else {
            panic!("{formula} should parse as a function call");
        };
        assert_eq!(
            func, expected_name,
            "DSL function name changed for {formula}"
        );
        assert_eq!(
            args.len(),
            expected_arity,
            "DSL argument count changed for {formula}"
        );

        let compiled = parse_and_compile(formula).expect("supported function should compile");
        let ExprNode::Call(function, args) = compiled.node else {
            panic!("{formula} should compile as a core function call");
        };
        assert_eq!(
            function, expected_function,
            "core function mapping changed for {formula}"
        );
        assert_eq!(
            args.len(),
            expected_arity,
            "compiled argument count changed for {formula}"
        );
    }

    let nested = "rolling_mean(lag(revenue, 1), 4)";
    let parsed = parse_formula(nested).expect("nested function should parse");
    let StmtExpr::Call { func, args } = parsed else {
        panic!("nested expression should have a function-call root");
    };
    assert_eq!(func, "rolling_mean");
    assert_eq!(args.len(), 2);
    assert!(matches!(
        &args[0],
        StmtExpr::Call { func, args } if func == "lag" && args.len() == 2
    ));

    let compiled = parse_and_compile(nested).expect("nested function should compile");
    let ExprNode::Call(Function::RollingMean, args) = compiled.node else {
        panic!("nested expression should compile with rolling_mean at the root");
    };
    assert_eq!(args.len(), 2);
    assert!(matches!(
        &args[0].node,
        ExprNode::Call(Function::Lag, lag_args) if lag_args.len() == 2
    ));
}

#[test]
fn malformed_and_unsupported_formulas_fail_at_the_public_boundary() {
    let invalid_formulas = [
        "",
        "revenue +",
        "revenue @@ cogs",
        "(revenue - cogs",
        "cs.interest_expense",
    ];

    for formula in invalid_formulas {
        assert!(
            parse_formula(formula).is_err(),
            "malformed formula unexpectedly parsed: {formula}"
        );
    }

    let compile_failures = [
        "revenue()",
        "lead(revenue, 1)",
        "lag(revenue)",
        "diff()",
        "pct_change(revenue, 1, 2)",
        "cumsum()",
        "rolling_mean(revenue)",
        "ewm_mean(revenue)",
        "ewm_std(revenue)",
        "std()",
        "shift(revenue)",
        "rank()",
        "quantile(revenue)",
        "sum()",
        "mean()",
        "min()",
        "max()",
        "annualize()",
        "annualize_rate(rate, 4)",
        "ttm(revenue, 4)",
        "ltm(revenue, 4)",
        "ytd()",
        "qtd(revenue, 4)",
        "fiscal_ytd(revenue)",
        "coalesce(revenue)",
        "abs(revenue, 1)",
        "sign()",
        "growth_rate(revenue, 1, 2)",
    ];

    for formula in compile_failures {
        assert!(
            parse_formula(formula).is_ok(),
            "compile-failure fixture must remain syntactically valid: {formula}"
        );
        assert!(
            parse_and_compile(formula).is_err(),
            "unsupported function or arity unexpectedly compiled: {formula}"
        );
    }
}
