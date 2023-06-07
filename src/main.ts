import { serve } from 'https://deno.land/std@0.187.0/http/server.ts';

import { LRU } from 'https://deno.land/x/lru@1.0.2/mod.ts';

import { handler } from '../build/images_proxy.js';

const TEN_MIB = 1024 * 1024 * 10;

const lru = new LRU<{
  body: ArrayBuffer;
  headers: Headers;
}>(20);

serve(async (request) => {
  const url = new URL(request.url);

  const key = (url.pathname + url.search).substring(1);

  if (key === 'favicon.ico') {
    return new Response(null, {
      status: 404,
    });
  }

  const hit = lru.get(key);

  if (hit) {
    console.log(`cache hit: ${key}`);

    return new Response(hit.body, {
      headers: hit.headers,
    });
  }

  const response = await handler(request);

  // set 12 days of cache
  response.headers.set('cache-control', `max-age=${86400 * 12}`);

  if (Number(response.headers.get('content-length')) <= TEN_MIB) {
    const v = {
      body: await response.arrayBuffer(),
      headers: response.headers,
    };

    lru.set(key, v);

    return new Response(v.body, {
      headers: v.headers,
    });
  } else {
    return response;
  }
});
