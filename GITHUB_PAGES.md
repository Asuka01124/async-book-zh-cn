# 在线阅读与自动发布

- [在线阅读全书](https://Asuka01124.github.io/async-book-zh-cn/)
- [从中文指南开始阅读](https://Asuka01124.github.io/async-book-zh-cn/part-guide/intro.html)
- [查看发布记录](https://github.com/Asuka01124/async-book-zh-cn/actions/workflows/pages.yml)

网站由 mdBook 生成，提供章节目录、全文搜索、主题切换和适配手机的阅读页面。书前导言和部分尚未翻译的章节目前保留英文；已完成的中文章节见 README 中的逐章记录。

## 自动更新

每次向 `codex/zh-cn` 分支推送提交后，`.github/workflows/pages.yml` 会构建书籍、检查已译章节及本地链接，再把 `book/` 发布到 GitHub Pages。构建或检查失败时，不发布这一版。

也可以在仓库 **Actions → 发布中文书籍到 GitHub Pages → Run workflow** 中选择 `codex/zh-cn` 分支手动发布。其他分支不会部署。

## 首次启用或迁移

在仓库 **Settings → Pages → Build and deployment → Source** 中选择 **GitHub Actions**，然后运行发布工作流。工作流使用 GitHub 自动提供的 `GITHUB_TOKEN` 和 OIDC 身份验证，无需新增个人令牌或仓库 Secret。

当前站点路径为 `/async-book-zh-cn/`，与 `book.toml` 中的 `output.html.site-url` 一致。仓库改名时，需要一起调整该配置与本文的阅读链接；发布分支改名时，需要调整工作流的分支触发条件和 `if` 条件。

## 本地预览

```sh
mdbook serve --open
```

本地生成的网站位于 `book/`，不需要把生成文件提交到 Git。
