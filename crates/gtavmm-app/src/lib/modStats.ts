import type { InstalledMod, ModStatus } from "../types";
import type { StatContext } from "../components/ModWorkspace";

/** Counts mods in one status. Curried so a stat card can name it directly. */
export function countByStatus(status: ModStatus) {
  return ({ mods }: StatContext) => mods.filter((m) => m.status === status).length;
}

/**
 * Mods whose install recorded a failure.
 *
 * The design's "Needs review" / "Unverified" cards want "mods that deserve a
 * second look". The engine has no such flag, so this stands in for it with the
 * nearest thing actually recorded: an install event that did not succeed. It is
 * an approximation and is documented as one rather than presented as a verdict.
 */
export function needsReview({ mods, failedModIds }: StatContext): number {
  return mods.filter((m: InstalledMod) => failedModIds.has(m.id)).length;
}
