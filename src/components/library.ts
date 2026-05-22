import { api, Category, LibraryItem } from "../api";
import { open } from "@tauri-apps/plugin-dialog";

export class LibraryView {
  el = document.createElement("div");
  items: LibraryItem[] = [];

  constructor() {
    this.el.className = "library";
    this.render();
  }

  async refresh() {
    this.items = await api.listLibrary();
    this.render();
  }

  async addFile() {
    const selected = await open({
      multiple: true,
      filters: [
        { name: "Audio", extensions: ["mp3", "wav", "flac", "ogg"] },
      ],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    for (const p of paths) {
      await api.addLibraryFile(p, "sfx");
    }
    await this.refresh();
  }

  render() {
    this.el.innerHTML = `
      <div class="library-toolbar">
        <button id="addBtn">+ Add Audio</button>
      </div>
      <table class="library-table">
        <thead><tr><th>Name</th><th>Category</th><th>Hotkey</th><th></th></tr></thead>
        <tbody>${this.items.map(this.row).join("")}</tbody>
      </table>
    `;
    this.el.querySelector<HTMLButtonElement>("#addBtn")!.onclick = () =>
      this.addFile();
    this.el
      .querySelectorAll<HTMLButtonElement>(".play-btn")
      .forEach((btn) => {
        btn.onclick = () => api.playTrack(btn.dataset.id!, false);
      });
    this.el
      .querySelectorAll<HTMLButtonElement>(".loop-btn")
      .forEach((btn) => {
        btn.onclick = () => api.playTrack(btn.dataset.id!, true);
      });
    this.el
      .querySelectorAll<HTMLButtonElement>(".del-btn")
      .forEach((btn) => {
        btn.onclick = async () => {
          await api.removeLibraryFile(btn.dataset.id!);
          await this.refresh();
        };
      });
  }

  row = (it: LibraryItem) => `
    <tr>
      <td>${escape(it.display_name)}</td>
      <td><select data-id="${it.id}" class="cat-sel">
        ${(["bgm", "sfx", "voiceline"] as Category[])
          .map(
            (c) =>
              `<option value="${c}"${c === it.category ? " selected" : ""}>${c}</option>`
          )
          .join("")}
      </select></td>
      <td>${it.hotkey ?? "—"}</td>
      <td>
        <button class="play-btn" data-id="${it.id}">▶</button>
        <button class="loop-btn" data-id="${it.id}">⟳</button>
        <button class="del-btn" data-id="${it.id}">✕</button>
      </td>
    </tr>`;
}

function escape(s: string) {
  return s.replace(
    /[&<>"']/g,
    (c) =>
      ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      }[c]!)
  );
}
