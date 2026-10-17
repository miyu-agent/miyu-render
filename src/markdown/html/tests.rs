//! HTML：`<br>` 换行、格式标签开关、别的标签只留字、注释跨行不要、不像标签的照字、实体解码。

use super::*;

fn pieces(html: &str) -> Vec<Piece<'_>> {
    let mut comment = false;
    scan(html, &mut comment)
}

#[test]
fn line_breaks_and_formats() {
    assert_eq!(
        pieces("甲<br>乙</br><b>粗</b><EM>斜</EM><span class=\"x\">字</span>"),
        [
            Piece::Text("甲".into()),
            Piece::LineBreak,
            Piece::Text("乙".into()),
            Piece::LineBreak,
            Piece::Open(Format::Bold),
            Piece::Text("粗".into()),
            Piece::Close(Format::Bold),
            Piece::Open(Format::Italic),
            Piece::Text("斜".into()),
            Piece::Close(Format::Italic),
            Piece::Text("字".into()),
        ]
    );
}

#[test]
fn comments_span_lines_and_stray_angles_stay() {
    let mut comment = false;
    assert_eq!(scan("前<!-- 开", &mut comment), [Piece::Text("前".into())]);
    assert!(comment);
    assert_eq!(
        scan("还在 --> 后", &mut comment),
        [Piece::Text(" 后".into())]
    );
    assert!(!comment);
    assert_eq!(
        pieces("a < b"),
        [
            Piece::Text("a ".into()),
            Piece::Text("<".into()),
            Piece::Text(" b".into())
        ]
    );
    assert_eq!(
        pieces("<1>"),
        [Piece::Text("<".into()), Piece::Text("1>".into())]
    );
}

#[test]
fn entities_are_decoded_and_unknown_ones_kept() {
    assert_eq!(
        pieces("&lt;a&gt; &amp; &nbsp;&#39;&foo;"),
        [Piece::Text("<a> &  '&foo;".into())]
    );
}

#[test]
fn depths_open_and_close() {
    let mut depths = Depths::default();
    depths.apply(Format::Bold, true);
    depths.apply(Format::Bold, true);
    depths.apply(Format::Bold, false);
    depths.apply(Format::Code, false);
    assert_eq!((depths.bold, depths.code), (1, 0));
}
