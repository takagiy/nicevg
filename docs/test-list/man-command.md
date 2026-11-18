# Test list — nicevg man (the help page)

`nicevg man` prints the help page, `bindings/js/doc/nicevg.txt`, in Vim's help file format, for agents drawing diagrams with `nicevg doc`.

## Command

- [x] When `nicevg man` runs, it can print the help page to stdout and exit 0.
- [x] When `nicevg man --toc` runs, it can print the table of contents alone.
- [x] When `nicevg --help` runs, it can name `nicevg man`.

## Help page

- [x] When the help page is read, every issue code but the port ones can be found as a `*nicevg-engine-issue-<code>*` tag.
- [x] When the help page is read, every option of `nicevg doc` can be found in `nicevg-cli-doc-input`.
- [x] When the help page is read, every `|link|` can be found as a `*tag*`.
- [x] When the style example in `nicevg-dsl-render-style` is drawn with `nicevg doc`, it can draw with no failures.
