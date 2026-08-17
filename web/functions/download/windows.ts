import { ReleaseKeys } from "../../src/lib/releaseKeys";
import {
  parseLatestMeta,
  rejectNonGet,
  type PagesEnv,
  type ReleaseObject,
} from "../../src/lib/releases";

const fallbackFilename = "MTGONotes-win-x64.zip";

function emptyStateRedirect(): Response {
  return new Response(null, {
    status: 302,
    headers: { Location: "/download?available=0" },
  });
}

export async function onRequest(context: {
  request: Request;
  env: PagesEnv;
}): Promise<Response> {
  return rejectNonGet(context.request.method) ?? onRequestGet(context);
}

export async function onRequestGet(context: {
  env: PagesEnv;
}): Promise<Response> {
  let zip: ReleaseObject | null;
  try {
    zip = await context.env.RELEASES.get(ReleaseKeys.latestZip);
  } catch {
    return emptyStateRedirect();
  }

  if (zip === null) {
    return emptyStateRedirect();
  }

  return new Response(zip.body ?? null, {
    status: 200,
    headers: {
      "Content-Type": "application/zip",
      "Content-Disposition": `attachment; filename="${await readFilename(context.env.RELEASES)}"`,
      "Cache-Control": "private, no-store",
    },
  });
}

async function readFilename(releases: PagesEnv["RELEASES"]): Promise<string> {
  try {
    const meta = await releases.get(ReleaseKeys.latestMeta);
    if (meta === null) {
      return fallbackFilename;
    }
    return parseLatestMeta(await meta.json())?.filename ?? fallbackFilename;
  } catch {
    return fallbackFilename;
  }
}
