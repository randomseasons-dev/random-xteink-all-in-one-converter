import { useState } from "react";
import MangaTab from "./MangaTab";
import WallpaperTab from "./WallpaperTab";
import "./App.css";

const TABS = ["Manga converter", "Wallpapers creator", "coming soon..."];
const TAGS = ["offline · .xtch", "offline · .bmp", "offline"];

export default function App() {
  const [tab, setTab] = useState(0);

  return (
    <div className="app">
      <header className="topbar">
        <nav className="tabs">
          {TABS.map((label, i) => (
            <button
              key={i}
              className={i === tab ? "tab active" : "tab"}
              onClick={() => setTab(i)}
            >
              {label}
            </button>
          ))}
        </nav>
        <div className="tag">{TAGS[tab]}</div>
      </header>

      {/* Tabs stay mounted so each keeps its files and settings while hidden. */}
      <MangaTab active={tab === 0} />
      <WallpaperTab active={tab === 1} />
      {tab === 2 && <div className="panel coming-soon">Coming soon...</div>}
    </div>
  );
}
