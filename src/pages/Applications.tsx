import { useState } from "react";
import { Search } from "lucide-react";
import { useSystem } from "../services/tauri";
import { AppList, BrowserNotice, Section } from "../components/shared";
import type { Application } from "../types";
export function Applications({
  onSelect,
}: {
  onSelect: (a: Application) => void;
}) {
  const s = useSystem((state) => state.snapshot)!;
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("all");
  const apps = s.applications.filter(
    (a) =>
      `${a.name} ${a.executable}`.toLowerCase().includes(query.toLowerCase()) &&
      (filter === "all" || filter === "playing"
        ? filter === "all" || a.active
        : filter === "manual"
          ? a.mode === "fixed" || a.mode === "default"
          : a.mode === filter),
  );
  return (
    <>
      <div className="toolbar">
        <div className="search">
          <Search size={15} />
          <input
            aria-label="Search applications"
            placeholder="Search applications or executable…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <select
          aria-label="Filter applications"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        >
          <option value="all">All applications</option>
          <option value="playing">Active audio streams</option>
          <option value="auto">Automatic</option>
          <option value="manual">Manual</option>
          <option value="ignore">Ignored</option>
        </select>
      </div>
      <Section title="Applications" meta={`${apps.length} processes`}>
        <AppList applications={apps} onSelect={onSelect} />
      </Section>
      <BrowserNotice />
    </>
  );
}
