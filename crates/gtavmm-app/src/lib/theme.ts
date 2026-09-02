/**
 * Theme resolution.
 *
 * The stylesheet defines the dark palette twice — once behind
 * `@media (prefers-color-scheme: dark)` and once behind `[data-theme="dark"]` —
 * because plain CSS cannot share one declaration block between a media query
 * and an attribute selector. That duplication is a liability: a colour changed
 * in only one place would silently differ between "the system is dark" and
 * "the user chose dark".
 *
 * The application avoids the problem entirely by never relying on the media
 * query. It reads the OS preference once at startup and always writes an
 * explicit `data-theme`, so exactly one block is ever in play. The media query
 * stays in the stylesheet only so the design mockup behaves correctly when
 * opened as a bare file with no script.
 *
 * Persisting a user's choice needs a `theme` column on `user_settings`, which
 * does not exist yet; until then this follows the system every time.
 */

export type Theme = "system" | "dark" | "light";

/** Turns "system" into the concrete palette the OS is currently asking for. */
export function resolveTheme(preference: Theme): "dark" | "light" {
  if (preference !== "system") return preference;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/** Writes the resolved palette onto the root element. */
export function applyTheme(preference: Theme): void {
  document.documentElement.setAttribute("data-theme", resolveTheme(preference));
}

/**
 * Applies the theme at startup and keeps following the OS while the preference
 * is "system". Once a stored preference exists this should be called with it
 * instead, and the listener will stop mattering.
 */
export function applyStartupTheme(preference: Theme = "system"): void {
  applyTheme(preference);
  if (preference !== "system") return;

  const query = window.matchMedia?.("(prefers-color-scheme: dark)");
  // Safari below 14 only has the deprecated addListener; guard rather than assume.
  query?.addEventListener?.("change", () => applyTheme("system"));
}
