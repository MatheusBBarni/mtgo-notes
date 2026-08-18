export const githubReleasesHref =
  "https://github.com/MatheusBBarni/mtgo-notes/releases";

export function pageHref(
  path: "" | "download" | "how-it-works" | "live-attach" | "privacy",
): string {
  const base = import.meta.env.BASE_URL.replace(/\/$/, "");
  return path ? `${base}/${path}` : base || "/";
}

export function assetHref(path: string): string {
  const base = import.meta.env.BASE_URL.endsWith("/")
    ? import.meta.env.BASE_URL
    : `${import.meta.env.BASE_URL}/`;
  return `${base}${path.replace(/^\//, "")}`;
}
