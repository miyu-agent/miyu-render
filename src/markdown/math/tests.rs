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

/// 审计第四轮（2026-11-02）：模块说明「嵌套深过 [`MAX_NESTING`] 的照原文（递归没有上限就是一个爆栈的口子）」，可 `depth` 只数
/// 裸 `{`：不带括号的 `\sqrt\sqrt…x`、`\hat\hat…x` 每一个也递归一层（`command` → `group` → `command`），守卫数不到。调试版
/// 2 MiB 栈的线程上两千层（八千个字）就爆栈、整个进程 abort（`catch_unwind` 接不住）；发行版实测一万层还撑得住，两万字以内
/// 到不了，余量只有两三倍。
#[test]
fn commands_without_braces_count_as_nesting_too() {
    let deep = format!("{}x", "\\sqrt".repeat(MAX_NESTING + 1));
    assert_eq!(unicode(&deep), deep, "嵌套深过上限的照原文");
}

/// 审计 #136：`group()` 不跳空白、不认 `\sqrt` 的可选参数 `[n]`：`\frac{1} {a+b}` 的分母只取到一个空格，排成「1/a+b」
/// （优先级错了）；`\sqrt[3]{x}` 排成「√[3]x」。
#[test]
fn audit_136_a_space_before_a_group_and_a_root_index_are_understood() {
    assert_eq!(unicode(r"\frac{1} {a+b}"), "1/(a+b)", "组前的空格要跳过");
    assert_eq!(unicode(r"\frac {1}{a+b}"), "1/(a+b)");
    let root = unicode(r"\sqrt[3]{x}");
    assert!(root == "³√x" || root == "∛x", "三次根号要认出 [3]：{root}");
}

/// 根号的次数没有上标字的照原样括着写在根号前面，不丢；空的次数照普通根号。
#[test]
fn a_root_index_without_superscripts_is_kept() {
    assert_eq!(unicode(r"\sqrt[n]{x+1}"), "ⁿ√(x+1)");
    assert_eq!(unicode(r"\sqrt[k+q]{x}"), "[k+q]√x");
    assert_eq!(unicode(r"\sqrt[]{x}"), "√x");
}
