export const IMAGE_ACCEPT = "image/png,image/jpeg,image/webp";
export interface SessionImage { name: string; mimeType: string; dataBase64: string }
export interface DraftImage extends SessionImage { id: string; previewUrl: string; bytes: number; pixels: number }
const MiB = 1024 * 1024;
export function validateImageSelection(count: number, bytes: number, files: readonly { name: string; size: number }[]): void {
  if (count + files.length > 8) throw new Error("Attach at most eight images.");
  let total = bytes;
  for (const file of files) {
    if (!file.size) throw new Error(`${file.name}: image is empty.`);
    if (new TextEncoder().encode(file.name).length > 255) throw new Error(`${file.name}: image name is too long.`);
    if (file.size > 5 * MiB) throw new Error(`${file.name}: maximum image size is 5 MiB.`);
    total += file.size;
  }
  if (total > 20 * MiB) throw new Error("Images must total at most 20 MiB.");
}
export function validateImageDimensions(width: number, height: number, existingPixels: number): number {
  if (!Number.isSafeInteger(width) || !Number.isSafeInteger(height) || width <= 0 || height <= 0) throw new Error("Invalid image dimensions.");
  const pixels = width * height;
  if (pixels > 16_000_000) throw new Error("Each image must be at most 16 megapixels.");
  if (pixels + existingPixels > 40_000_000) throw new Error("Images must total at most 40 megapixels.");
  return pixels;
}
function imageType(bytes: Uint8Array): string {
  if (bytes.length >= 8 && [137,80,78,71,13,10,26,10].every((v,i) => bytes[i] === v)) return "image/png";
  if (bytes[0] === 255 && bytes[1] === 216 && bytes[2] === 255) return "image/jpeg";
  if (new TextDecoder().decode(bytes.subarray(0,4)) === "RIFF" && new TextDecoder().decode(bytes.subarray(8,12)) === "WEBP") return "image/webp";
  throw new Error("Use a PNG, JPEG, or WebP image.");
}
// Read bounded container headers before asking Chromium to allocate pixels.
export function imageDimensions(bytes: Uint8Array, mimeType: string): {width: number; height: number} {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const fail = () => { throw new Error("Cannot read image dimensions."); };
  if (mimeType === "image/png") {
    if (bytes.length < 24 || new TextDecoder().decode(bytes.subarray(12,16)) !== "IHDR") return fail();
    return {width:view.getUint32(16),height:view.getUint32(20)};
  }
  if (mimeType === "image/webp") {
    if (bytes.length < 30) return fail();
    const chunk = new TextDecoder().decode(bytes.subarray(12,16));
    const little24 = (offset:number) => bytes[offset] + bytes[offset+1] * 256 + bytes[offset+2] * 65536;
    if (chunk === "VP8X") return {width:little24(24)+1,height:little24(27)+1};
    if (chunk === "VP8L" && bytes[20] === 47) return {width:1 + ((bytes[21] | bytes[22] << 8) & 0x3fff),height:1 + ((bytes[22] >> 6 | bytes[23] << 2 | bytes[24] << 10) & 0x3fff)};
    if (chunk === "VP8 " && bytes[23] === 157 && bytes[24] === 1 && bytes[25] === 42) return {width:view.getUint16(26,true) & 0x3fff,height:view.getUint16(28,true) & 0x3fff};
    return fail();
  }
  if (mimeType === "image/jpeg") {
    let offset = 2;
    while (offset + 3 < bytes.length) {
      if (bytes[offset++] !== 255) return fail();
      while (bytes[offset] === 255) offset++;
      const marker = bytes[offset++];
      if (marker === 217 || marker === 218) return fail();
      if (marker === 1 || (marker >= 208 && marker <= 215)) continue;
      if (offset + 2 > bytes.length) return fail();
      const length = view.getUint16(offset);
      if (length < 2 || offset + length > bytes.length) return fail();
      if ([192,193,194,195,197,198,199,201,202,203,205,206,207].includes(marker)) {
        if (length < 8) return fail();
        return {width:view.getUint16(offset+5),height:view.getUint16(offset+3)};
      }
      offset += length;
    }
  }
  return fail();
}
export function releaseImages(images: readonly DraftImage[]): void { for (const image of images) URL.revokeObjectURL(image.previewUrl); }
export async function prepareImageBatch(existing: readonly DraftImage[], files: readonly File[]): Promise<DraftImage[]> {
  validateImageSelection(existing.length, existing.reduce((n,i) => n + i.bytes, 0), files);
  const added: DraftImage[] = [];
  let totalPixels = existing.reduce((n,i) => n + i.pixels, 0);
  try {
    for (const file of files) {
      try {
        const bytes = new Uint8Array(await file.arrayBuffer());
        const mimeType = imageType(bytes);
        const {width, height} = imageDimensions(bytes, mimeType);
        const pixels = validateImageDimensions(width, height, totalPixels);
        const scale = Math.min(1, 512 / Math.max(width, height));
        const bitmap = await createImageBitmap(new Blob([bytes], {type: mimeType}), {resizeWidth: Math.max(1, Math.round(width * scale)), resizeHeight: Math.max(1, Math.round(height * scale))});
        let preview: Blob;
        try {
          const canvas = document.createElement("canvas"); canvas.width = bitmap.width; canvas.height = bitmap.height;
          const context = canvas.getContext("2d");
          if (!context) throw new Error("Image preview is unavailable.");
          context.drawImage(bitmap, 0, 0);
          preview = await new Promise<Blob>((resolve,reject) => canvas.toBlob((blob) => blob ? resolve(blob) : reject(new Error("Cannot preview image.")), "image/png"));
          canvas.width = 0; canvas.height = 0;
        } finally { bitmap.close(); }
        let binary = "";
        for (let offset = 0; offset < bytes.length; offset += 8192) binary += String.fromCharCode(...bytes.subarray(offset, offset + 8192));
        added.push({id: crypto.randomUUID(), name: file.name, mimeType, dataBase64: btoa(binary), previewUrl: URL.createObjectURL(preview), bytes: bytes.length, pixels});
        totalPixels += pixels;
      } catch (error) {
        throw new Error(`${file.name}: ${error instanceof Error ? error.message : "Cannot read image."}`);
      }
    }
    return added;
  } catch (error) { releaseImages(added); throw error; }
}
