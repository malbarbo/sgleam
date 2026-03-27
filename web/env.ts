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
    };
}
