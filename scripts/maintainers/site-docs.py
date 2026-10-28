"""Turn the plugin guides of this repository into articles for the formacad.ru documentation site.

usage: python scripts/maintainers/site-docs.py <site content dir> <commit sha> [repository]

Reads docs/{ru,en}/{plugins,plugin-authoring,plugin-platform}.md of the current checkout, drops the
repository navigation line, rewrites links between these guides to site paths (/docs/...) with
site-style anchors and points every other relative link to GitHub at the given commit.
Writes <site content dir>/<locale>/<article>.md. The site repository itself is FormaCAD-Server.
"""
import os
import posixpath
import re
import sys

out, sha = sys.argv[1:3]
repository = sys.argv[3] if len(sys.argv) > 3 else "lovlyDev/FormaCAD"
cad = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
blob = f"https://github.com/{repository}/blob/{sha}/"
tree = f"https://github.com/{repository}/tree/{sha}/"
articles = {
    "plugins.md": "plugins/plugins",
    "plugin-authoring.md": "plugins/authoring",
    "plugin-platform.md": "developers/plugin-platform",
    "command-api.md": "developers/command-api",
}
converted = ("plugins.md", "plugin-authoring.md", "plugin-platform.md")


def rewrite(locale: str, target: str) -> str:
    path, _, anchor = target.partition("#")
    anchor = f"#{anchor}" if anchor else ""
    if path.startswith(("http://", "https://", "mailto:")) or not path:
        return target
    if path in articles:
        return f"/docs/{articles[path]}{anchor}"
    repo_path = posixpath.normpath(posixpath.join(f"docs/{locale}", path))
    base = tree if not posixpath.splitext(repo_path)[1] else blob
    return f"{base}{repo_path}{anchor}"


def github_slug(heading: str) -> str:
    return re.sub(r"[^\w\- ]", "", heading.strip().lower()).replace(" ", "-")


def site_slug(heading: str) -> str:
    return re.sub(r"^-|-$", "", re.sub(r"[^\w]+|_", "-", heading.lower()))


def read(locale: str, name: str) -> str:
    with open(os.path.join(cad, "docs", locale, name), encoding="utf-8") as source:
        return source.read()


def site_anchors(locale: str, name: str) -> dict:
    heads = re.findall(r"^#{2,6} (.+)$", read(locale, name), re.M)
    return {github_slug(h): site_slug(h.replace("`", " ").replace("*", " ")) for h in heads}


for locale in ("ru", "en"):
    anchors = {articles[name]: site_anchors(locale, name) for name in converted}

    def fix_anchor(match):
        article, anchor = match.group(1), match.group(2)
        mapped = anchors.get(article, {}).get(anchor)
        if not mapped and article in anchors:
            raise SystemExit(f"unknown anchor {article}#{anchor}")
        return f"](/docs/{article}#{mapped or anchor})"

    for name in converted:
        lines = read(locale, name).split("\n")
        # Drop the repository navigation line (Documentation · English · …).
        if len(lines) > 2 and lines[2].startswith("[") and " · " in lines[2]:
            del lines[1:3]
        text = "\n".join(lines)
        text = re.sub(r"\]\(([^)\s]+)\)", lambda m: f"]({rewrite(locale, m.group(1))})", text)
        text = re.sub(r"\]\(/docs/([^)#\s]+)#([^)\s]+)\)", fix_anchor, text)
        dest = os.path.join(out, locale, f"{articles[name]}.md")
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        with open(dest, "w", encoding="utf-8", newline="\n") as target:
            target.write(text)
        print(dest, len(text))
