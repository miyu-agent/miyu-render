//! 审计第四轮（2026-11-02）：拉起 `miyu-render serve`，照核心那样说协议。

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};

/// 读循环（`main.rs` 的 `read`）碰到一行不是 UTF-8 的就 `break`：标准输入照样开着，整个包却不再读、退出了，后面的请求一条也
/// 不回，核心只看到它没了。坏的一行应该跳过（同读不懂的 JSON），接着读下一行。
#[test]
fn a_line_that_is_not_utf8_is_skipped_and_the_next_is_answered() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_miyu-render"))
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("起得来");
    let mut input = child.stdin.take().expect("有标准输入");
    let output = BufReader::new(child.stdout.take().expect("有标准输出"));
    let (tell, lines) = mpsc::channel::<Value>();
    std::thread::spawn(move || {
        for line in output.lines().map_while(Result::ok) {
            if let Ok(value) = serde_json::from_str(&line)
                && tell.send(value).is_err()
            {
                break;
            }
        }
    });
    let next = || lines.recv_timeout(Duration::from_secs(10)).ok();
    // 握手：hello、package.methods、provide 各回一个空的结果。
    for _ in 0..3 {
        let asked = next().expect("包先发请求");
        writeln!(
            input,
            "{}",
            json!({"jsonrpc": "2.0", "id": asked["id"], "result": {}})
        )
        .expect("送得进");
    }
    input.write_all(b"\xff\xfe\n").expect("送得进");
    writeln!(
        input,
        "{}",
        json!({"jsonrpc": "2.0", "id": 7, "method": "nope", "params": {}})
    )
    .expect("送得进");
    input.flush().expect("送得进");
    let answer = next();
    drop(input);
    drop(child.kill());
    drop(child.wait());
    assert_eq!(
        answer.as_ref().map(|answer| answer["id"].clone()),
        Some(json!(7)),
        "坏的一行以后不回了：{answer:?}"
    );
}
