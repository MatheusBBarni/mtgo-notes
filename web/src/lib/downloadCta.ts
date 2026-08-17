const downloadLink =
  /<a class="button-primary download-cta"[^>]*>Download for Windows<\/a>/;

export function applyDownloadAvailability(
  html: string,
  available: boolean,
): string {
  if (available) {
    return html;
  }

  return html
    .replace(downloadLink, "")
    .replace('class="download-empty" hidden', 'class="download-empty"');
}
