export interface FakeR2Object {
  body: ReadableStream<Uint8Array> | null;
  text(): Promise<string>;
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
