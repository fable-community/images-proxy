import decode from 'https://deno.land/x/wasm_image_decoder@v0.0.7/mod.js';

import pixelmatch from 'https://esm.sh/pixelmatch@5.3.0';

import { dirname, join } from 'https://deno.land/std@0.188.0/path/mod.ts';

import { existsSync } from 'https://deno.land/std@0.188.0/fs/mod.ts';

import { assertEquals } from 'https://deno.land/std@0.188.0/testing/asserts.ts';

import { proxy } from '../mod.ts';

const directory = dirname(import.meta.url);

const kiara = 'https://images2.imgbox.com/05/06/ilSpCC45_o.png';
const fauna = 'https://images2.imgbox.com/76/07/BLiqJb65_o.png';
const aqua = 'https://images2.imgbox.com/a8/8c/a91bsWKH_o.png';

const guraEmbed = 'https://imgbox.com/mxkxpJOP';

const compare = async (
  snapShotPath: URL,
  image: Uint8Array,
): Promise<number> => {
  const existing = decode(await Deno.readFile(snapShotPath));

  const diff = pixelmatch(
    existing.data,
    decode(image.buffer).data,
    null,
    existing.width,
    existing.height,
  );

  return diff;
};

Deno.test('large portrait', async (test) => {
  const { format, image } = await proxy(kiara);

  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.webp`),
  );

  assertEquals(format, 'image/webp');

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(snapShotPath, image);
  } else {
    assertEquals(await compare(snapShotPath, image), 0);
  }
});

Deno.test('large portrait 2', async (test) => {
  const { format, image } = await proxy(fauna);

  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.webp`),
  );

  assertEquals(format, 'image/webp');

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(snapShotPath, image);
  } else {
    assertEquals(await compare(snapShotPath, image), 0);
  }
});

Deno.test('large square', async (test) => {
  const { format, image } = await proxy(aqua);

  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.webp`),
  );

  assertEquals(format, 'image/webp');

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(snapShotPath, image);
  } else {
    assertEquals(await compare(snapShotPath, image), 0);
  }
});

Deno.test('preview portrait', async (test) => {
  const { format, image } = await proxy(kiara, 'preview');

  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.webp`),
  );

  assertEquals(format, 'image/webp');

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(snapShotPath, image);
  } else {
    assertEquals(await compare(snapShotPath, image), 0);
  }
});

Deno.test('preview square', async (test) => {
  const { format, image } = await proxy(aqua, 'preview');

  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.webp`),
  );

  assertEquals(format, 'image/webp');

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(snapShotPath, image);
  } else {
    assertEquals(await compare(snapShotPath, image), 0);
  }
});

Deno.test('ogimage', async (test) => {
  const { format, image } = await proxy(guraEmbed, undefined);

  const snapShotPath = new URL(
    join(directory, `__snapshots__/${test.name}.webp`),
  );

  assertEquals(format, 'image/webp');

  if (!existsSync(snapShotPath)) {
    await Deno.writeFile(snapShotPath, image);
  } else {
    assertEquals(await compare(snapShotPath, image), 0);
  }
});
