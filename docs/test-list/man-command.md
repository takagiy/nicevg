# Test list — nicevg man (the help page)

`nicevg man` prints the help page, `bindings/js/doc/nicevg.txt`, in Vim's help file format, for agents drawing diagrams with `nicevg doc` and `nicevg run`.

## Command

- [x] When `nicevg man` runs with no query, it can print the table of contents and how to query it.
- [x] When `nicevg man` runs with no query, it can list the tag of each issue in the table of contents.
- [x] When a query is a tag, `nicevg man` can print that section with its title line and subsections.
- [x] When a query is text in a section, `nicevg man` can print that section with its title line, without the sections inside or outside it, the text emphasized.
- [x] When text is in the table of contents too, `nicevg man` can leave the table of contents out.
- [x] When matches of several queries nest, `nicevg man` can print only the widest, once, with the emphasis of all.
- [x] When matches do not nest, `nicevg man` can print each in page order, separated by a rule.
- [x] When a query matches nothing, `nicevg man` can report it and exit 1.
- [x] When `nicevg --help` runs, it can name `nicevg man`.

## Help page

- [x] When the help page is read, every issue code but the port ones can be found as a `*nicevg-engine-issue-<code>*` tag.
- [x] When the help page is read, every option of `nicevg doc` can be found in `nicevg-cli-doc-input`.
- [x] When the help page is read, every option of `nicevg run` can be found in `nicevg-cli-run-input`.
- [x] When the help page is read, every `|link|` can be found as a `*tag*`.
- [x] When the style example in `nicevg-dsl-render-style` is drawn with `nicevg doc`, it can draw with no failures.
