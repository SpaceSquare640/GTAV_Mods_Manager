export type Mode = "legacy" | "enhanced" | "fivem";
export type Sub = "mods" | "lspdfr" | "client" | "server" | "converter";

export type ModStatus = "Active" | "Disabled" | "Uninstalled";

export interface InstalledMod {
  id: number;
  name: string;
  source_type: string;
  install_path: string;
  installed_at: string;
  status: ModStatus;
  notes: string | null;
  link: string | null;
}

export type DetectGameResult =
  | { status: "found"; install_path: string; edition: string }
  | { status: "not_found" };
