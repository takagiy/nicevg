/**
 * `nicevg man`: reads the help page, in Vim's help format, by its tags and text.
 *
 * A section starts at a title line ending with its `*tag*` and holds the lines
 * up to the next title. Its subsections are the sections after it whose tags
 * start with its tag and `-`, so tag names alone give the nesting.
 */

/** A section as line indexes: its title, the end of its own lines, and the end of its subsections. */
type Section = { tag: string; start: number; own: number; end: number };

/** A range of lines to print. */
type Range = { start: number; end: number };

const TITLE = /\s\*([a-z0-9.-]+)\*$/;
const RULE = "=".repeat(78);
/** The table of contents, printed with no query; text is not searched in it, as it names every section. */
const CONTENTS = "nicevg-contents";
const HINT = `Use \`nicevg man <tag>\` to read a section with its subsections, or
\`nicevg man <text>\` to read the sections holding the text. Pass several
queries to read them at once. Read sections as you need them, not up front.`;

function sections(lines: string[]): Section[] {
  // The modeline closes the page.
  const pageEnd = lines.findIndex((line) => line.startsWith(" vim:"));
  const titles = lines.flatMap((line, at) => {
    const tag = TITLE.exec(line)?.[1];
    return tag ? [{ tag, at }] : [];
  });
  // Drops the blank lines and rules that separate a section from the next.
  const trim = (start: number, end: number) => {
    while (end > start + 1 && /^(\s*|=+|-+)$/.test(lines[end - 1]!)) end--;
    return end;
  };
  return titles.map(({ tag, at }, index) => {
    const next = titles[index + 1]?.at ?? pageEnd;
    const outside = titles.slice(index + 1).find((title) => !title.tag.startsWith(`${tag}-`))?.at ?? pageEnd;
    return { tag, start: at, own: trim(at, next), end: trim(at, outside) };
  });
}

/**
 * The help page's sections matching `queries`: a tag gives its section with
 * subsections, and text gives each section holding it in its own lines. Of
 * nested matches only the widest is kept, with every text emphasized by
 * `emphasize`. With no queries, the table of contents and how to query it.
 */
export function man(page: string, queries: string[], emphasize: (text: string) => string): { text: string; missing: string[] } {
  const lines = page.split("\n");
  const all = sections(lines);
  if (queries.length === 0) {
    const contents = all.find((section) => section.tag === CONTENTS)!;
    return { text: `${lines.slice(contents.start + 1, contents.own).join("\n").trim()}\n\n${HINT}\n`, missing: [] };
  }

  const ranges: Range[] = [];
  const texts: string[] = [];
  const missing: string[] = [];
  for (const query of queries) {
    const tagged = all.find((section) => section.tag === query);
    if (tagged) {
      ranges.push({ start: tagged.start, end: tagged.end });
      continue;
    }
    const needle = query.toLowerCase();
    const searched = all.filter((section) => section.tag !== CONTENTS);
    const holding = needle ? searched.filter((section) => lines.slice(section.start, section.own).some((line) => line.toLowerCase().includes(needle))) : [];
    if (holding.length === 0) {
      missing.push(query);
      continue;
    }
    texts.push(query);
    ranges.push(...holding.map((section) => ({ start: section.start, end: section.own })));
  }

  // Sections nest or are apart, so a range is inside the last kept one or after it.
  const kept: Range[] = [];
  for (const range of ranges.sort((a, b) => a.start - b.start || b.end - a.end)) {
    const last = kept.at(-1);
    if (!last || range.start >= last.end) kept.push(range);
  }

  // The longest text first, so a text holding another is emphasized whole.
  const escaped = [...texts].sort((a, b) => b.length - a.length).map((text) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const pattern = escaped.length > 0 ? new RegExp(escaped.join("|"), "gi") : undefined;
  const show = (line: string) => (pattern ? line.replace(pattern, emphasize) : line);
  const text = kept.map((range) => lines.slice(range.start, range.end).map(show).join("\n")).join(`\n\n${RULE}\n`);
  return { text: text ? `${text}\n` : "", missing };
}
