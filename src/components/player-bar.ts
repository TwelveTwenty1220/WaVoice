import { api, AppConfig } from "../api";

export class PlayerBar {
  el = document.createElement("div");
  running = false;
  config: AppConfig | null = null;
  private meterTimer: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    this.el.className = "player-bar";
    this.render();
    this.startMeterLoop();
  }

  async init() {
    this.config = await api.getConfig();
    this.running = await api.engineStatus();
    this.render();
  }

  async toggle() {
    if (this.running) {
      await api.stopEngine();
    } else {
      try {
        await api.startEngine();
      } catch (e) {
        alert(`Start failed: ${e}`);
      }
    }
    this.running = await api.engineStatus();
    this.render();
  }

  startMeterLoop() {
    const tick = async () => {
      try {
        const m = await api.getMeters();
        const mic = this.el.querySelector<HTMLDivElement>(".meter-mic-fill");
        const out = this.el.querySelector<HTMLDivElement>(".meter-out-fill");
        if (mic) mic.style.width = `${Math.min(100, m.mic_rms * 300)}%`;
        if (out) out.style.width = `${Math.min(100, m.output_rms * 300)}%`;
      } catch {}
      this.meterTimer = setTimeout(tick, 100);
    };
    tick();
  }

  destroy() {
    if (this.meterTimer) clearTimeout(this.meterTimer);
  }

  render() {
    this.el.innerHTML = `
      <button id="toggleBtn">${this.running ? "■ Stop" : "▶ Start"}</button>
      <button id="stopAllBtn">⏹ Stop all sounds</button>
      <div class="meter"><div>Mic</div><div class="meter-bar"><div class="meter-mic-fill"></div></div></div>
      <div class="meter"><div>Out</div><div class="meter-bar"><div class="meter-out-fill"></div></div></div>
    `;
    this.el.querySelector<HTMLButtonElement>("#toggleBtn")!.onclick = () =>
      this.toggle();
    this.el.querySelector<HTMLButtonElement>("#stopAllBtn")!.onclick = () =>
      api.stopAll();
  }
}
