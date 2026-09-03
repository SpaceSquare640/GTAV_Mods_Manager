/**
 * A very small syntax highlighter for the three file types the editor opens.
 *
 * Hand-written rather than pulling in a highlighting library: this project is
 * offline-first and ships no CDN assets, and the alternative would add a large
 * dependency to colour three simple formats. It produces the four span classes
 * the design already defines — `.cm` comment, `.tg` tag, `.st` string, `.kw`
 * keyword — and nothing else.
 *
 * It is a tokeniser for display only. It does not validate, and it will colour
 * some pathological input wrong; nothing depends on its output being correct.
 */
export type Language = "xml" | "ini" | "json" | "text";

export function languageFor(path: string): Language {
  const ext = path.split(".").pop()?.toLowerCase();
  if (ext === "xml" || ext === "meta") return "xml";
  if (ext === "ini" || ext === "cfg") return "ini";
  if (ext === "json") return "json";
  return "text";
}

/** Escapes first, always: the result is inserted as HTML. */
function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/**
 * Returns HTML for one line. Callers must not escape the input themselves —
 * this does it, and doing it twice would show `&amp;lt;` on screen.
 */
export function highlightLine(line: string, language: Language): string {
  const escaped = escapeHtml(line);
  switch (language) {
    case "xml":
      // One pass, comments first in the alternation. Two sequential replaces
      // would not do: the tag pattern also matches the text inside a comment,
      // so a second pass re-wraps what the first already wrapped and the
      // comment ends up painted as a tag.
      return escaped.replace(/(&lt;!--.*?--&gt;)|(&lt;[^&]*?&gt;)/g, (_m, comment, tag) =>
        comment
          ? `<span class="cm">${comment}</span>`
          : `<span class="tg">${tag.replace(
              /"[^"]*"/g,
              (q: string) => `<span class="st">${q}</span>`
            )}</span>`
      );
    case "ini":
      if (/^\s*[;#]/.test(escaped)) return `<span class="cm">${escaped}</span>`;
      if (/^\s*\[.*\]\s*$/.test(escaped)) return `<span class="tg">${escaped}</span>`;
      return escaped.replace(/^(\s*[^=]+)(=)(.*)$/, (_m, k, eq, v) =>
        `<span class="kw">${k}</span>${eq}<span class="st">${v}</span>`
      );
    case "json":
      return escaped
        .replace(/"(?:[^"\\]|\\.)*"(\s*:)?/g, (m, colon) =>
          colon ? `<span class="kw">${m}</span>` : `<span class="st">${m}</span>`
        )
        .replace(/\b(true|false|null)\b/g, (m) => `<span class="kw">${m}</span>`);
    default:
      return escaped;
  }
}
