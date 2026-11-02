//! mermaid 图（设计 36 第四节第 2 条，2026-10-17 项目主人定：和背景无缝，和终端一样）。
//!
//! - 配色照页面的六个颜色设好再出 SVG（[`theme`]），不再像旧版那样画完去改 SVG 里的颜色：节点底是页面底往字挪一点（同引用的底），
//!   边框是线的颜色，线和箭头是淡字，字是正文的字，线上标签的底、子图里面是页面底。
//! - 透明（[`transparent`]）：SVG 最外面那一块底是渲染器照 `background` 写的一句，照我们自己设的颜色找到这一句、换成不填，画出来的
//!   图除了图本身都是透明的，按透明度叠到页面上，图的底就是页面本身。旧版不知道底是什么颜色，只能猜「第一个整幅的方块」，浮点
//!   误差、思维导图没有那个方块时就露出白底（09-22、09-24）。
//! - 大小（同旧版）：图里的字缩到 24 像素，宽不超过一栏、高不超过 2200；间距收紧（节点 24、层 36），思维导图用树状。
//! - 字体和正文一样：图里的字先用正文的字体族，字体库就是排字的那一份。
//! - 规模（审计第四轮 R-2）：排版没有时限，也停不下来，稠密的图越大越慢（调试版 25 个节点两两连、600 条边 134 秒，发行版
//!   1700 条边 333 秒）。先读一遍数节点和边，多过 [`MAX_NODES`]、[`MAX_EDGES`] 的不排，照代码块画；这么多也画不进一栏、看不清。
//! - 渲染器量字宽时自己找系统字体，找到的记下在哪个文件（核心补过的那一份，审计第四轮 R-3）：记在包的状态目录里
//!   （[`remember_fonts`]），不往几个程序共用的 `~/.cache/mmdr` 写。

use std::path::PathBuf;
use std::sync::{Arc, Once};

use image::RgbaImage;
use mermaid_rs_renderer::{LayoutConfig, Theme};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

use crate::theme::{Palette, Rgba, mix};

/// 图里的字多大（旧版 `IMAGE_TEXT_PX`：正文 36 号的三分之二）。
pub const TEXT_PX: f32 = 24.0;

/// 一张图最高多少（旧版 `MAX_DIAGRAM_HEIGHT`：太高的图在分栏里放不下）。
pub const MAX_HEIGHT: u32 = 2200;

/// 源码最长多少（同旧版）。
const MAX_SOURCE: usize = 64 * 1024;

/// 一张图最多几个节点（一条链 300 个，调试版排 3 秒）。
const MAX_NODES: usize = 200;

/// 一张图最多几条边（最慢的是两两连的：调试版 210 条 30 秒、380 条 57 秒；状态图 200 条转移 16 秒）。
const MAX_EDGES: usize = 200;

/// 画一张图：画不出来（写错了、空的、太长、节点或边太多、画成空的）的交回空的，调的一方照代码块画。空的围栏读的时候就不成块，渲染器也不收空字。
pub fn draw(
    source: &str,
    palette: &Palette,
    body: &str,
    fonts: &Arc<usvg::fontdb::Database>,
    max_width: u32,
) -> Option<RgbaImage> {
    if source.len() > MAX_SOURCE {
        return None;
    }
    remember_fonts();
    let parsed = mermaid_rs_renderer::parse_mermaid_strict(source).ok()?;
    if parsed.graph.nodes.len() > MAX_NODES || parsed.graph.edges.len() > MAX_EDGES {
        return None;
    }
    let (theme, layout) = (theme(palette, body), layout(palette));
    // 同 `render_with_options`，只是先读出来数过。
    let laid = mermaid_rs_renderer::compute_layout(&parsed.graph, &theme, &layout);
    let svg = mermaid_rs_renderer::render_svg(&laid, &theme, &layout);
    let svg = transparent(&svg, &theme.background)?;
    raster(&svg, body, fonts, theme.font_size, max_width)
}

/// 渲染器记字体位置的目录，整个进程定一次：核心给了状态目录（`MIYU_PACKAGE_STATE`，作者说明：运行时的状态写那里）的记在
/// 它下面的 `mermaid-fonts/`，没给的不记（每个进程照旧扫一次系统字体库）。不定的话渲染器照它原来的 `$XDG_CACHE_HOME/mmdr`。
fn remember_fonts() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let state = std::env::var_os("MIYU_PACKAGE_STATE").filter(|dir| !dir.is_empty());
        mermaid_rs_renderer::set_font_cache_dir(
            state.map(|dir| PathBuf::from(dir).join("mermaid-fonts")),
        );
    });
}

/// 照页面的六个颜色配的主题。
pub fn theme(palette: &Palette, body: &str) -> Theme {
    let mut theme = Theme::modern();
    let node = hex(palette.quote());
    theme.font_family = format!("\"{body}\", {}", theme.font_family);
    theme.background = hex(palette.background);
    theme.primary_text_color = hex(palette.text);
    theme.text_color = hex(palette.text);
    theme.primary_border_color = hex(palette.line);
    theme.line_color = hex(palette.muted);
    theme.secondary_color = hex(palette.header());
    theme.tertiary_color = hex(mix(palette.background, palette.accent, 12));
    theme.edge_label_background = hex(palette.background);
    theme.cluster_background = hex(mix(palette.background, palette.text, 2));
    theme.cluster_border = hex(palette.line);
    theme.sequence_actor_fill.clone_from(&node);
    theme.primary_color = node;
    theme.sequence_actor_border = hex(palette.line);
    theme.sequence_actor_line = hex(palette.muted);
    theme.sequence_note_fill = hex(mix(palette.background, palette.accent, 12));
    theme.sequence_note_border = hex(palette.line);
    theme.sequence_activation_fill = hex(palette.header());
    theme.sequence_activation_border = hex(palette.line);
    theme.pie_title_text_color = hex(palette.text);
    theme.pie_section_text_color = hex(palette.text);
    theme.pie_legend_text_color = hex(palette.text);
    theme.pie_stroke_color = hex(palette.background);
    theme.pie_outer_stroke_color = hex(palette.line);
    theme.git_commit_label_color = hex(palette.text);
    theme.git_commit_label_background = hex(palette.quote());
    theme.git_tag_label_color = hex(palette.text);
    theme.git_tag_label_background = hex(palette.header());
    theme.git_tag_label_border = hex(palette.line);
    theme.pie_colors = pie(palette).map(hex);
    // 扇形不另加透明：和图例的方块一样深（出厂扇形半透明，深色页面上比图例暗一截）。
    theme.pie_opacity = 1.0;
    theme
}

/// 饼图的十二个颜色：强调色、淡字、正文三种深浅交替，相邻的扇形分得开。
fn pie(palette: &Palette) -> [Rgba; 12] {
    let bg = palette.background;
    [
        mix(bg, palette.accent, 80),
        mix(bg, palette.muted, 70),
        mix(bg, palette.accent, 45),
        mix(bg, palette.text, 30),
        mix(bg, palette.muted, 40),
        mix(bg, palette.accent, 25),
        mix(bg, palette.text, 55),
        mix(bg, palette.muted, 90),
        mix(bg, palette.accent, 60),
        mix(bg, palette.text, 18),
        mix(bg, palette.muted, 25),
        mix(bg, palette.accent, 35),
    ]
}

/// 版式（同旧版）：间距收紧（默认都是 50），思维导图用树状（根在左、往右长，宽度只随层数长，一栏装得下）。
///
/// 思维导图的颜色在版式这边：根用强调色；各枝在强调色、淡字之间交替，填色是浅浅的一层，线色深一些，字是正文的颜色（渲染器出厂
/// 是饱和的黄绿蓝）。
fn layout(palette: &Palette) -> LayoutConfig {
    let mut layout = LayoutConfig {
        node_spacing: 24.0,
        rank_spacing: 36.0,
        ..LayoutConfig::default()
    };
    layout.mindmap.layout_algorithm = "lr-tree".to_string();
    let bg = palette.background;
    let hues = [palette.accent, palette.muted];
    layout.mindmap.section_colors = (0..12)
        .map(|index| {
            hex(mix(
                bg,
                hues[index % 2],
                if index % 4 < 2 { 18 } else { 28 },
            ))
        })
        .collect();
    layout.mindmap.section_line_colors = (0..12)
        .map(|index| hex(mix(bg, hues[index % 2], 65)))
        .collect();
    layout.mindmap.section_label_colors = vec![hex(palette.text); 12];
    layout.mindmap.root_fill = Some(hex(palette.accent));
    layout.mindmap.root_text = Some(hex(palette.background));
    layout
}

/// 最外面那一块底换成不填：渲染器照 `fill="<background>"` 写的第一个 `<rect`。找不到的交回空的（渲染器换了写法，宁可照代码块画，
/// 也不画出一块和页面对不上的底）。
pub fn transparent(svg: &str, background: &str) -> Option<String> {
    let fill = format!("fill=\"{background}\"/>");
    let start = svg.find("<rect ")?;
    let end = start + svg[start..].find("/>")? + 2;
    let rect = &svg[start..end];
    rect.ends_with(&fill).then(|| {
        format!(
            "{}{}{}",
            &svg[..start],
            rect.replace(&fill, "fill=\"none\"/>"),
            &svg[end..]
        )
    })
}

/// SVG 画成透明的图：字缩到 [`TEXT_PX`]，宽不超过 `max_width`、高不超过 [`MAX_HEIGHT`]。
///
/// `<image>` 一律不取（[`no_images`]）。
fn raster(
    svg: &str,
    body: &str,
    fonts: &Arc<usvg::fontdb::Database>,
    font_size: f32,
    max_width: u32,
) -> Option<RgbaImage> {
    let options = usvg::Options {
        font_family: body.to_string(),
        fontdb: Arc::clone(fonts),
        image_href_resolver: no_images(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(svg, &options).ok()?;
    let natural = tree.size();
    if natural.width() <= 0.0 || natural.height() <= 0.0 {
        return None;
    }
    let scale = (TEXT_PX / font_size)
        .min(super::units::float(max_width) / natural.width())
        .min(super::units::float(MAX_HEIGHT) / natural.height());
    let width = super::units::ceil(natural.width() * scale).max(1);
    let height = super::units::ceil(natural.height() * scale).max(1);
    let mut pixmap = Pixmap::new(width, height)?;
    resvg::render(
        &tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia 存的是预乘过透明度的颜色：还原成直通的，叠的时候照不透明度混合。
    let mut image = RgbaImage::new(width, height);
    for (pixel, colour) in image.pixels_mut().zip(pixmap.pixels()) {
        let colour = colour.demultiply();
        pixel.0 = [colour.red(), colour.green(), colour.blue(), colour.alpha()];
    }
    Some(image)
}

/// 图里的 `<image>` 一张都不取（审计第四轮 R-1）。渲染器把 `style`、`classDef` 的值原样拼进 SVG，模型写的样式能在图里另写一个
/// `<image href="/本机/的/文件">`；usvg 默认照路径读文件（这个进程不在沙盒里），读得到的 SVG 会被画进发出去的图，设备文件、管道
/// 还会一直读、一直等。内嵌的 `data:` 也不收：渲染器自己只在 C4 的人像里用两张 PNG，PNG 本来就没开着画。
fn no_images() -> usvg::ImageHrefResolver<'static> {
    usvg::ImageHrefResolver {
        resolve_data: Box::new(|_, _, _| None),
        resolve_string: Box::new(|_, _| None),
    }
}

/// `#rrggbb`。
fn hex([r, g, b, _]: Rgba) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

#[cfg(test)]
mod tests;
