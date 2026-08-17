export interface ReleaseStatus {
  available: boolean;
  version?: string;
  filename?: string;
  uploadedAt?: string;
}

export interface LatestMeta {
  version: string;
  filename: string;
  uploadedAt: string;
}

export interface ReleaseObject {
  body?: ReadableStream<Uint8Array> | null;
  json(): Promise<unknown>;
}

export interface PagesEnv {
  RELEASES: {
    get(key: string): Promise<ReleaseObject | null>;
  };
}

export function rejectNonGet(method: string): Response | null {
  if (method === "GET") {
    return null;
  }
  return new Response(null, { status: 405 });
}

export function parseLatestMeta(value: unknown): LatestMeta | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  if (
    !("version" in value) ||
    !("filename" in value) ||
    !("uploadedAt" in value) ||
    typeof value.version !== "string" ||
    typeof value.filename !== "string" ||
    typeof value.uploadedAt !== "string"
  ) {
    return null;
  }
  return {
    version: value.version,
    filename: value.filename,
    uploadedAt: value.uploadedAt,
  };
}
