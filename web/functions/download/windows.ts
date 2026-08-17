interface R2GetResult {
  body: ReadableStream<Uint8Array> | null;
  json(): Promise<unknown>;
}

interface ReleaseEnv {
  RELEASES: {
    get(key: string): Promise<R2GetResult | null>;
  };
}

const latestZipKey = "releases/windows/latest.zip";
const latestMetaKey = "releases/windows/latest.json";
const fallbackFilename = "MTGONotes-win-x64.zip";

function emptyStateRedirect(): Response {
  return new Response(null, {
    status: 302,
    headers: { Location: "/download?available=0" },
  });
}

export async function onRequest(context: {
  request: Request;
  env: ReleaseEnv;
}): Promise<Response> {
  if (context.request.method !== "GET") {
    return new Response(null, { status: 405 });
  }
  return onRequestGet(context);
}

export async function onRequestGet(context: {
  env: ReleaseEnv;
}): Promise<Response> {
  let zip: R2GetResult | null;
  try {
    zip = await context.env.RELEASES.get(latestZipKey);
  } catch {
    return emptyStateRedirect();
  }

  if (zip === null) {
    return emptyStateRedirect();
  }

  const filename = await readFilename(context.env.RELEASES);
  return new Response(zip.body, {
    status: 200,
    headers: {
      "Content-Type": "application/zip",
      "Content-Disposition": `attachment; filename="${filename}"`,
      "Cache-Control": "private, no-store",
    },
  });
}

async function readFilename(
  releases: ReleaseEnv["RELEASES"],
): Promise<string> {
  try {
    const meta = await releases.get(latestMetaKey);
    if (meta === null) {
      return fallbackFilename;
    }

    const parsed = await meta.json();
    if (
      parsed &&
      typeof parsed === "object" &&
      "filename" in parsed &&
      typeof parsed.filename === "string" &&
      parsed.filename.length > 0
    ) {
      return parsed.filename;
    }
  } catch {
    return fallbackFilename;
  }

  return fallbackFilename;
}
