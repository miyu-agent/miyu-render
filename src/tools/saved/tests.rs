//! 存图：照页的先后交回、读得回来；第一页没有、有一页是链接的算没画过；改名换上旧的；临时文件不留。

use std::fs;

use super::*;
use crate::test_support::workspace;

/// 一页 `width`×`height` 的图。
fn page(width: u32, height: u32) -> Page {
    let mut png = Vec::new();
    image::RgbaImage::new(width, height)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("编得了");
    Page { png, width, height }
}

#[test]
fn stored_pages_are_found_again_in_order() {
    let dir = folder(&workspace()).expect("建得了");
    let stored = store(&dir, "k", &[page(3, 4), page(5, 6)]).expect("存得了");
    assert_eq!(
        stored,
        [
            Saved {
                name: "render-k-1.png".into(),
                width: 3,
                height: 4
            },
            Saved {
                name: "render-k-2.png".into(),
                width: 5,
                height: 6
            },
        ]
    );
    assert_eq!(find(&dir, "k"), Some(stored));
    assert_eq!(fs::read_dir(&dir).expect("有").count(), 2, "临时文件不留");
    assert_eq!(find(&dir, "other"), None);
}

#[test]
fn a_missing_first_page_means_not_drawn_and_old_pages_are_replaced() {
    let dir = folder(&workspace()).expect("建得了");
    store(&dir, "k", &[page(3, 4), page(5, 6)]).expect("存得了");
    fs::remove_file(dir.join("render-k-1.png")).expect("删得了");
    assert_eq!(find(&dir, "k"), None, "第一页最后写：它不在就是没写完");
    store(&dir, "k", &[page(7, 8)]).expect("存得了");
    assert_eq!(find(&dir, "k").expect("在")[0].width, 7, "换上新的");
}

#[cfg(unix)]
#[test]
fn a_linked_page_means_not_drawn() {
    let dir = folder(&workspace()).expect("建得了");
    store(&dir, "k", &[page(3, 4)]).expect("存得了");
    std::os::unix::fs::symlink(dir.join("render-k-1.png"), dir.join("render-k-2.png"))
        .expect("链接");
    assert_eq!(find(&dir, "k"), None);
}

#[cfg(unix)]
#[test]
fn a_planted_temporary_link_is_not_followed() {
    // 临时文件的名字猜得到（进程号、计数）：每个可能的名字都预先放一个链到外面的链接，写不进去就算了，不能顺着写出去。
    let dir = folder(&workspace()).expect("建得了");
    let outside = workspace().join("victim");
    fs::write(&outside, "keep").expect("写得了");
    for counter in 0..2000 {
        let temp = dir.join(format!(
            "render-k-1.png.{}-{counter}.tmp",
            std::process::id()
        ));
        std::os::unix::fs::symlink(&outside, temp).expect("链接");
    }
    assert!(store(&dir, "k", &[page(3, 4)]).is_err(), "写不进去");
    assert_eq!(
        fs::read_to_string(&outside).expect("在"),
        "keep",
        "外面的没被写"
    );
}
