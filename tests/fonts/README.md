测试用的字体：测试不靠跑测试的机器装了什么字体（AGENTS.md 第 4 条），照这三份认、画。都裁小了、改了名，不和系统里的撞。

| 文件 | 字体族名 | 从哪来 | 许可 |
|---|---|---|---|
| `cjk.otf` | Miyu Test CJK | Noto Sans CJK SC Regular，只留 ASCII 和测试用到的几十个汉字、标点 | SIL Open Font License 1.1（`OFL-cjk.txt`） |
| `latin.ttf` | Miyu Test Latin | DejaVu Sans，只留 ASCII（没有中文：测「挑到的字体不含中文」） | DejaVu 的许可（`DejaVu-LICENSE.txt`） |
| `mono.ttf` | Miyu Test Mono | DejaVu Sans Mono，只留 ASCII | 同上 |

用 fonttools 的 `pyftsubset` 裁、改 `name` 表里的族名。要加字的，照原来的命令加上字重裁一遍。
