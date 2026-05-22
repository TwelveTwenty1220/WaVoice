import { invoke } from "@tauri-apps/api/core";

export type Category = "bgm" | "sfx" | "voiceline";

export interface LibraryItem {
  id: string;
  path: string;
  display_name: string;
  category: Category;
  hotkey: string | null;
}

export interface Devices {
  inputs: string[];
  outputs: string[];
  vb_cable: string | null;
}

export interface AppConfig {
  input_device: string | null;
  output_device: string | null;
  carrier_path: string | null;
  mic_gain: number;
  music_gain: number;
  carrier_gain: number;
  carrier_gate_rms: number;
  krisp_bypass_enabled: boolean;
}

export interface Meters {
  mic_rms: number;
  output_rms: number;
}

export const api = {
  listDevices: () => invoke<Devices>("list_devices"),
  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<void>("save_config", { config }),
  startEngine: () => invoke<void>("start_engine"),
  stopEngine: () => invoke<void>("stop_engine"),
  engineStatus: () => invoke<boolean>("engine_status"),
  playTrack: (id: string, looping: boolean) =>
    invoke<void>("play_track", { id, looping }),
  stopAll: () => invoke<void>("stop_all"),
  addLibraryFile: (path: string, category: Category) =>
    invoke<LibraryItem>("add_library_file", { path, category }),
  removeLibraryFile: (id: string) =>
    invoke<void>("remove_library_file", { id }),
  listLibrary: () => invoke<LibraryItem[]>("list_library"),
  setHotkey: (id: string, accelerator: string | null) =>
    invoke<void>("set_hotkey", { id, accelerator }),
  getMeters: () => invoke<Meters>("get_meters"),
};
