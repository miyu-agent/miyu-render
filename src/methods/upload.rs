//! 把一页交给核心存成 blob（施工 R-10）：核心一行最多收 1 MiB，大一点的图整张写进 `blob.put` 的一行里就被拒（`line_too_long`），
//! 长文转图不出图。不超过 [`CHUNK`] 的照旧一次 `blob.put`；大的照核心的分块上传：`blob.open`、几次 `blob.write`（每块最多
//! [`CHUNK`]，核心定的）、`blob.close`，回的和 `blob.put` 一样。

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::json;

use super::Core;
use crate::wire::Answer;

/// 一块最多多少字节（核心 `blob.write` 的上限）；不超过它的一次交完。
pub const CHUNK: usize = 512 * 1024;

/// 把名字是 `name` 的 PNG `bytes` 交给核心，回 `blob.put` 那样的回应。
pub fn put(core: &dyn Core, name: &str, bytes: &[u8]) -> Answer {
    if bytes.len() <= CHUNK {
        return core.call(
            "blob.put",
            &json!({"data": STANDARD.encode(bytes), "name": name}),
        );
    }
    let opened = core.call(
        "blob.open",
        &json!({"name": name, "media_type": "image/png", "size": bytes.len()}),
    )?;
    let upload = opened["upload"]
        .as_str()
        .ok_or_else(|| json!({"message": "blob.open gave no upload"}))?;
    for (index, chunk) in bytes.chunks(CHUNK).enumerate() {
        core.call(
            "blob.write",
            &json!({"upload": upload, "offset": index * CHUNK, "data": STANDARD.encode(chunk)}),
        )?;
    }
    core.call("blob.close", &json!({ "upload": upload }))
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;

    use super::{CHUNK, put};
    use crate::test_support::setup;

    #[test]
    fn a_big_picture_goes_in_chunks_and_a_small_one_in_one_go() {
        // 假核心照真核心查：一行超过 1 MiB 的当场 panic。整张写进 `blob.put` 的（原来的做法）这一条转红。
        let (_, fake) = setup(false, &["cjk.otf"]);
        let big: Vec<u8> = (0u8..251).cycle().take(CHUNK * 2 + 4321).collect();
        let answer = put(fake.as_ref(), "render-1.png", &big).expect("交得进");
        assert_eq!(answer["blob"], "b1");
        let methods = fake.methods.lock().expect("没 panic").clone();
        assert_eq!(
            methods,
            [
                "blob.open",
                "blob.write",
                "blob.write",
                "blob.write",
                "blob.close"
            ]
        );
        let stored = fake.put.lock().expect("没 panic")[0].clone();
        assert_eq!(stored["name"], "render-1.png");
        let bytes = STANDARD
            .decode(stored["data"].as_str().expect("有内容"))
            .expect("是 base64");
        assert_eq!(bytes, big, "拼起来一个字节不差");

        let small = vec![7u8; CHUNK];
        put(fake.as_ref(), "render-2.png", &small).expect("交得进");
        let methods = fake.methods.lock().expect("没 panic").clone();
        assert_eq!(methods.last().map(String::as_str), Some("blob.put"));
        assert_eq!(methods.len(), 6, "正好一块的一次交完");
    }

    #[test]
    fn a_refusal_midway_comes_back_as_the_error() {
        let (_, fake) = setup(true, &["cjk.otf"]);
        let big = vec![1u8; CHUNK + 1];
        let error = put(fake.as_ref(), "render-1.png", &big).expect_err("核心拒了");
        assert_eq!(error["code"], -32010);
    }
}
