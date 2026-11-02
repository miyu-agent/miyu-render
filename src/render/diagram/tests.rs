//! mermaid：配色照页面的颜色；最外面那一块底换成不填（找不到那一句的交回空的）；画出来四个角透明、中间有图、不超一栏和最高；
//! 写错的、空的画不出来。

use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::theme::Theme;

/// 只有测试字体的字体库。
fn fonts() -> Arc<usvg::fontdb::Database> {
    let mut database = usvg::fontdb::Database::new();
    database
        .load_font_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fonts/cjk.otf"))
        .expect("读得出");
    Arc::new(database)
}

#[test]
fn the_theme_follows_the_page() {
    let palette = Theme::Dark.palette();
    let theme = theme(&palette, "Miyu Test CJK");
    assert_eq!(theme.background, hex(palette.background));
    assert_eq!(
        theme.primary_color,
        hex(palette.quote()),
        "节点底比页面深一档"
    );
    assert_eq!(theme.primary_border_color, hex(palette.line));
    assert_eq!(theme.line_color, hex(palette.muted));
    assert_eq!(theme.primary_text_color, hex(palette.text));
    assert_eq!(theme.edge_label_background, hex(palette.background));
    assert!(theme.font_family.starts_with("\"Miyu Test CJK\", "));
    assert_eq!(hex([1, 171, 255, 0]), "#01abff");
}

#[test]
fn the_outer_backdrop_becomes_unfilled() {
    let svg = "<svg><rect x=\"0\" y=\"0\" width=\"9\" height=\"9\" fill=\"#1c1d20\"/><rect fill=\"#1c1d20\"/></svg>";
    assert_eq!(
        transparent(svg, "#1c1d20").as_deref(),
        Some(
            "<svg><rect x=\"0\" y=\"0\" width=\"9\" height=\"9\" fill=\"none\"/><rect fill=\"#1c1d20\"/></svg>"
        ),
        "只换最外面那一块"
    );
    assert_eq!(transparent(svg, "#ffffff"), None, "颜色对不上的不碰");
    assert_eq!(transparent("<svg></svg>", "#ffffff"), None);
}

#[test]
fn a_diagram_is_drawn_on_a_clear_ground() {
    let palette = Theme::Paper.palette();
    let image = draw(
        "graph TD\n  A[开始] --> B[结束]\n",
        &palette,
        "Miyu Test CJK",
        &fonts(),
        960,
    )
    .expect("画得出");
    assert!(image.width() <= 960 && image.height() <= MAX_HEIGHT);
    let (w, h) = (image.width() - 1, image.height() - 1);
    for (x, y) in [(0, 0), (w, 0), (0, h), (w, h)] {
        assert_eq!(image.get_pixel(x, y).0[3], 0, "角上透明：({x}, {y})");
    }
    assert!(
        image.pixels().filter(|pixel| pixel.0[3] == 255).count() > 500,
        "有图"
    );
}

#[test]
fn wide_diagrams_shrink_into_the_column() {
    let mut source = String::from("graph LR\n");
    for n in 0..30 {
        writeln!(source, "  N{n} --> N{}", n + 1).expect("写进字符串不会失败");
    }
    let image = draw(
        &source,
        &Theme::Paper.palette(),
        "Miyu Test CJK",
        &fonts(),
        400,
    )
    .expect("画得出");
    assert!(image.width() <= 400, "{}", image.width());
}

#[test]
fn broken_or_empty_sources_are_not_drawn() {
    let palette = Theme::Paper.palette();
    assert!(draw("   ", &palette, "Miyu Test CJK", &fonts(), 960).is_none());
    assert!(
        draw(
            "graph TD\n  A -->",
            &palette,
            "Miyu Test CJK",
            &fonts(),
            960
        )
        .is_none()
    );
    assert!(
        draw(
            "not a diagram at all",
            &palette,
            "Miyu Test CJK",
            &fonts(),
            960
        )
        .is_none()
    );
}

#[test]
fn mindmaps_and_pies_take_colours_from_the_page() {
    let palette = Theme::Dark.palette();
    let layout = layout(&palette);
    assert_eq!(
        layout.mindmap.root_fill.as_deref(),
        Some(hex(palette.accent).as_str())
    );
    assert_eq!(layout.mindmap.section_label_colors[0], hex(palette.text));
    assert!(
        layout
            .mindmap
            .section_colors
            .iter()
            .all(|colour| colour.starts_with('#')),
        "不用渲染器出厂的 hsl 饱和色"
    );
    let pie = theme(&palette, "x").pie_colors;
    for pair in pie.windows(2) {
        assert_ne!(pair[0], pair[1], "相邻的扇形分得开");
    }
}

/// 一个新的空文件夹（系统临时目录下）。
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("miyu-render-audit-{}-{name}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).expect("建得了");
    dir
}

/// 审计第四轮（2026-11-02）：mermaid-rs-renderer 把 `style`、`classDef` 的值（`fill:` 后面的字）原样拼进 SVG 的属性，不转义；
/// 一个引号收口就能在图里另写元素。usvg 默认照 `<image href>` 读本机的文件（`default_string_resolver`：路径在就 `fs::read`），
/// SVG 文件整个画进图里。这个进程不在沙盒里：模型写的一张图能把本机读得到的 SVG 画进图、发出去；`/dev/zero` 这类路径会一直读
/// 到内存耗尽（没跑，只读代码）。
#[test]
fn a_diagram_never_draws_a_file_from_the_disk() {
    let dir = scratch("image-href");
    let secret = dir.join("secret.svg");
    std::fs::write(
        &secret,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"100\"><rect width=\"100\" height=\"100\" fill=\"#ff0000\"/></svg>",
    )
    .expect("写得了");
    let source = format!(
        "graph TD\n  A[x] --> B[y]\n  style A fill:#fff\" /><image href=\"{}\" x=\"0\" y=\"0\" width=\"300\" height=\"300\" /><rect fill=\"#fff\n",
        secret.display()
    );
    let image = draw(
        &source,
        &Theme::Paper.palette(),
        "Miyu Test CJK",
        &fonts(),
        960,
    );
    drop(std::fs::remove_dir_all(&dir));
    let red = image.map_or(0, |image| {
        image
            .pixels()
            .filter(|pixel| {
                pixel.0[0] > 240 && pixel.0[1] < 20 && pixel.0[2] < 20 && pixel.0[3] > 200
            })
            .count()
    });
    assert_eq!(red, 0, "本机的文件画进了图：{red} 个红点");
}

/// 审计第四轮：mermaid 的排版没有时限，源码只限 64 KB（实际被两万字的上限管着）。六十个节点、一千七百来条边（一万九千字）的
/// 流程图在发行版里排了 333 秒，一千五百条边的一条链 10 秒、状态图 12 秒（实测）。`methods::draw` 30 秒不等了，可画的线程停不下来，
/// 一直拿着画的那一套的锁（见 `methods::tests::a_drawing_that_ran_over_does_not_hold_up_the_next`）。图的大小（节点、边）该有上限，
/// 或者排版本身限时。
#[test]
fn a_dense_diagram_is_given_up_within_the_time_limit() {
    let mut source = String::from("graph LR\n");
    'all: for from in 0..60 {
        for to in 0..60 {
            if from != to {
                writeln!(source, "n{from}-->n{to}").expect("写进字符串不会失败");
                if source.len() > 19_000 {
                    break 'all;
                }
            }
        }
    }
    let (tell, done) = std::sync::mpsc::channel();
    let fonts = fonts();
    std::thread::spawn(move || {
        let image = draw(
            &source,
            &Theme::Paper.palette(),
            "Miyu Test CJK",
            &fonts,
            960,
        );
        drop(tell.send(image));
    });
    let limit = crate::methods::TIME_LIMIT;
    let answered = done.recv_timeout(limit);
    assert!(answered.is_ok(), "一张图 {limit:?} 还没排完");
}

/// 节点、边多过上限的不排（排版停不下来，越大越慢）：[`MAX_NODES`] 个节点的一条链照画，多一个不画；两个节点之间连
/// [`MAX_EDGES`] 加一条的不画。
#[test]
fn too_many_nodes_or_edges_are_not_drawn() {
    let chain = |nodes: usize| {
        let mut source = String::from("graph TD\n");
        for node in 1..nodes {
            writeln!(source, "n{}-->n{node}", node - 1).expect("写进字符串不会失败");
        }
        source
    };
    let palette = Theme::Paper.palette();
    let drawn = |source: &str| draw(source, &palette, "Miyu Test CJK", &fonts(), 960).is_some();
    assert!(drawn(&chain(MAX_NODES)), "正好上限照画");
    assert!(!drawn(&chain(MAX_NODES + 1)), "多一个节点不画");
    let parallel = format!("graph TD\n{}", "a-->b\n".repeat(MAX_EDGES + 1));
    assert!(!drawn(&parallel), "多一条边不画");
}

/// 审计第四轮：这里依赖的是 crates.io 上没补过的 mermaid-rs-renderer 0.3.1。它量字宽时另扫一遍系统字体，挑中的字体整份拷进一个
/// 全局的表常驻，还整份写进 `$XDG_CACHE_HOME/mmdr/font-cache/<哈希>.font`（没设的是 `~/.cache/mmdr`）：包外面、几个用这个库的
/// 程序共用、卸载不清；核心 V-35 为这个补过这个库（`third_party/mermaid-rs-renderer`），一律不碰那个目录。跑这里的测试也往开发者
/// 的家目录里写。另起一个测试进程，`XDG_CACHE_HOME`、`HOME` 指到临时文件夹，画一张图，看那里有没有多出东西。
#[test]
fn drawing_a_diagram_writes_nothing_under_the_cache_home() {
    if std::env::var_os("MIYU_RENDER_AUDIT_CHILD").is_some() {
        draw(
            "graph TD\n  A[开始] --> B[结束]\n",
            &Theme::Paper.palette(),
            "Miyu Test CJK",
            &fonts(),
            960,
        )
        .expect("画得出");
        return;
    }
    let home = scratch("cache-home");
    let status = std::process::Command::new(std::env::current_exe().expect("找得到自己"))
        .args([
            "render::diagram::tests::drawing_a_diagram_writes_nothing_under_the_cache_home",
            "--exact",
            "--test-threads=1",
        ])
        .env("MIYU_RENDER_AUDIT_CHILD", "1")
        .env("XDG_CACHE_HOME", &home)
        .env("HOME", &home)
        .stdout(std::process::Stdio::null())
        .status()
        .expect("起得来");
    let left: Vec<_> = walk(&home);
    drop(std::fs::remove_dir_all(&home));
    assert!(status.success(), "子进程画图失败：{status}");
    assert!(left.is_empty(), "画一张图往缓存目录里写了：{left:?}");
}

/// `dir` 下所有的文件（递归）。
fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else {
            found.push(path);
        }
    }
    found
}
