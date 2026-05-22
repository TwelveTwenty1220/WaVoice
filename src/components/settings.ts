import { api, AppConfig, Devices } from "../api";
import { open } from "@tauri-apps/plugin-dialog";

export class SettingsView {
  el = document.createElement("div");
  config: AppConfig | null = null;
  devices: Devices | null = null;
  onChange: () => void;

  constructor(onChange: () => void) {
    this.el.className = "settings";
    this.onChange = onChange;
    this.load();
  }

  async load() {
    this.devices = await api.listDevices();
    this.config = await api.getConfig();
    this.render();
  }

  async save() {
    if (!this.config) return;
    await api.saveConfig(this.config);
    this.onChange();
  }

  async pickCarrier() {
    const sel = await open({
      multiple: false,
      filters: [
        { name: "Audio", extensions: ["wav", "mp3", "flac", "ogg"] },
      ],
    });
    if (sel && typeof sel === "string" && this.config) {
      this.config.carrier_path = sel;
      await this.save();
      this.render();
    }
  }

  render() {
    if (!this.config || !this.devices) {
      this.el.innerHTML = "Loading…";
      return;
    }
    const c = this.config;
    const d = this.devices;
    const vbWarning = d.vb_cable
      ? ""
      : `<div style="background:#5a2a2a;padding:8px;border-radius:4px;margin-bottom:12px;">
        VB-Cable not detected. Install from <a href="https://vb-audio.com/Cable/" target="_blank" style="color:#9af">vb-audio.com/Cable</a> and restart Windows.
      </div>`;

    this.el.innerHTML = `
      <h2>Settings</h2>
      ${vbWarning}

      <label>Input Device (your real microphone)
        <select id="inputSel">
          <option value="">—</option>
          ${d.inputs
            .map(
              (n) =>
                `<option value="${n}"${n === c.input_device ? " selected" : ""}>${n}</option>`
            )
            .join("")}
        </select>
      </label>

      <label>Output Device (must be VB-Cable Input)
        <select id="outputSel">
          <option value="">—</option>
          ${d.outputs
            .map(
              (n) =>
                `<option value="${n}"${n === c.output_device ? " selected" : ""}>${n}</option>`
            )
            .join("")}
        </select>
      </label>

      <label><input type="checkbox" id="krispBox"${
        c.krisp_bypass_enabled ? " checked" : ""
      }/> Krisp bypass (recommended)</label>

      <label>Carrier sample file (optional, used when you're silent)
        <div style="display:flex;gap:8px;">
          <input type="text" value="${c.carrier_path ?? ""}" readonly style="flex:1"/>
          <button id="pickCarrierBtn">Browse…</button>
        </div>
      </label>

      <label>Mic gain: <span id="micVal">${c.mic_gain.toFixed(2)}</span>
        <input type="range" min="0" max="2" step="0.05" value="${c.mic_gain}" id="micGain"/></label>
      <label>Music gain: <span id="musVal">${c.music_gain.toFixed(2)}</span>
        <input type="range" min="0" max="2" step="0.05" value="${c.music_gain}" id="musicGain"/></label>
      <label>Carrier gain: <span id="carVal">${c.carrier_gain.toFixed(2)}</span>
        <input type="range" min="0" max="0.5" step="0.005" value="${c.carrier_gain}" id="carrierGain"/></label>
      <label>Carrier gate (RMS threshold): <span id="gateVal">${c.carrier_gate_rms.toFixed(
        3
      )}</span>
        <input type="range" min="0" max="0.1" step="0.001" value="${c.carrier_gate_rms}" id="gateRms"/></label>

      <div style="display:flex;gap:8px;margin-top:12px;">
        <button id="applyBtn">Apply (restart engine)</button>
      </div>
    `;

    const bindSel = (id: string, key: keyof AppConfig) => {
      this.el.querySelector<HTMLSelectElement>(`#${id}`)!.onchange = (e) => {
        (this.config as any)[key] =
          (e.target as HTMLSelectElement).value || null;
        this.save();
      };
    };
    bindSel("inputSel", "input_device");
    bindSel("outputSel", "output_device");

    this.el.querySelector<HTMLInputElement>("#krispBox")!.onchange = (e) => {
      this.config!.krisp_bypass_enabled = (e.target as HTMLInputElement).checked;
      this.save();
    };

    this.el.querySelector<HTMLButtonElement>("#pickCarrierBtn")!.onclick =
      () => this.pickCarrier();

    const bindRange = (id: string, key: keyof AppConfig, lbl: string) => {
      const el = this.el.querySelector<HTMLInputElement>(`#${id}`)!;
      el.oninput = () => {
        const v = parseFloat(el.value);
        (this.config as any)[key] = v;
        this.el.querySelector<HTMLSpanElement>(`#${lbl}`)!.innerText =
          v.toFixed(key.includes("gate") ? 3 : 2);
      };
      el.onchange = () => this.save();
    };
    bindRange("micGain", "mic_gain", "micVal");
    bindRange("musicGain", "music_gain", "musVal");
    bindRange("carrierGain", "carrier_gain", "carVal");
    bindRange("gateRms", "carrier_gate_rms", "gateVal");

    this.el.querySelector<HTMLButtonElement>("#applyBtn")!.onclick = async () => {
      await api.stopEngine();
      try {
        await api.startEngine();
      } catch (e) {
        alert(`Start failed: ${e}`);
      }
      this.onChange();
    };
  }
}
