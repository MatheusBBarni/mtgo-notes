export type Theme = "light" | "dark";

export const themeStorageKey = "theme";

export function resolveTheme(
  stored: string | null,
  prefersDark: boolean,
): Theme {
  if (stored === "light" || stored === "dark") {
    return stored;
  }
  return prefersDark ? "dark" : "light";
}
