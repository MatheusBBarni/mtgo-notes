interface ReleaseEnv {
  RELEASES: {
    get(key: string): Promise<unknown>;
  };
}

export async function onRequestGet(context: {
  env: ReleaseEnv;
}): Promise<Response> {
  const obj = await context.env.RELEASES.get("releases/windows/latest.zip");
  if (obj === null) {
    return new Response(null, {
      status: 302,
      headers: { Location: "/download?available=0" },
    });
  }

  return new Response(null, { status: 500 });
}
