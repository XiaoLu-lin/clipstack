export type ClipKind = "text" | "link" | "color" | "image" | "file";

export interface ClipItem {
  id: number;
  kind: ClipKind;
  text: string;
  meta: string | null;
  thumb: string | null;
  source: string | null;
  sourceIcon: string | null;
  pinned: boolean;
  hash: string;
  createdAt: number;
}

export interface Status {
  paused: boolean;
  capacity: number;
  autostart: boolean;
  theme: string;
  autoPaste: boolean;
  hotkey: string;
}

export interface SourceApp {
  bundle: string;
  name: string;
  icon: string | null;
}
