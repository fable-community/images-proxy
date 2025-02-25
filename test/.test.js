import { test, expect } from "vitest";
import sharp from "sharp";
import pixelmatch from "pixelmatch";
import { join } from "path";
import { promises, existsSync } from "fs";
import { fileURLToPath } from "url";
import { dirname } from "path";
import { proxy } from "../build/images_proxy";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const kiara = "https://images2.imgbox.com/05/06/ilSpCC45_o.png";
const fauna = "https://images2.imgbox.com/76/07/BLiqJb65_o.png";
const aqua = "https://images2.imgbox.com/a8/8c/a91bsWKH_o.png";
const guraEmbed = "https://imgbox.com/mxkxpJOP";

const compare = async (snapShotPath, image) => {
  const existing = sharp(await promises.readFile(snapShotPath));
  const decoded = sharp(image.buffer);

  // console.log(await existing.metadata());
  // console.log(await decoded.metadata());

  return pixelmatch(
    await existing.raw().toBuffer(),
    await decoded.raw().toBuffer(),
    null,
    (await existing.metadata()).width,
    (await existing.metadata()).height
  );
};

test("large portrait", async () => {
  const { format, image } = await proxy(kiara);
  const snapShotPath = join(__dirname, "__snapshots__", "large portrait.png");

  expect(format).toBe("image/png");

  if (!existsSync(snapShotPath)) {
    await promises.writeFile(snapShotPath, image);
  } else {
    expect(await compare(snapShotPath, image)).toBe(0);
  }
}, 20000);

test("large portrait 2", async () => {
  const { format, image } = await proxy(fauna);
  const snapShotPath = join(__dirname, "__snapshots__", "large portrait 2.png");

  expect(format).toBe("image/png");

  if (!existsSync(snapShotPath)) {
    await promises.writeFile(snapShotPath, image);
  } else {
    expect(await compare(snapShotPath, image)).toBe(0);
  }
}, 20000);

test("large square", async () => {
  const { format, image } = await proxy(aqua);
  const snapShotPath = join(__dirname, "__snapshots__", "large square.png");

  expect(format).toBe("image/png");

  if (!existsSync(snapShotPath)) {
    await promises.writeFile(snapShotPath, image);
  } else {
    expect(await compare(snapShotPath, image)).toBe(0);
  }
}, 20000);

test("preview portrait", async () => {
  const { format, image } = await proxy(kiara, "preview");
  const snapShotPath = join(__dirname, "__snapshots__", "preview portrait.png");

  expect(format).toBe("image/png");

  if (!existsSync(snapShotPath)) {
    await promises.writeFile(snapShotPath, image);
  } else {
    expect(await compare(snapShotPath, image)).toBe(0);
  }
}, 20000);

test("preview square", async () => {
  const { format, image } = await proxy(aqua, "preview");
  const snapShotPath = join(__dirname, "__snapshots__", "preview square.png");

  expect(format).toBe("image/png");

  if (!existsSync(snapShotPath)) {
    await promises.writeFile(snapShotPath, image);
  } else {
    expect(await compare(snapShotPath, image)).toBe(0);
  }
}, 20000);

test.skip("ogimage ", async () => {
  const { format, image } = await proxy(guraEmbed, undefined);
  const snapShotPath = join(__dirname, "__snapshots__", "ogimage.jpeg");

  expect(format).toBe("image/jpeg");

  if (!existsSync(snapShotPath)) {
    await promises.writeFile(snapShotPath, image);
  } else {
    expect(await compare(snapShotPath, image)).toBe(0);
  }
}, 20000);
