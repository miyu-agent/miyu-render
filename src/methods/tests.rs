//! `markdown` 方法：画好经 `blob.put` 交核心（字节是 PNG、宽高对得上，名字照第几页），回编号和宽高；长的分几页；画崩了接住、下一次重开；
//! 太久不等；`theme` 照写的、不写照配置；写错的
//! `bad_params`（-32602）；字体一个有中文的都没有、`blob.put` 不成的回失败（-32010）；登记的名字；不认识的方法。

use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use super::*;
use crate::test_support::{Fake, fonts, setup};

fn markdown(state: &State, params: &Value) -> Answer {
    call(
        state,
        &json!({"method": "markdown", "params": params, "from": "onebot"}),
    )
}

/// `blob.put` 收到的那一张解开。
fn uploaded(fake: &Fake) -> image::RgbaImage {
    let put = fake.put.lock().expect("没 panic");
    assert_eq!(put.len(), 1);
    let bytes = STANDARD
        .decode(put[0]["data"].as_str().expect("有 data"))
        .expect("是 base64");
    image::load_from_memory(&bytes).expect("是图").to_rgba8()
}

#[test]
fn a_page_goes_to_the_core_and_comes_back_as_a_blob() {
    let (state, fake) = setup(false, &["cjk.otf"]);
    let answer = markdown(&state, &json!({"markdown": "你好世界"})).expect("画得出");
    let image = uploaded(&fake);
    assert_eq!(
        fake.put.lock().expect("没 panic")[0]["name"],
        "render-1.png"
    );
    assert_eq!(
        answer,
        json!({"pages": [{"blob": "b1", "width": image.width(), "height": image.height()}]})
    );
    assert_eq!(
        image.get_pixel(0, 0).0,
        Theme::Paper.palette().background,
        "不写照配置：出厂纸色"
    );
}

#[test]
fn the_theme_comes_from_the_call_or_the_settings() {
    let (state, fake) = setup(false, &["cjk.otf"]);
    markdown(&state, &json!({"markdown": "你好", "theme": "dark"})).expect("画得出");
    assert_eq!(
        uploaded(&fake).get_pixel(0, 0).0,
        Theme::Dark.palette().background
    );
    let (state, fake) = setup(false, &["cjk.otf"]);
    state.settings.lock().expect("没 panic").theme = Theme::Light;
    markdown(&state, &json!({"markdown": "你好"})).expect("画得出");
    assert_eq!(
        uploaded(&fake).get_pixel(0, 0).0,
        Theme::Light.palette().background
    );
}

#[test]
fn wrong_params_are_refused() {
    let (mut state, fake) = setup(false, &["cjk.otf"]);
    // 两万个字在 debug 构建里排要二十几秒（release 不到一秒），别让时限误判。
    state.limit = Duration::from_mins(5);
    let long = "字".repeat(MAX_CHARS + 1);
    for params in [
        json!({}),
        json!({"markdown": "  "}),
        json!({"markdown": 1}),
        json!({"markdown": long}),
        json!({"markdown": "a", "theme": "sepia"}),
        json!({"markdown": "a", "theme": 1}),
    ] {
        let error = markdown(&state, &params).expect_err("该拒");
        assert_eq!(error["code"], -32602, "{params}");
    }
    assert!(
        fake.put.lock().expect("没 panic").is_empty(),
        "拒了的不画、不交"
    );
    assert!(
        markdown(&state, &json!({"markdown": "字".repeat(MAX_CHARS)})).is_ok(),
        "正好两万个照画"
    );
}

#[test]
fn no_chinese_font_or_a_refused_put_fails() {
    let (state, _) = setup(false, &["latin.ttf"]);
    let error = markdown(&state, &json!({"markdown": "你好"})).expect_err("没有中文字体");
    assert_eq!(error["code"], -32010);
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.starts_with("no font with Chinese"))
    );
    let (state, _) = setup(true, &["cjk.otf"]);
    let error = markdown(&state, &json!({"markdown": "你好"})).expect_err("交不进去");
    assert_eq!(error["code"], -32010);
}

#[test]
fn registered_names_and_unknown_methods() {
    assert_eq!(
        register(),
        json!({"methods": [{"name": "markdown"}, {"name": "render.markdown"}]})
    );
    let (state, _) = setup(false, &["cjk.otf"]);
    let error = call(&state, &json!({"method": "nope", "params": {}})).expect_err("不认识");
    assert_eq!(error["code"], -32601);
}

#[test]
fn a_long_reply_comes_back_as_several_pages() {
    let (mut state, fake) = setup(false, &["cjk.otf"]);
    // 测的是分页，不是限时：调试版编出来的慢，和别的测试一起跑时撞得上 30 秒。
    state.limit = Duration::from_mins(5);
    let long = "这个群里说话的时候长回复会转成图。\n\n".repeat(400);
    let answer = markdown(&state, &json!({ "markdown": long })).expect("画得出");
    let pages = answer["pages"].as_array().expect("几页");
    assert!(pages.len() >= 2, "{answer}");
    let names: Vec<Value> = fake
        .put
        .lock()
        .expect("没 panic")
        .iter()
        .map(|put| put["name"].clone())
        .collect();
    assert_eq!(names.len(), pages.len());
    assert_eq!(names[1], "render-2.png");
}

#[test]
fn a_crash_is_caught_and_the_next_call_starts_over() {
    let (mut state, _) = setup(false, &["cjk.otf"]);
    let once = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let good = fonts(&["cjk.otf"]);
    let flag = Arc::clone(&once);
    state.fonts = Arc::new(move || {
        assert!(
            !flag.swap(false, std::sync::atomic::Ordering::SeqCst),
            "字体库读到一半崩了"
        );
        good()
    });
    let error = markdown(&state, &json!({"markdown": "你好"})).expect_err("崩了");
    assert_eq!(error["message"], "the renderer crashed");
    assert!(
        markdown(&state, &json!({"markdown": "你好"})).is_ok(),
        "下一次重开"
    );
}

#[test]
fn too_slow_gives_up() {
    let (mut state, _) = setup(false, &["cjk.otf"]);
    state.limit = Duration::ZERO;
    let error = markdown(&state, &json!({"markdown": "你好"})).expect_err("太久");
    assert_eq!(error["message"], "took too long to draw");
}

/// 审计第四轮（2026-11-02）：超时的那一张不等了，可画它的线程还拿着 `engine` 的锁一直画到完；后来的请求各开一个线程，排在锁后面，
/// 等锁的时间也算进它自己的限时：一句「你好」也跟着交回「太久」。一张画不完的（或画得极慢的）能让后面所有的都超时，排队的线程
/// 也一个个攒着。
#[test]
fn a_drawing_that_ran_over_does_not_hold_up_the_next() {
    let (mut state, _) = setup(false, &["cjk.otf"]);
    state.limit = Duration::from_secs(1);
    assert!(
        markdown(&state, &json!({"markdown": "你好"})).is_ok(),
        "短的一句一秒画得完"
    );
    // 调试版两万个字要画二十几秒：超时。
    let error = markdown(&state, &json!({"markdown": "字".repeat(MAX_CHARS)})).expect_err("太久");
    assert_eq!(error["message"], "took too long to draw");
    let next = markdown(&state, &json!({"markdown": "你好"}));
    assert!(next.is_ok(), "前一张超时了，后一句短的也跟着超时：{next:?}");
}

/// 没人要了还在画的占满了（字体库读不完，前几张都超时），又没有闲着的那一套：下一张当场回忙，不排队、不另开；读完了、还回来
/// 以后照常画。
#[test]
fn strays_fill_up_and_the_next_is_told_busy() {
    let (mut state, _) = setup(false, &["cjk.otf"]);
    state.limit = Duration::from_millis(200);
    let gate = Arc::new(std::sync::Mutex::new(()));
    let held = gate.lock().expect("没 panic");
    let (good, waiting) = (fonts(&["cjk.otf"]), Arc::clone(&gate));
    state.fonts = Arc::new(move || {
        drop(waiting.lock());
        good()
    });
    for _ in 0..pool::MAX_STRAY {
        let error = markdown(&state, &json!({"markdown": "你好"})).expect_err("读不完字体库");
        assert_eq!(error["message"], "took too long to draw");
    }
    let error = markdown(&state, &json!({"markdown": "你好"})).expect_err("忙");
    assert_eq!(error["message"], "the renderer is busy");
    drop(held);
    state.limit = Duration::from_mins(5);
    // 前几张读完字体库、画完还回来以后才不忙。
    let answer = (0..200).find_map(|_| {
        let answer = markdown(&state, &json!({"markdown": "你好"}));
        if answer
            .as_ref()
            .is_err_and(|error| error["message"] == "the renderer is busy")
        {
            std::thread::sleep(Duration::from_millis(50));
            return None;
        }
        Some(answer)
    });
    assert!(answer.is_some_and(|answer| answer.is_ok()), "读完了照常画");
}

#[test]
fn the_bus_name_render_reaches_the_same_method() {
    // 施工 R-11（核心设计 67，BUS-4）：总线上提供名字 `render`，核心照 `render.markdown` 转来；不带前缀的照旧登记着。
    let (state, _fake) = setup(false, &["cjk.otf"]);
    let by_name = super::call(
        &state,
        &json!({"method": "render.markdown", "params": {"markdown": "你好"}, "from": "im"}),
    )
    .expect("画得出");
    assert_eq!(by_name["pages"].as_array().map(Vec::len), Some(1));
    let registered = super::register();
    let names: Vec<&str> = registered["methods"]
        .as_array()
        .expect("是数组")
        .iter()
        .filter_map(|one| one["name"].as_str())
        .collect();
    assert_eq!(names, ["markdown", "render.markdown"]);
    // 清单里声明了名字、导出的是带前缀的那一个（核心照它认 `bus.call` 调不调得到）。
    let manifest = include_str!("../../package/package.toml");
    assert!(manifest.contains("provides = [\"render\"]"), "{manifest}");
    assert!(
        manifest.contains("methods = [\"render.markdown\"]"),
        "{manifest}"
    );
}
