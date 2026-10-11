// The page's side of a session of sinteract, on a clock of its own: the
// room of sinteract plays the server, and a frame of the display comes
// every FRAME_MS of that clock. A wait of the engine moves the clock on
// instead of sleeping, so a world runs its ticks at once.

import { encodeInput, readServerMessage } from "sinteract/protocol.ts";
import { Room } from "sinteract/room.ts";
import type { SessionPipe } from "./wasi.ts";

const FRAME_MS = 16n;
// A world that runs this long on the clock of the room never ends, and the
// test fails instead of running forever.
const LIMIT_MS = 600_000n;

export type KeyType = "press" | "down" | "up";

/** What the player does, one step every `every` frames of the display. */
export type Step = { key: string; type: KeyType } | "close";

export class FakeRoom {
  /** Milliseconds, for the `now` of the WASI shim. */
  ms = 0n;
  /** The scenes of the frames that the room showed, in order. */
  readonly frames: Uint8Array[] = [];
  readonly #steps: Step[];
  readonly #every: number;
  #room: Room | null = null;
  #toEngine: Uint8Array[] = [];
  #hungUp = false;
  #nextFrame = FRAME_MS;
  #callback: ((now: number) => void) | null = null;
  #count = 0;

  constructor(steps: Step[] = [], every = 1) {
    this.#steps = [...steps];
    this.#every = every;
  }

  readonly pipe: SessionPipe = {
    write: (bytes) => {
      // The first bytes of a session after the end of another.
      if (!this.#room) this.#open();
      this.#room?.receive(bytes);
    },
    read: (max) => {
      const first = this.#toEngine[0];
      if (!first) return new Uint8Array();
      if (first.length <= max) {
        this.#toEngine.shift();
        return first;
      }
      this.#toEngine[0] = first.subarray(max);
      return first.subarray(0, max);
    },
    readable: () => this.#hungUp || this.#toEngine.length > 0,
    close: (fd) => {
      if (fd !== 4) return;
      this.#room?.end();
      this.#room = null;
      this.#callback = null;
    },
  };

  /** Moves the clock on, to `until` or the next frame of the display,
   * whichever comes first. */
  advance(until: bigint | null): void {
    if (until !== null && until < this.#nextFrame) {
      this.ms = until > this.ms ? until : this.ms;
      return;
    }
    if (this.#nextFrame > LIMIT_MS) {
      throw new Error("the world did not end in ten minutes of its clock");
    }
    this.ms = this.#nextFrame;
    this.#nextFrame += FRAME_MS;
    this.#frame();
  }

  #open(): void {
    this.#toEngine = [];
    this.#hungUp = false;
    const room = new Room({
      write: (bytes) => this.#toEngine.push(bytes.slice()),
      onError: (e) => {
        throw e;
      },
      requestFrame: (callback) => {
        this.#callback = callback;
        return 1;
      },
      cancelFrame: () => {
        this.#callback = null;
      },
    });
    room.link.onmessage = (payload) => {
      const message = readServerMessage(payload);
      if (message?.kind === "frame") this.frames.push(message.scene);
    };
    this.#room = room;
  }

  #frame(): void {
    const callback = this.#callback;
    this.#callback = null;
    callback?.(Number(this.ms));
    if (!this.#room || ++this.#count % this.#every !== 0) return;
    const step = this.#steps.shift();
    if (step === undefined) return;
    if (step === "close") {
      // The server ends the session, and the engine reads the end of fd 3.
      this.#hungUp = true;
      return;
    }
    this.#room.link.send(encodeInput({
      kind: "key",
      type: step.type,
      key: step.key,
      modifiers: { alt: false, ctrl: false, shift: false, meta: false },
    }));
  }
}
