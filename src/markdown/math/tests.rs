//! 行内公式换成 Unicode（用例照旧版终端那一份）：上下标、分式加括号、根号、希腊字母、算子名、不认得的照原文、嵌套太深的照原文。

use super::*;

#[test]
fn common_inline_formulas() {
    assert_eq!(unicode(r"E=mc^2"), "E=mc²");
    assert_eq!(
        unicode(r"x_{n+1}=x_n-\frac{f(x_n)}{f'(x_n)}"),
        "xₙ₊₁=xₙ-(f(xₙ))/(f′(xₙ))"
    );
    assert_eq!(
        unicode(r"q=\frac{1+\sqrt5}{2}\approx 1.618"),
        "q=(1+√5)/2≈1.618"
    );
    assert_eq!(unicode(r"\alpha\in(0,1)"), "α∈(0,1)");
    assert_eq!(unicode(r"\sqrt{d_k}"), "√dₖ");
    assert_eq!(unicode(r"O(n\log n)"), "O(n log n)");
    assert_eq!(unicode(r"\operatorname{softmax}(z)_i"), "softmax(z)ᵢ");
    assert_eq!(unicode(r"a^{-1}b^{2n}"), "a⁻¹b²ⁿ");
    assert_eq!(unicode(r"\sum_{i=1}^{N} x_i"), "Σᵢ₌₁^N xᵢ");
    assert_eq!(unicode(r"90\%"), "90%");
    assert_eq!(
        unicode(r"\frac{(a)+(b)}{c}"),
        "((a)+(b))/c",
        "头一个括号中途就关了，还要再括"
    );
}

#[test]
fn unknown_commands_and_deep_nesting_stay() {
    assert_eq!(unicode(r"\weirdcmd{x}"), "\\weirdcmdx");
    assert_eq!(unicode(r"AT^{\top}"), "ATᵀ");
    let deep = format!(
        "{}x{}",
        "{".repeat(MAX_NESTING + 1),
        "}".repeat(MAX_NESTING + 1)
    );
    assert_eq!(unicode(&deep), deep);
}
