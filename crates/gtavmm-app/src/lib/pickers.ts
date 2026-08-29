// Thin wrappers around @tauri-apps/plugin-dialog's native file/folder pickers.
// Centralized here so every page uses the same real dialog calls instead of each
// page reaching into the plugin directly.
import { open } from "@tauri-apps/plugin-dialog";

/** Opens a native "pick a folder" dialog. Returns null if the user cancelled. */
export async function pickFolder(title?: string): Promise<string | null> {
  const result = await open({ directory: true, multiple: false, title });
  return typeof result === "string" ? result : null;
}

/** Opens a native "pick a file" dialog, optionally filtered by extension. */
export async function pickFile(
  extensions: string[],
  title?: string
): Promise<string | null> {
  const result = await open({
    directory: false,
    multiple: false,
    title,
    filters: extensions.length ? [{ name: "Files", extensions }] : undefined,
  });
  return typeof result === "string" ? result : null;
}
