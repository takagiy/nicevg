# Test list — nicevg run (one DSL script to one SVG)

`nicevg run <script> [options]` runs one DSL script in the sandbox, writes the SVG it renders, and prints a JSON report like `nicevg doc`'s: `output`, `failures`, `remainingIssues` and `console.<method>`. Paths in it are relative to the current directory.

## Input and output

- [x] When `nicevg run <script>` runs, it can write the SVG beside the script, its extension replaced by `.svg`, and print the report to stdout.
- [x] When `-o <file>` is given, it can write the SVG there, overwriting what was there.
- [x] When `-o -` is given, it can write the SVG to stdout and the report to stderr.
- [x] When `-s <code>` is given, it can draw that DSL to `out.svg` in the current directory.
- [x] When `-s -` is given, it can read the DSL from stdin.
- [x] When both or neither of `<script>` and `-s` are given, it can print the usage and exit 2.
- [x] When the script file does not exist, it can say so and exit 2.
- [x] When the SVG would overwrite the script, it can refuse and exit 2.
- [x] When run in a subdirectory of the sandbox, the report's paths can be relative to the current directory.

## The script

- [x] When the script writes to the console, the report can hold each line as `<file>:<line>:<column>: <message>` under `console.<method>`, the place being where it was called.
- [x] When the script throws, it can report the failure with the stack, write nothing, keep the old SVG and exit 2.
- [x] When the diagram has issues left, the report can list them and the command exit 0.
- [x] When the script imports a file in the sandbox directory, it can draw; outside it, it can fail.
- [x] When the script runs past `--timeout`, it can fail.
