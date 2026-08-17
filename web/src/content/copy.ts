export const primaryDownloadHref = "/download/windows";

const forbidden = [
  "tournament-safe",
  "ban-proof",
  "auto-updater",
  "their current deck",
] as const;

const thirdPartyHosts = [
  "googletagmanager.com",
  "google-analytics.com",
  "fonts.googleapis.com",
  "plausible.io",
  "cloudflareinsights.com",
  "doubleclick.net",
] as const;

export function assertAllowedCopy(sample: string): void {
  const haystack = sample.toLowerCase();
  for (const term of forbidden) {
    if (haystack.includes(term)) {
      throw new Error(`Forbidden copy: ${term}`);
    }
  }
}

export function findForbiddenThirdParty(html: string): string[] {
  const haystack = html.toLowerCase();
  return thirdPartyHosts.filter((host) => haystack.includes(host));
}
