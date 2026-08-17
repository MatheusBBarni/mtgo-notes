export const primaryDownloadHref = "/download/windows";

const forbidden = [
  "tournament-safe",
  "ban-proof",
  "auto-updater",
  "their current deck",
] as const;

export function assertAllowedCopy(sample: string): void {
  const haystack = sample.toLowerCase();
  for (const term of forbidden) {
    if (haystack.includes(term)) {
      throw new Error(`Forbidden copy: ${term}`);
    }
  }
}
