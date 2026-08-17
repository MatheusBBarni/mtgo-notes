import { applyDownloadAvailability } from "../src/lib/downloadCta";

export async function onRequest(context: {
  request: Request;
  next: () => Promise<Response>;
}): Promise<Response> {
  const url = new URL(context.request.url);
  const response = await context.next();
  if (
    url.pathname !== "/download" ||
    url.searchParams.get("available") !== "0"
  ) {
    return response;
  }

  const html = await response.text();
  return new Response(applyDownloadAvailability(html, false), {
    status: response.status,
    headers: response.headers,
  });
}
