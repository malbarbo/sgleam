// Host environment imports for the REPL ABI (env namespace).
// Provides the env import namespace for WASM modules.

import { KeyEvent, KEYNONE } from "./ui_channel.ts";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export interface EnvOptions {
    getBuffer(): ArrayBuffer;
    checkInterrupt(): boolean;
    sleep(ms: bigint): void;
    svg(data: string): void;
    dequeueKeyEvent(): KeyEvent | null;
}

export function computeTextWidth(m: TextMetrics): number {
    return m.width;
}

export function computeTextHeight(m: TextMetrics): number {
    return m.fontBoundingBoxAscent + m.fontBoundingBoxDescent;
}

export function computeTextXOffset(m: TextMetrics): number {
    return -m.width / 2;
}

export function computeTextYOffset(m: TextMetrics): number {
    const h = m.fontBoundingBoxAscent + m.fontBoundingBoxDescent;
    return m.fontBoundingBoxAscent - h / 2;
}

function measureText(
    buffer: ArrayBuffer,
    text: number,
    textLen: number,
    fontCss: number,
    fontCssLen: number,
): TextMetrics {
    const b = new Uint8Array(buffer);
    const jtext = decoder.decode(b.slice(text, text + textLen));
    const jfontCss = decoder.decode(b.slice(fontCss, fontCss + fontCssLen));
    // deno-lint-ignore no-undef
    const offscreen = new OffscreenCanvas(1, 1);
    const ctx = offscreen.getContext("2d")!;
    ctx.font = jfontCss;
    return ctx.measureText(jtext);
}

export function makeEnv(options: EnvOptions) {
    const buf = () => options.getBuffer();

    function textFn(compute: (m: TextMetrics) => number) {
        return (
            text: number,
            textLen: number,
            fontCss: number,
            fontCssLen: number,
        ) => compute(measureText(buf(), text, textLen, fontCss, fontCssLen));
    }

    return {
        check_interrupt: (): number => options.checkInterrupt() ? 1 : 0,
        sleep: (ms: bigint): void => options.sleep(ms),
        draw_svg: (ptr: number, len: number): void => {
            const b = new Uint8Array(buf());
            options.svg(decoder.decode(b.slice(ptr, ptr + len)));
        },
        get_key_event: (
            ptr: number,
            len: number,
            mods: number,
        ): number => {
            const event = options.dequeueKeyEvent();
            if (event === null) {
                return KEYNONE;
            }
            const b = new Uint8Array(buf());
            const encoded = encoder.encode(event.key);
            b.set(encoded.subarray(0, len), ptr);
            b.fill(0, ptr + encoded.length, ptr + len);
            b[mods + 0] = event.alt ? 1 : 0;
            b[mods + 1] = event.ctrl ? 1 : 0;
            b[mods + 2] = event.shift ? 1 : 0;
            b[mods + 3] = event.meta ? 1 : 0;
            b[mods + 4] = event.repeat ? 1 : 0;
            return event.type;
        },
        text_width: textFn(computeTextWidth),
        text_height: textFn(computeTextHeight),
        text_x_offset: textFn(computeTextXOffset),
        text_y_offset: textFn(computeTextYOffset),
        ...makeBitmapImports(buf),
    };
}

function detectImageDimensions(
    data: Uint8Array,
): { width: number; height: number } {
    // PNG
    if (
        data.length >= 24 && data[0] === 0x89 && data[1] === 0x50 &&
        data[2] === 0x4E && data[3] === 0x47
    ) {
        const view = new DataView(data.buffer, data.byteOffset);
        return { width: view.getUint32(16), height: view.getUint32(20) };
    }
    // JPEG
    if (data.length >= 2 && data[0] === 0xFF && data[1] === 0xD8) {
        let i = 2;
        while (i + 9 < data.length) {
            if (data[i] !== 0xFF) {
                i++;
                continue;
            }
            const marker = data[i + 1];
            if (marker === 0xC0 || marker === 0xC2) {
                const view = new DataView(data.buffer, data.byteOffset);
                return {
                    width: view.getUint16(i + 7),
                    height: view.getUint16(i + 5),
                };
            }
            const len = new DataView(data.buffer, data.byteOffset).getUint16(
                i + 2,
            );
            i += 2 + len;
        }
    }
    // GIF
    if (
        data.length >= 10 && data[0] === 0x47 && data[1] === 0x49 &&
        data[2] === 0x46
    ) {
        const view = new DataView(data.buffer, data.byteOffset);
        return {
            width: view.getUint16(6, true),
            height: view.getUint16(8, true),
        };
    }
    // BMP
    if (data.length >= 26 && data[0] === 0x42 && data[1] === 0x4D) {
        const view = new DataView(data.buffer, data.byteOffset);
        return {
            width: Math.abs(view.getInt32(18, true)),
            height: Math.abs(view.getInt32(22, true)),
        };
    }
    return { width: 0, height: 0 };
}

function guessMime(path: string): string {
    if (path.endsWith(".png")) return "image/png";
    if (path.endsWith(".jpg") || path.endsWith(".jpeg")) return "image/jpeg";
    if (path.endsWith(".gif")) return "image/gif";
    if (path.endsWith(".bmp")) return "image/bmp";
    if (path.endsWith(".webp")) return "image/webp";
    if (path.endsWith(".svg")) return "image/svg+xml";
    return "application/octet-stream";
}

function makeBitmapImports(buf: () => ArrayBuffer) {
    let cached: { width: number; height: number; dataUri: string } | null =
        null;
    const cachedBytes: number[] = [];

    return {
        load_bitmap_fetch: (pathPtr: number, pathLen: number): number => {
            const b = new Uint8Array(buf());
            const path = decoder.decode(b.slice(pathPtr, pathPtr + pathLen));
            try {
                const xhr = new XMLHttpRequest();
                xhr.open("GET", path, false);
                xhr.responseType = "arraybuffer";
                xhr.send();
                if (xhr.status !== 200) {
                    console.log(
                        `Error loading bitmap ${path}: HTTP ${xhr.status}`,
                    );
                    cached = null;
                    return 0;
                }
                const data = new Uint8Array(xhr.response as ArrayBuffer);
                const { width, height } = detectImageDimensions(data);
                if (width === 0 || height === 0) {
                    console.log(
                        `Error: could not detect image dimensions for ${path}`,
                    );
                    cached = null;
                    return 0;
                }
                // Base64 encode
                let binary = "";
                for (let i = 0; i < data.length; i++) {
                    binary += String.fromCharCode(data[i]);
                }
                const b64 = btoa(binary);
                const mime = guessMime(path);
                const dataUri = `data:${mime};base64,${b64}`;
                cached = { width, height, dataUri };
                // Pre-encode to UTF-8 for the data copy
                const encoded = encoder.encode(dataUri);
                cachedBytes.length = 0;
                cachedBytes.push(...encoded);
                return encoded.length;
            } catch (e) {
                console.log(`Error loading bitmap ${path}: ${e}`);
                cached = null;
                return 0;
            }
        },
        load_bitmap_width: (): number => cached?.width ?? 0,
        load_bitmap_height: (): number => cached?.height ?? 0,
        load_bitmap_data: (bufPtr: number, bufLen: number): number => {
            if (!cached) return 0;
            const b = new Uint8Array(buf());
            const len = Math.min(bufLen, cachedBytes.length);
            for (let i = 0; i < len; i++) {
                b[bufPtr + i] = cachedBytes[i];
            }
            return len;
        },
    };
}
