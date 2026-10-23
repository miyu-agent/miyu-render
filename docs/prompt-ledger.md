## 给模型看的字（登记簿）

这个包发给模型的每一处字。改了照核心仓库的规矩重量 token、写明为什么要。token 照 o200k 数（tiktoken，`o200k_base`）。

| 哪一处 | 原文在哪 | 什么时候发 | token | 为什么要 |
|---|---|---|---|---|
| `render_image` 的说明、参数 | `src/tools.rs` | 每一轮跟着工具面（预设开着这个包时） | 95（整份 `{name, description, input_schema}` 的 JSON；说明 35） | 说明两句：能画什么（表格、mermaid、公式、着色的代码，她才知道哪些内容值得画）；图只存成文件、她不展示就没人看得到（R-8 起不说存在哪：有 `pictures` 的存那里，存哪看结果里的路径）（最容易踩的坑：以为画了就是发了，2026-10-17 项目主人定只生成文件）。参数 `markdown` 名字说清了不写；`path` 一句（只认工作区里的 `.md`）。不列 `required`：二选一，写在报错里 |
| 结果：每一页一行 | `src/tools/render_image.rs` | 画成了 | 每页约 20 到 30（工作区的按 `/w/rendered/render-<12 位>-1.png 1088x413` 18；家目录的看数据根有多长，`/home/u/.miyu/home/owner/pictures/…` 26） | 路径给她调 `show`，长宽让她知道图多大；同一份字、同样的配置画过的交回原来的文件 |
| 报错：参数 | `src/tools/render_image.rs` | 两个都给了、都没给 | 8、5 | `Give markdown or path, not both.`、`Give markdown or path.` |
| 报错：`path` | `src/tools/render_image.rs` | 工作区外、不是 `.md`、太大、读不出 | 约 6 到 17 | 说清哪一条不行，她能换个做法（把字直接给、挪进工作区） |
| 报错：存图 | `src/tools/saved.rs` | `rendered/` 建不了、指到工作区外、写不进；`pictures` 建不了、是链接（`The pictures folder is a link.` 7） | 约 7 到 15（`The pictures folder is not a folder.` 8，兜底，正常走不到：是普通文件的建的时候就报错） | 同上 |
| 报错：画 | `src/methods.rs` 交回的原因，首字母大写、加句号 | 太长、没有中文字体、画崩了、太久 | 约 5 到 25 | 和 `markdown` 方法同一句；没有中文字体的那句说清该装什么 |
| 报错：没有工作区、不认识的工具名 | `src/tools.rs`、`src/tools/render_image.rs` | 核心没给 `cwd`、工具名对不上 | 6、约 6 | 兜底，正常走不到 |
