//! 画好存进工作区的 `rendered/`、交回路径和长宽、不交给核心；画过的不重画，换了配置的重画；`path` 只认工作区里的 `.md`、
//! `.markdown`（链接绕出去的、别的后缀、文件夹、太大的不认）；`markdown`、`path` 二选一；`rendered/` 指到外面的不用；
//! 预先放好的同名链接被换掉、不顺着写出去。

use std::fs;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

use super::*;
use crate::test_support::{setup, workspace};
use crate::theme::Theme;

fn params(cwd: &Path, args: &Value) -> Value {
    json!({"tool": "render_image", "args": args, "cwd": cwd.to_str().expect("是 UTF-8"), "session": "s1", "call_id": "c1"})
}

/// 结果的每一行拆成路径和长宽。
fn lines(text: &str) -> Vec<(String, u32, u32)> {
    text.lines()
        .map(|line| {
            let (path, size) = line.rsplit_once(' ').expect("路径 长x宽");
            let (width, height) = size.split_once('x').expect("长x宽");
            (
                path.to_string(),
                width.parse().expect("数"),
                height.parse().expect("数"),
            )
        })
        .collect()
}

#[test]
fn pages_are_saved_in_the_workspace_and_listed() {
    let (state, fake) = setup(false, &["cjk.otf"]);
    let dir = workspace();
    let text = run(
        &state,
        &params(&dir, &json!({"markdown": "# 你好\n\n世界"})),
    )
    .expect("画得出");
    let got = lines(&text);
    assert_eq!(got.len(), 1, "{text}");
    let (path, width, height) = &got[0];
    assert!(
        path.starts_with(dir.join("rendered").to_str().expect("UTF-8")),
        "{path}"
    );
    let name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .expect("有名字");
    assert!(
        name.starts_with("render-")
            && name.ends_with("-1.png")
            && name.len() == "render--1.png".len() + 12,
        "{name}"
    );
    assert_eq!(
        image::image_dimensions(path).expect("是 PNG"),
        (*width, *height)
    );
    assert!(fake.put.lock().expect("没 panic").is_empty(), "不交给核心");
    let entries = fs::read_dir(dir.join("rendered")).expect("有").count();
    assert_eq!(entries, 1, "临时文件不留");
}

#[test]
fn the_same_text_is_not_drawn_twice_but_new_settings_are() {
    let (mut state, _) = setup(false, &["cjk.otf"]);
    let dir = workspace();
    let args = json!({"markdown": "你好"});
    let first = run(&state, &params(&dir, &args)).expect("画得出");
    state.limit = Duration::ZERO;
    assert_eq!(
        run(&state, &params(&dir, &args)),
        Ok(first.clone()),
        "画过的不重画（限时是零也照交）"
    );
    state.settings.lock().expect("没 panic").theme = Theme::Dark;
    assert_eq!(
        run(&state, &params(&dir, &args)),
        Err("Took too long to draw.".into()),
        "换了配色要重画"
    );
    state.limit = crate::methods::TIME_LIMIT;
    let dark = run(&state, &params(&dir, &args)).expect("画得出");
    assert_ne!(dark, first);
    let (path, ..) = &lines(&dark)[0];
    let pixel = image::open(path)
        .expect("是图")
        .to_rgba8()
        .get_pixel(0, 0)
        .0;
    assert_eq!(pixel, Theme::Dark.palette().background);
}

#[test]
fn every_setting_and_the_text_change_the_key() {
    let base = Settings::default();
    let keys = [
        key(&base, "a"),
        key(&base, "b"),
        key(
            &Settings {
                theme: Theme::Light,
                ..base.clone()
            },
            "a",
        ),
        key(
            &Settings {
                font: Some("X".into()),
                ..base.clone()
            },
            "a",
        ),
        key(
            &Settings {
                code_font: Some("X".into()),
                ..base.clone()
            },
            "a",
        ),
        key(
            &Settings {
                font_size: 20,
                ..base.clone()
            },
            "a",
        ),
        key(
            &Settings {
                font: Some("a".into()),
                ..base.clone()
            },
            "",
        ),
    ];
    for (index, one) in keys.iter().enumerate() {
        assert_eq!(one.len(), 12);
        assert!(one.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(!keys[index + 1..].contains(one), "{index}");
    }
    assert_eq!(key(&base, "a"), key(&base, "a"));
}

#[test]
fn a_markdown_file_in_the_workspace_is_read() {
    let (state, _) = setup(false, &["cjk.otf"]);
    let dir = workspace();
    fs::create_dir_all(dir.join("notes")).expect("建得了");
    fs::write(dir.join("notes").join("a.MD"), "# 标题").expect("写得了");
    let relative = run(&state, &params(&dir, &json!({"path": "notes/a.MD"}))).expect("相对路径");
    let absolute = dir.join("notes").join("a.MD");
    let absolute = run(
        &state,
        &params(&dir, &json!({"path": absolute.to_str().expect("UTF-8")})),
    )
    .expect("绝对路径");
    assert_eq!(relative, absolute, "同一份字");
    let direct = run(&state, &params(&dir, &json!({"markdown": "# 标题"}))).expect("画得出");
    assert_eq!(direct, relative, "和直接给字一样");
}

#[test]
fn only_markdown_files_inside_the_workspace() {
    let (state, _) = setup(false, &["cjk.otf"]);
    let dir = workspace();
    let outside = workspace();
    fs::write(outside.join("secret.md"), "key").expect("写得了");
    fs::write(dir.join("a.txt"), "x").expect("写得了");
    fs::create_dir_all(dir.join("folder.md")).expect("建得了");
    let big = "a".repeat(usize::try_from(MAX_BYTES).expect("够") + 1);
    fs::write(dir.join("big.md"), big).expect("写得了");
    let secret = outside.join("secret.md");
    let secret = secret.to_str().expect("UTF-8");
    for (args, why) in [
        (
            json!({"path": secret}),
            format!("{secret} is outside the workspace."),
        ),
        (
            json!({"path": "a.txt"}),
            "a.txt is not a .md or .markdown file.".into(),
        ),
        (
            json!({"path": "folder.md"}),
            "folder.md is not a .md or .markdown file.".into(),
        ),
        (
            json!({"path": "big.md"}),
            format!("big.md is longer than {MAX_CHARS} characters."),
        ),
        (
            json!({"markdown": "a", "path": "a.md"}),
            "Give markdown or path, not both.".into(),
        ),
        (json!({"markdown": " "}), "Give markdown or path.".into()),
        (json!({}), "Give markdown or path.".into()),
    ] {
        assert_eq!(run(&state, &params(&dir, &args)), Err(why), "{args}");
    }
    let missing = run(&state, &params(&dir, &json!({"path": "missing.md"}))).expect_err("没有");
    assert!(missing.starts_with("Cannot read missing.md: "), "{missing}");
    assert!(!dir.join("rendered").exists(), "拒了的不建文件夹");
}

#[test]
fn too_long_markdown_is_refused_with_a_sentence() {
    let (state, _) = setup(false, &["cjk.otf"]);
    let dir = workspace();
    let long = "字".repeat(MAX_CHARS + 1);
    assert_eq!(
        run(&state, &params(&dir, &json!({ "markdown": long }))),
        Err(format!("Markdown is longer than {MAX_CHARS} characters."))
    );
}

#[cfg(unix)]
#[test]
fn links_do_not_lead_outside() {
    use std::os::unix::fs::symlink;
    let (state, _) = setup(false, &["cjk.otf"]);
    let dir = workspace();
    let outside = workspace();
    fs::write(outside.join("secret.md"), "key").expect("写得了");
    symlink(outside.join("secret.md"), dir.join("link.md")).expect("链接");
    assert_eq!(
        run(&state, &params(&dir, &json!({"path": "link.md"}))),
        Err("link.md is outside the workspace.".into())
    );
    // 存图的文件夹是链到外面的：不用。
    symlink(&outside, dir.join("rendered")).expect("链接");
    assert_eq!(
        run(&state, &params(&dir, &json!({"markdown": "a"}))),
        Err("rendered/ points outside the workspace.".into())
    );
    assert_eq!(
        fs::read_dir(&outside).expect("有").count(),
        1,
        "外面没多东西"
    );
}

#[cfg(unix)]
#[test]
fn a_planted_link_is_replaced_not_followed() {
    use std::os::unix::fs::symlink;
    let (state, _) = setup(false, &["cjk.otf"]);
    let dir = workspace();
    let outside = workspace();
    fs::write(outside.join("victim"), "keep").expect("写得了");
    let name = format!("render-{}-1.png", key(&Settings::default(), "a"));
    fs::create_dir_all(dir.join("rendered")).expect("建得了");
    symlink(outside.join("victim"), dir.join("rendered").join(&name)).expect("链接");
    let text = run(&state, &params(&dir, &json!({"markdown": "a"}))).expect("画得出");
    assert!(text.contains(&name), "{text}");
    assert_eq!(
        fs::read_to_string(outside.join("victim")).expect("在"),
        "keep",
        "外面的没被写"
    );
    let meta = fs::symlink_metadata(dir.join("rendered").join(&name)).expect("在");
    assert!(meta.is_file(), "链接换成了图");
}

#[test]
fn pages_go_to_the_pictures_folder_when_given() {
    let (state, _) = setup(false, &["cjk.otf"]);
    let dir = workspace();
    let home = workspace();
    let pictures = home.join("home").join("owner").join("pictures");
    let mut call = params(&dir, &json!({"markdown": "# 你好"}));
    call["pictures"] = json!(pictures.to_str().expect("UTF-8"));
    let text = run(&state, &call).expect("画得出");
    let got = lines(&text);
    assert_eq!(got.len(), 1, "{text}");
    let (path, width, height) = &got[0];
    assert_eq!(Path::new(path).parent(), Some(pictures.as_path()), "{path}");
    assert_eq!(
        image::image_dimensions(path).expect("是 PNG"),
        (*width, *height)
    );
    assert!(!dir.join("rendered").exists(), "工作区里不建 rendered/");
    let again = run(&state, &call).expect("画得出");
    assert_eq!(again, text, "画过的不重画");
}

#[cfg(unix)]
#[test]
fn a_linked_pictures_folder_is_refused() {
    use std::os::unix::fs::symlink;
    let (state, _) = setup(false, &["cjk.otf"]);
    let dir = workspace();
    let elsewhere = workspace();
    let pictures = workspace().join("pictures");
    symlink(&elsewhere, &pictures).expect("链接");
    let mut call = params(&dir, &json!({"markdown": "a"}));
    call["pictures"] = json!(pictures.to_str().expect("UTF-8"));
    assert_eq!(
        run(&state, &call),
        Err("The pictures folder is a link.".to_string())
    );
    assert_eq!(
        fs::read_dir(&elsewhere).expect("有").count(),
        0,
        "别处没多东西"
    );
}

#[test]
fn an_empty_pictures_falls_back_to_the_workspace() {
    let (state, _) = setup(false, &["cjk.otf"]);
    let dir = workspace();
    let mut call = params(&dir, &json!({"markdown": "a"}));
    call["pictures"] = json!("");
    let text = run(&state, &call).expect("画得出");
    assert!(
        text.starts_with(dir.join("rendered").to_str().expect("UTF-8")),
        "{text}"
    );
}

#[test]
fn a_pictures_file_is_not_a_folder() {
    let (state, _) = setup(false, &["cjk.otf"]);
    let dir = workspace();
    let pictures = workspace().join("pictures");
    fs::write(&pictures, "not a folder").expect("写得了");
    let mut call = params(&dir, &json!({"markdown": "a"}));
    call["pictures"] = json!(pictures.to_str().expect("UTF-8"));
    let refused = run(&state, &call).expect_err("不是文件夹");
    assert!(
        refused.starts_with("Cannot create the pictures folder"),
        "{refused}"
    );
    assert_eq!(fs::read_to_string(&pictures).expect("在"), "not a folder");
}
