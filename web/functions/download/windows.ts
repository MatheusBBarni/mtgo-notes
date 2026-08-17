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

export async function onRequestGet(context: {
  env: ReleaseEnv;
}): Promise<Response> {
  const zip = await context.env.RELEASES.get(latestZipKey);
  if (zip === null) {
    return new Response(null, {
      status: 302,
      headers: { Location: "/download?available=0" },
    });
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
  const meta = await releases.get(latestMetaKey);
  if (meta === null) {
    return fallbackFilename;
  }

  try {
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
