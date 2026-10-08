import type { ShellMemoryResult } from "./shellLayoutMemory.ts";

export type ShellMemoryRebuildResult = ShellMemoryResult | { ok: false; diagnostic: "user_cancelled" };
type RebuildKind = "layout" | "navigation";
type OwnerWindow = { isDestroyed(): boolean };
type RebuildDialogOptions = {
  type: "warning";
  title: string;
  message: string;
  detail: string;
  buttons: ["Cancel", "Rebuild"];
  defaultId: 0;
  cancelId: 0;
  noLink: true;
};

const cancelled: ShellMemoryRebuildResult = { ok: false, diagnostic: "user_cancelled" };

function dialogOptions(kind: RebuildKind): RebuildDialogOptions {
  return kind === "layout" ? {
    type: "warning",
    title: "Rebuild shell layout?",
    message: "Discard the saved shell layout?",
    detail: "This resets saved Sessions and inspector presentation settings.",
    buttons: ["Cancel", "Rebuild"], defaultId: 0, cancelId: 0, noLink: true,
  } : {
    type: "warning",
    title: "Rebuild workspace navigation?",
    message: "Discard saved workspace navigation?",
    detail: "This clears the remembered last central surface for every workspace.",
    buttons: ["Cancel", "Rebuild"], defaultId: 0, cancelId: 0, noLink: true,
  };
}

export async function confirmShellMemoryRebuild<TWindow extends OwnerWindow>(
  kind: RebuildKind,
  owner: TWindow | null,
  showMessageBox: (window: TWindow, options: RebuildDialogOptions) => Promise<{ response: number }>,
  rebuild: () => Promise<ShellMemoryResult>,
): Promise<ShellMemoryRebuildResult> {
  if (!owner || owner.isDestroyed()) return cancelled;
  const { response } = await showMessageBox(owner, dialogOptions(kind));
  if (response !== 1 || owner.isDestroyed()) return cancelled;
  return rebuild();
}
