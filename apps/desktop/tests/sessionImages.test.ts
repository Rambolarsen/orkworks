import test from "node:test";
import assert from "node:assert/strict";
import { validateImageSelection, validateImageDimensions, imageDimensions } from "../src/sessionImages.ts";

// Catches admitting oversized batches before file reads or image decoding.
test("selection rejects a whole batch when it would exceed count or byte limits", () => {
  assert.throws(() => validateImageSelection(7, 0, [{name:"one.png", size:1}, {name:"two.png", size:1}]), /eight/i);
  assert.throws(() => validateImageSelection(0, 0, [{name:"big.png", size:5 * 1024 * 1024 + 1}]), /5 MiB/);
  assert.throws(() => validateImageSelection(4, 20 * 1024 * 1024, [{name:"next.png", size:1}]), /20 MiB/);
  assert.doesNotThrow(() => validateImageSelection(7, 0, [{name:"last.png", size:1}]));
});

test("selection rejects empty files and unbounded names", () => {
  assert.throws(() => validateImageSelection(0, 0, [{name:"empty.png", size:0}]), /empty/i);
  assert.throws(() => validateImageSelection(0, 0, [{name:"é".repeat(128), size:1}]), /name/i);
});

// Catches decoding an image bomb or retaining multiple oversized surfaces.
test("dimensions reject oversized images and aggregate pixels", () => {
  assert.throws(() => validateImageDimensions(4001, 4000, 0), /16 megapixels/);
  assert.throws(() => validateImageDimensions(3000, 3000, 32_000_000), /40 megapixels/);
  assert.throws(() => validateImageDimensions(0, 1, 0), /dimensions/i);
  assert.equal(validateImageDimensions(4000, 4000, 24_000_000), 16_000_000);
});

test("image container dimensions are inspected before pixel allocation", () => {
  const png = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=","base64");
  assert.deepEqual(imageDimensions(png,"image/png"),{width:1,height:1});
  assert.throws(() => imageDimensions(png.subarray(0,23),"image/png"));
  const jpeg = new Uint8Array([255,216,255,192,0,11,8,0,30,0,40,1,1,17,0]);
  assert.deepEqual(imageDimensions(jpeg,"image/jpeg"),{width:40,height:30});
  const webp = new Uint8Array(30); webp.set(new TextEncoder().encode("VP8X"),12); webp[24]=39; webp[27]=29;
  assert.deepEqual(imageDimensions(webp,"image/webp"),{width:40,height:30});
});
