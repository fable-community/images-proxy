import decode from 'https://deno.land/x/wasm_image_decoder@v0.0.7/mod.js';

import pixelmatch from 'https://esm.sh/pixelmatch@5.3.0';

import { dirname, join } from 'https://deno.land/std@0.188.0/path/mod.ts';

import { existsSync } from 'https://deno.land/std@0.188.0/fs/mod.ts';

import { assertEquals } from 'https://deno.land/std@0.188.0/testing/asserts.ts';

import { handler } from '../build/images_proxy.js';

const directory = dirname(import.meta.url);

const kiara = 'http://./https://images2.imgbox.com/05/06/ilSpCC45_o.png';
const fauna = 'http://./https://images2.imgbox.com/76/07/BLiqJb65_o.png';
const aqua = 'http://./https://images2.imgbox.com/a8/8c/a91bsWKH_o.png';

const guraEmbed = 'http://./https://imgbox.com/mxkxpJOP';

const compare = async (
  snapShotPath: URL,
  response: Response,
): Promise<number> => {
  const existing = decode(await Deno.readFile(snapShotPath));

  const diff = pixelmatch(
    existing.data,
    decode(await response.arrayBuffer()).data,
    null,
    existing.width,
    existing.height,
  );

  return diff;
};

Deno.test('large portrait', async (test) => {
  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.jpeg`),
  );

  const response = await handler(new Request(kiara));

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(
      snapShotPath,
      new Uint8Array(await response.arrayBuffer()),
    );
  } else {
    assertEquals(await compare(snapShotPath, response), 0);
  }
});

Deno.test('large portrait blurred', async (test) => {
  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.jpeg`),
  );

  const response = await handler(new Request(`${kiara}?blur`));

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(
      snapShotPath,
      new Uint8Array(await response.arrayBuffer()),
    );
  } else {
    assertEquals(await compare(snapShotPath, response), 0);
  }
});

Deno.test('large portrait 2', async (test) => {
  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.jpeg`),
  );

  const response = await handler(new Request(fauna));

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(
      snapShotPath,
      new Uint8Array(await response.arrayBuffer()),
    );
  } else {
    assertEquals(await compare(snapShotPath, response), 0);
  }
});

Deno.test('large square', async (test) => {
  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.jpeg`),
  );

  const response = await handler(new Request(aqua));

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(
      snapShotPath,
      new Uint8Array(await response.arrayBuffer()),
    );
  } else {
    assertEquals(await compare(snapShotPath, response), 0);
  }
});

Deno.test('preview portrait', async (test) => {
  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.jpeg`),
  );

  const response = await handler(new Request(`${kiara}?size=preview`));

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(
      snapShotPath,
      new Uint8Array(await response.arrayBuffer()),
    );
  } else {
    assertEquals(await compare(snapShotPath, response), 0);
  }
});

Deno.test('preview square', async (test) => {
  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.jpeg`),
  );

  const response = await handler(new Request(`${aqua}?size=preview`));

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(
      snapShotPath,
      new Uint8Array(await response.arrayBuffer()),
    );
  } else {
    assertEquals(await compare(snapShotPath, response), 0);
  }
});

Deno.test('ogimage', async (test) => {
  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.jpeg`),
  );

  const response = await handler(new Request(guraEmbed));

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(
      snapShotPath,
      new Uint8Array(await response.arrayBuffer()),
    );
  } else {
    assertEquals(await compare(snapShotPath, response), 0);
  }
});
