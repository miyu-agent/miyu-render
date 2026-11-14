# miyu-render

> <sub>本文档由 Anthropic 的 Claude Fable 5.2 生成。</sub>

Miyu 的渲染软件包（编号 `render`）：把 Markdown 画成 PNG 图，代码上色、表格、公式、Mermaid 图都画。主要给通讯平台用，回复太长时 im 把它画成图发出去；在终端、网页里也能让 AI 画。

## 装

先装好 Miyu，再：

```sh
miyu pkg install render
```

装上就跑，不用打开，不用下模型。能装在 `linux-x86_64`、`macos-aarch64`、`windows-x86_64` 上。装了多出这几样：

- AI 用的工具 `render_image`：把一段 Markdown 或工作区里的 `.md` 文件画成图，存成文件，交回路径。
- 总线方法 `render.markdown`：别的软件包交 Markdown，拿回一页页图。im 的长回复转图用的就是它。
- 设置键 `render.theme`、`render.font`、`render.code_font`、`render.font_size`。

能画什么、图存在哪、每项设置是什么、出了问题怎么办，见 [附加软件：渲染](https://github.com/miyu-agent/miyu/blob/main/docs/guide/addons.md#渲染-render)。

字体用系统里装着的，不下载。一个有中文的字体都没有的画不了：Linux 装 Noto Sans CJK（例如 `noto-fonts-cjk` 包）。

## 开发

一个 Rust 程序 `miyu-render`，只经协议和核心说话，不依赖核心的 crate。包文件夹是 `package/`，里面只有清单 `package.toml`。

| 目录 | 是什么 |
|---|---|
| `src/markdown/` | 读 Markdown（pulldown-cmark） |
| `src/render/` | 排字、画图（cosmic-text、resvg；公式 ratex，KaTeX 的字体编进程序；Mermaid 用核心仓库 `third_party/mermaid-rs-renderer` 的那一份，照 `Cargo.toml` 钉住一个提交） |
| `src/fonts/` | 挑系统字体：Linux 问 fontconfig，macOS、Windows 照默认的字体名 |
| `src/tools/`、`src/methods/` | 工具 `render_image`、总线方法 `render.markdown` |
| `tests/protocol.rs` | 拉起 `miyu-render serve`，照核心那样说协议 |
| `tests/fonts/` | 测试用的字体，裁小、改了名；从哪来、许可证见 `tests/fonts/README.md` |
| `docs/prompt-ledger.md` | 给模型看的字（工具说明、参数说明、工具结果）的登记簿 |

测试不靠跑测试的机器装了什么字体，只用 `tests/fonts/` 里的。

要 Rust（版本照 `rust-toolchain.toml`，`rustup` 会自己装）。CI（`.github/workflows/ci.yml`）在 Linux、macOS、Windows 上跑：

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --locked
cargo test --locked --no-fail-fast
```

另查每个 `.rs` 不超过 500 行，测试也算。

在本机试装：编好程序放进包文件夹的 `bin/`，再把包文件夹拷成名叫 `render` 的文件夹装上（包的编号照文件夹名定）：

```sh
cargo build --release --locked
mkdir -p package/bin
cp target/release/miyu-render package/bin/
cp -R package /tmp/render
miyu pkg install /tmp/render
```

Windows 上程序带 `.exe`。发版推 `v<版本>` 的标签，`.github/workflows/release.yml` 一种系统打一个 `.miyu` 传进 Release，标签的版本照 `package/package.toml` 的 `version`（`Cargo.toml` 的版本跟着一起改）。

## 许可证

AGPL-3.0-or-later，见 `LICENSE`。依赖照各自的 `license`；测试字体照 `tests/fonts/` 里随附的许可证。
