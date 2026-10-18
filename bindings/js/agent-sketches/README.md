# Agent sketches

169 sketches in nicevg's TypeScript DSL, each a diagram of a real, public
project (Kubernetes' scheduler, PostgreSQL's catalogs, WordPress' schema,
the Rust release train ...), written by an AI agent (Codex) from the
project's own docs and code. `INDEX.md` lists them with what each explains;
each file's header names the project and the pages it was drawn from.

They are material for trying nicevg on diagrams an agent actually drafts:
nodes placed by hand with `x` and `y`, the edges between them, and the
styles that draw them. None has been rendered or repaired; they are first
drafts, crowded, misaligned or overlapping as their author left them.

## How they were made

One sketch a round. Each round named what the diagram should explain (a
request's path, what a project stores, how a change reaches a release,
how it is deployed ...), a rough size, what a node should stand for and
which part of the project to show, and a few loose leanings in how to
draft it (how neat, plain boxes or shapes of its own, how much the edges
say, short or full names, Japanese labels now and then). The agent chose
the project, looked it up on the web, and chose the kind of diagram and
its look. `styles/er.ts` was given to it: tables with a port on each
column's row, for edges from a foreign key's column to the key.

Each sketch was checked only for syntax and types (`tsc`) and for edges
and parents that name real nodes or ports; none was run.

## Checking them

From `bindings/js`:

    node_modules/.bin/tsc --noEmit -p agent-sketches

The folder is not part of the package.
