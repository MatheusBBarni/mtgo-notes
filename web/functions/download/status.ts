import { ReleaseKeys } from "../../src/lib/releaseKeys";
import {
  parseLatestMeta,
  rejectNonGet,
  type PagesEnv,
  type ReleaseObject,
  type ReleaseStatus,
} from "../../src/lib/releases";

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
    return jsonStatus({ available: false });
  }

  if (zip === null) {
    return jsonStatus({ available: false });
  }

  try {
    const meta = await context.env.RELEASES.get(ReleaseKeys.latestMeta);
    if (meta === null) {
      return jsonStatus({ available: true });
    }

    const parsed = parseLatestMeta(await meta.json());
    if (!parsed) {
      return jsonStatus({ available: true });
    }

    return jsonStatus({
      available: true,
      version: parsed.version,
      filename: parsed.filename,
      uploadedAt: parsed.uploadedAt,
    });
  } catch {
    return jsonStatus({ available: true });
  }
}

function jsonStatus(status: ReleaseStatus): Response {
  return Response.json(status);
}
