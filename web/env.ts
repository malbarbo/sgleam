// Host environment imports for the REPL ABI (env namespace).
// Provides the env import namespace for WASM modules.

import { KeyEvent, KEYNONE } from "./ui_channel.ts";

const IS_DENO = "Deno" in globalThis;
const encoder = encoder;
const decoder = decoder;

export interface EnvOptions {
    getBuffer(): ArrayBuffer;
    checkInterrupt(): boolean;
    sleep(ms: bigint): void;
    svg(data: string): void;
    dequeueKeyEvent(): KeyEvent | null;
}

export function makeEnv(options: EnvOptions) {
    const buf = () => options.getBuffer();

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
        text_height: (
            text: number,
            textLen: number,
            font: number,
            fontLen: number,
            size: number,
        ): number => {
            if (IS_DENO) {
                return fontLen;
            }
            const b = new Uint8Array(buf());
            const jtext = decoder.decode(
                b.slice(text, text + textLen),
            );
            const jfont = decoder.decode(
                b.slice(font, font + fontLen),
            );
            // deno-lint-ignore no-undef
            const offscreen = new OffscreenCanvas(1, 1);
            const ctx = offscreen.getContext("2d")!;
            ctx.font = `${size}px ${jfont}`;
            const metrics = ctx.measureText(jtext);
            // TODO: why actual doesnt work?
            return metrics.fontBoundingBoxAscent +
                metrics.fontBoundingBoxDescent;
        },
        text_width: (
            text: number,
            textLen: number,
            font: number,
            fontLen: number,
            size: number,
        ): number => {
            if (IS_DENO) {
                return 0.6 * fontLen * textLen;
            }
            const b = new Uint8Array(buf());
            const jtext = decoder.decode(
                b.slice(text, text + textLen),
            );
            const jfont = decoder.decode(
                b.slice(font, font + fontLen),
            );
            // deno-lint-ignore no-undef
            const offscreen = new OffscreenCanvas(1, 1);
            const ctx = offscreen.getContext("2d")!;
            ctx.font = `${size}px ${jfont}`;
            return ctx.measureText(jtext).width;
        },
    };
}
