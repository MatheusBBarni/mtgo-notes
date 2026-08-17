export interface FakeR2Object {
  body: ReadableStream<Uint8Array> | null;
  text(): Promise<string>;
  json(): Promise<unknown>;
  arrayBuffer(): Promise<ArrayBuffer>;
}

export function fakeObject(bytes: Uint8Array): FakeR2Object {
  return {
    body: new ReadableStream({
      start(controller) {
        controller.enqueue(bytes);
        controller.close();
      },
    }),
    async text() {
      return new TextDecoder().decode(bytes);
    },
    async json() {
      return JSON.parse(new TextDecoder().decode(bytes));
    },
    async arrayBuffer() {
      return bytes.buffer.slice(
        bytes.byteOffset,
        bytes.byteOffset + bytes.byteLength,
      ) as ArrayBuffer;
    },
  };
}

export function fakeJson(value: unknown): FakeR2Object {
  return fakeObject(new TextEncoder().encode(JSON.stringify(value)));
}

export function fakeR2(
  objects: Record<string, FakeR2Object | null> = {},
): { get(key: string): Promise<FakeR2Object | null> } {
  return {
    async get(key: string) {
      return objects[key] ?? null;
    },
  };
}
