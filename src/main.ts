import { LibraryView } from "./components/library";
import { PlayerBar } from "./components/player-bar";
import { SettingsView } from "./components/settings";

const root = document.querySelector<HTMLDivElement>("#app")!;
root.innerHTML = `
  <header class="app-header">
    <h1>WaVoic</h1>
    <nav>
      <button data-tab="library">Library</button>
      <button data-tab="settings">Settings</button>
    </nav>
  </header>
  <main id="main"></main>
  <footer id="player"></footer>
`;

const player = new PlayerBar();
document.querySelector("#player")!.appendChild(player.el);
player.init();

const library = new LibraryView();
const settings = new SettingsView(() => player.init());
library.refresh();

const main = document.querySelector<HTMLElement>("#main")!;
function switchTab(tab: string) {
  main.innerHTML = "";
  if (tab === "library") main.appendChild(library.el);
  else if (tab === "settings") main.appendChild(settings.el);
}
document.querySelectorAll<HTMLButtonElement>("nav button").forEach((b) => {
  b.onclick = () => switchTab(b.dataset.tab!);
});
switchTab("library");
