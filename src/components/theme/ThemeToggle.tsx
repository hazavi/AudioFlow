import { Moon, Sun, Monitor } from "lucide-react";
import { useEffect, useState } from "react";
import { useUI } from "../../stores/ui";
import { desktop, settings } from "../../services/tauri";
import { Button } from "../ui/button";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "../ui/dropdown-menu";
import type { Theme } from "../../types";
export function ThemeToggle() {
  const { theme, setTheme } = useUI();
  // Adapted from Origin UI's MIT theme dropdown on 21st.dev; persistence and
  // live Windows preference tracking are provided by AudioFlow.
  const [systemDark, setSystemDark] = useState(
    () => matchMedia("(prefers-color-scheme: dark)").matches,
  );
  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");
    const change = () => setSystemDark(media.matches);
    media.addEventListener("change", change);
    return () => media.removeEventListener("change", change);
  }, []);
  const isLight = theme === "light" || (theme === "system" && !systemDark);
  async function select(next: Theme) {
    setTheme(next);
    if (desktop && !(await settings({ theme: next }))) setTheme(theme);
  }
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button size="icon" variant="ghost" aria-label="Choose theme">
          {isLight ? <Sun size={16} /> : <Moon size={16} />}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        {(
          [
            ["dark", Moon],
            ["light", Sun],
            ["system", Monitor],
          ] as const
        ).map(([value, Icon]) => (
          <DropdownMenuItem key={value} onSelect={() => void select(value)}>
            <Icon size={14} /> {value[0].toUpperCase() + value.slice(1)}{" "}
            {theme === value && <span className="accent">✓</span>}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
