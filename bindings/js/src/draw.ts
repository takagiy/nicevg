/**
 * Drawing a node's picture for `render()`'s `style`, as SVG written out:
 *
 * ```ts
 * const store = ({ no, label }) => {
 *   const id = text(no), name = text(label);
 *   const sep = id.width + 24, w = sep + name.width + 24, h = Math.max(id.height, name.height) + 20;
 *   return svg`
 *     <path d="M ${w} 0 H 0 V ${h} H ${w}" fill="none" stroke="#333"/>
 *     <line x1="${sep}" y1="0" x2="${sep}" y2="${h}" stroke="#333"/>
 *     ${id.at({ cx: sep / 2, cy: h / 2 })}
 *     ${name.at({ left: sep + 12, cy: h / 2 })}`;
 * };
 * ```
 */

const escape = (value: string) =>
  value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** SVG written with `svg`: embedded as it is, where strings are escaped. */
export class SvgFragment {
  readonly #markup: string;

  constructor(markup: string) {
    this.#markup = markup;
  }

  toString(): string {
    return this.#markup;
  }

  /** This fragment with its origin moved to `x`, `y`. */
  at(x: number, y: number): SvgFragment {
    return svg`<g transform="translate(${x} ${y})">${this}</g>`;
  }
}

function embed(value: unknown): string {
  if (value instanceof SvgFragment || value instanceof Text) {
    return value.toString();
  }
  if (Array.isArray(value)) {
    return value.map(embed).join("");
  }
  return escape(String(value));
}

/** SVG markup with values embedded: strings escaped, fragments, texts and arrays of them as they are. */
export function svg(strings: TemplateStringsArray, ...values: unknown[]): SvgFragment {
  return new SvgFragment(strings.reduce((markup, string, index) => markup + embed(values[index - 1]) + string));
}

// shortcut: the common wide ranges (CJK, kana, hangul, fullwidth forms), not every East Asian Wide character; widen it if a script measures short.
const WIDE = /[ᄀ-ᅟ⺀-〾ぁ-㏿㐀-䶿一-鿿ꀀ-꓏가-힣豈-﫿︰-﹏＀-｠￠-￦]|[\u{20000}-\u{3fffd}]/u;

/** Where a text goes: one of `left`, `cx`, `right` across, one of `top`, `cy`, `bottom` down. */
export type TextPlace = ({ left: number } | { cx: number } | { right: number }) &
  ({ top: number } | { cy: number } | { bottom: number });

/** A line of text, sized the way nicevg measures text, placed with `at`. */
export class Text {
  /** Its width, wide characters one font size each and others 0.59. */
  readonly width: number;
  /** Its line's height, 1.2 font sizes. */
  readonly height: number;

  constructor(
    readonly content: string,
    readonly size = 14,
    readonly place: TextPlace = { left: 0, top: 0 },
  ) {
    let width = 0;
    for (const character of content) {
      width += WIDE.test(character) ? size : character.length * size * 0.59;
    }
    this.width = Math.ceil(width);
    this.height = Math.ceil(size * 1.2);
  }

  /** This text with the point of its box named in `place` at the coordinates given. */
  at(place: TextPlace): Text {
    const across = ["left", "cx", "right"].filter((key) => key in place);
    const down = ["top", "cy", "bottom"].filter((key) => key in place);
    if (across.length !== 1 || down.length !== 1) {
      throw new TypeError(
        `text "${this.content}": at() takes one of left, cx, right and one of top, cy, bottom, not ${Object.keys(place).join(", ") || "none"}`,
      );
    }
    return new Text(this.content, this.size, place);
  }

  /** The `<text>` element, centred on its line by `dominant-baseline`. */
  toString(): string {
    const place = this.place as Record<string, number>;
    const [x, anchor] =
      "cx" in place ? [place.cx!, "middle"] : "right" in place ? [place.right!, "end"] : [place.left!, "start"];
    const y = "cy" in place ? place.cy! : "top" in place ? place.top! + this.height / 2 : place.bottom! - this.height / 2;
    return `<text x="${x}" y="${y}" font-size="${this.size}" text-anchor="${anchor}" dominant-baseline="central">${escape(this.content)}</text>`;
  }
}

/** A line of text at `size` (14 by default), to embed in `svg` once placed with `at`. */
export function text(content: string, size = 14): Text {
  return new Text(content, size);
}
