interface R2GetResult {
  json(): Promise<unknown>;
}

interface ReleaseEnv {
  RELEASES: {
    get(key: string): Promise<R2GetResult | null>;
  };
}

const latestZipKey = "releases/windows/latest.zip";
const latestMetaKey = "releases/windows/latest.json";

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
    return Response.json({ available: false });
  }

  if (zip === null) {
    return Response.json({ available: false });
  }

  try {
    const meta = await context.env.RELEASES.get(latestMetaKey);
    if (meta === null) {
      return Response.json({ available: true });
    }

    const parsed = await meta.json();
    if (!isLatestMeta(parsed)) {
      return Response.json({ available: true });
    }

    return Response.json({
      available: true,
      version: parsed.version,
      filename: parsed.filename,
      uploadedAt: parsed.uploadedAt,
    });
  } catch {
    return Response.json({ available: true });
  }
}

function isLatestMeta(
  value: unknown,
): value is { version: string; filename: string; uploadedAt: string } {
  return (
    !!value &&
    typeof value === "object" &&
    "version" in value &&
    "filename" in value &&
    "uploadedAt" in value &&
    typeof value.version === "string" &&
    typeof value.filename === "string" &&
    typeof value.uploadedAt === "string"
  );
}
