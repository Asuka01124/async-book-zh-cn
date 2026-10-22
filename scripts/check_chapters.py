"""检查已译章节及 mdBook HTML；仅使用 Python 标准库，不访问外网。"""

from __future__ import annotations

import argparse
from html.parser import HTMLParser
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
CHAPTERS = (
    "part-guide/intro.md", "part-guide/concurrency.md",
    "part-guide/async-await.md", "part-guide/more-async-await.md",
    "part-guide/io.md", "part-guide/concurrency-primitives.md",
)


class Page(HTMLParser):
    def __init__(self, path: Path):
        super().__init__()
        self.ids: set[str] = set()
        self.links: list[str] = []
        self.feed(path.read_text(encoding="utf-8"))

    def handle_starttag(self, tag, attrs):
        values = dict(attrs)
        if values.get("id"):
            self.ids.add(values["id"])
        if tag == "a" and values.get("name"):
            self.ids.add(values["name"])
        if tag == "a" and values.get("href"):
            self.links.append(values["href"])


def check(chapters: list[str], output: Path) -> list[str]:
    errors: list[str] = []
    cache: dict[Path, Page] = {}
    for chapter in chapters:
        source = ROOT / "src" / chapter
        if not source.is_file():
            errors.append(f"{chapter}: 章节源文件不存在")
            continue
        for number, line in enumerate(source.read_text(encoding="utf-8").splitlines(), 1):
            if re.search(r"\b(?:TODO|FIXME)\b|todo!\s*\(|unimplemented!\s*\(", line):
                errors.append(f"{chapter}:{number}: 仍有未完成标记")
            if re.search(r"\]\(\s*\)", line):
                errors.append(f"{chapter}:{number}: 仍有空链接")
        html = (output / Path(chapter).with_suffix(".html")).resolve()
        if not html.is_file():
            errors.append(f"{chapter}: 找不到 HTML，请先运行 mdbook build")
            continue
        if html not in cache:
            cache[html] = Page(html)
        for href in cache[html].links:
            url = urlsplit(href)
            if url.scheme or url.netloc or url.path.startswith("/"):
                continue
            target = (html.parent / unquote(url.path)).resolve() if url.path else html
            if not target.exists():
                errors.append(f"{chapter}: 本地链接不存在：{href}")
                continue
            if url.fragment and target.suffix == ".html":
                if target not in cache:
                    cache[target] = Page(target)
                if unquote(url.fragment) not in cache[target].ids:
                    errors.append(f"{chapter}: 锚点不存在：{href}")
    return sorted(set(errors))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("chapters", nargs="*", help="相对 src/ 的章节路径")
    parser.add_argument("--output", type=Path, default=ROOT / "book")
    args = parser.parse_args()
    chapters = args.chapters or list(CHAPTERS)
    errors = check(chapters, args.output)
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        print(f"检查失败：{len(errors)} 个问题。", file=sys.stderr)
        return 1
    print(f"检查通过：{len(chapters)} 章，无未完成标记或无效本地链接。")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
