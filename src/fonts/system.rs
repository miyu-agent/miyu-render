//! 系统的通用名换成哪几个字体族（设计 36 第四节第 3 条）。Linux 问 fontconfig（`fc-match`，照中文挑无衬线）；macOS、Windows 没有
//! 这类别名，照系统默认的几个名字。问不到的（没装 fontconfig、命令失败）照常见的几个名字。

use std::process::Command;

use super::Generic;

/// 一个有中文的字体都没有时，说该装什么。
pub const INSTALL_HINT: &str = if cfg!(target_os = "macos") {
    "macOS ships PingFang SC; check that system fonts are not disabled"
} else if cfg!(windows) {
    "install Microsoft YaHei (it ships with Windows) or Noto Sans CJK"
} else {
    "install Noto Sans CJK (for example the noto-fonts-cjk package)"
};

/// 这台机器上的候选。
pub fn generic() -> Generic {
    if cfg!(target_os = "macos") {
        return Generic {
            sans: names(&["PingFang SC", "Hiragino Sans GB", "Heiti SC"]),
            mono: names(&["Menlo", "SF Mono", "Monaco"]),
            emoji: names(&["Apple Color Emoji"]),
        };
    }
    if cfg!(windows) {
        return Generic {
            sans: names(&["Microsoft YaHei", "Microsoft YaHei UI", "SimHei"]),
            mono: names(&["Consolas", "Cascadia Mono", "Courier New"]),
            emoji: names(&["Segoe UI Emoji"]),
        };
    }
    let mut sans = fc_match("sans-serif:lang=zh-cn")
        .into_iter()
        .collect::<Vec<_>>();
    sans.extend(names(&[
        "Noto Sans CJK SC",
        "Source Han Sans SC",
        "WenQuanYi Micro Hei",
        "DejaVu Sans",
    ]));
    let mut mono = fc_match("monospace").into_iter().collect::<Vec<_>>();
    mono.extend(names(&[
        "DejaVu Sans Mono",
        "Liberation Mono",
        "Noto Sans Mono",
    ]));
    let emoji = names(&[
        "Noto Color Emoji",
        "Twemoji",
        "JoyPixels",
        "Apple Color Emoji",
    ]);
    Generic { sans, mono, emoji }
}

/// 问 fontconfig 这个名字落到哪个族：交回它的第一个族名；问不到的是空的。
fn fc_match(pattern: &str) -> Option<String> {
    let output = Command::new("fc-match")
        .args(["-f", "%{family[0]}", pattern])
        .output()
        .ok()?;
    let name = String::from_utf8(output.stdout).ok()?;
    let name = name.trim();
    (output.status.success() && !name.is_empty()).then(|| name.to_string())
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_string()).collect()
}
