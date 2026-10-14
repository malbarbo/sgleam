// WASI preview1 polyfill.
// Provides the wasi_snapshot_preview1 import namespace for WASM modules.

const STDOUT = 1;
const STDERR = 2;

const WASI_ESUCCESS = 0;
const WASI_EINVAL = 28;
const WASI_ENOSYS = 52;
const WASI_EBADF = 8;

export interface WasiOptions {
  getBuffer(): ArrayBufferLike;
  write(fd: number, text: string): void;
  readStdin?: () => string;
  args?: string[];
  env?: string[];
  /** When false, fd_fdstat_get reports stdout/stderr as regular files so
   * `std::io::IsTerminal` returns false. Defaults to true. */
  isTty?: boolean;
  /** Epoch milliseconds, for a caller that wants the clock to be its own:
   * a test that advances one by hand fires the `world` loop's ticks without
   * burning wall-clock time. Defaults to the real clock, and replaces both
   * the realtime and the monotonic one when given -- a clock stopped for the
   * engine has to be stopped whichever it asks for. */
  now?: () => bigint;
  /** Moves the clock of `now` on, to at most `until` milliseconds, or as far
   * as the next thing that happens when `until` is null. A wait of
   * `poll_oneoff` calls it instead of sleeping, so a test runs on its own
   * time. It comes with `now`, and without both a wait takes the real
   * time. */
  advance?: (until: bigint | null) => void;
  /** The session of sinteract on fd 3 and fd 4. Without it, both are bad
   * descriptors. It needs `now` and `advance`, which a read uses to wait for
   * the room. */
  session?: SessionPipe;
}

/** The host of a session: what the engine writes on fd 4 and reads on fd 3. */
export interface SessionPipe {
  write(bytes: Uint8Array): void;
  /** At most `max` bytes, and none at the end of the stream. Only called
   * when `readable` says so. */
  read(max: number): Uint8Array;
  /** Returns `true` if a read of fd 3 has bytes or the end, `false`
   * otherwise. */
  readable(): boolean;
  /** Called when the engine closes fd 4. */
  end(): void;
}

const SESSION_IN = 3;
const SESSION_OUT = 4;

const EVENTTYPE_CLOCK = 0;
const EVENTTYPE_FD_READ = 1;
const SUBSCRIPTION_BYTES = 48;
const EVENT_BYTES = 32;
const SUBCLOCKFLAGS_ABSTIME = 1;

interface Subscription {
  userdata: bigint;
  tag: number;
  fd: number;
  // In nanoseconds of the clock, for a clock.
  deadline: bigint;
}

// Blocks the thread for `ms` of real time.
function realSleep(ms: number): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

export function makeWasi(options: WasiOptions) {
  const args = options.args ?? [];
  const env = options.env ?? [];
  const isTty = options.isTty ?? true;
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();
  const buf = () => options.getBuffer();
  // The buffers of a WASI iovec array, as [ptr, len] pairs of 8 bytes each.
  const iovecs = (iovsPtr: number, iovsLen: number): [number, number][] => {
    const dataView = new DataView(buf());
    return Array.from({ length: iovsLen }, (_, i) => [
      dataView.getInt32(iovsPtr + i * 8, true),
      dataView.getInt32(iovsPtr + i * 8 + 4, true),
    ]);
  };
  // A wait on a stopped clock, or on a session with nothing to move it on,
  // would never end.
  if (!options.now !== !options.advance) {
    throw new Error("makeWasi: `now` and `advance` go together");
  }
  if (options.session && !options.advance) {
    throw new Error("makeWasi: `session` needs `now` and `advance`");
  }

  // Nanoseconds of the clock `clockId`, or null for a clock WASI does not
  // have.
  const clockNs = (clockId: number): bigint | null => {
    if (clockId > 3) return null;
    if (options.now) return options.now() * 1_000_000n;
    if (clockId === 0) return BigInt(Date.now()) * 1_000_000n;
    return BigInt(Math.round(performance.now() * 1_000_000));
  };

  // Total size of the buffer args_get/environ_get write into: every string
  // null terminated, one after the other.
  const tableSize = (strings: string[]) =>
    strings.reduce((size, s) => size + encoder.encode(s).length + 1, 0);

  // The layout both args_get and environ_get produce: the strings, null
  // terminated, in `bufPtr`, and a pointer to each one, followed by a null
  // pointer, in `ptrPtr`.
  const writeTable = (strings: string[], ptrPtr: number, bufPtr: number) => {
    const byteView = new Uint8Array(buf());
    const dataView = new DataView(buf());
    let offset = bufPtr;
    strings.forEach((s, i) => {
      const encoded = encoder.encode(s);
      dataView.setInt32(ptrPtr + i * 4, offset, true);
      byteView.set(encoded, offset);
      offset += encoded.length;
      byteView[offset++] = 0;
    });
    dataView.setInt32(ptrPtr + strings.length * 4, 0, true);
  };

  return {
    clock_time_get: (
      clockId: number,
      _precision: bigint,
      timePtr: number,
    ): number => {
      try {
        const timestamp = clockNs(clockId);
        if (timestamp === null) return WASI_EINVAL;
        new DataView(buf()).setBigUint64(timePtr, timestamp, true);
        return WASI_ESUCCESS;
      } catch {
        console.error("clock_time_get failed");
        return WASI_ENOSYS;
      }
    },
    environ_get: (
      environPtr: number,
      environBufPtr: number,
    ): number => {
      try {
        writeTable(env, environPtr, environBufPtr);
        return WASI_ESUCCESS;
      } catch {
        console.error("environ_get failed");
        return WASI_ENOSYS;
      }
    },
    environ_sizes_get: (
      environCountPtr: number,
      environBufSizePtr: number,
    ): number => {
      try {
        const dataView = new DataView(buf());
        dataView.setInt32(environCountPtr, env.length, true);
        dataView.setInt32(environBufSizePtr, tableSize(env), true);
        return WASI_ESUCCESS;
      } catch {
        console.error("environ_sizes_get failed");
        return WASI_ENOSYS;
      }
    },
    proc_exit: (): number => 0,
    fd_write: (
      fd: number,
      iovsPtr: number,
      iovsLen: number,
      nwrittenPtr: number,
    ): number => {
      if (fd === SESSION_OUT && options.session) {
        let total = 0;
        for (const [ptr, len] of iovecs(iovsPtr, iovsLen)) {
          options.session.write(new Uint8Array(buf(), ptr, len).slice());
          total += len;
        }
        new DataView(buf()).setInt32(nwrittenPtr, total, true);
        return WASI_ESUCCESS;
      }
      // A world writes to the session to open it, and the error tells it that
      // there is none.
      if (fd === SESSION_OUT) return WASI_EBADF;
      if (fd !== STDOUT && fd !== STDERR) {
        console.error("fd_write: unsupported file descriptor:", fd);
        return WASI_EBADF;
      }
      try {
        let total = 0;
        for (const [ptr, len] of iovecs(iovsPtr, iovsLen)) {
          options.write(fd, decoder.decode(new Uint8Array(buf(), ptr, len)));
          total += len;
        }
        new DataView(buf()).setInt32(nwrittenPtr, total, true);
        return WASI_ESUCCESS;
      } catch {
        console.error("fd_write failed");
        return WASI_ENOSYS;
      }
    },
    fd_seek: (): number => 0,
    fd_read: (
      fd: number,
      iovsPtr: number,
      iovsLen: number,
      nreadPtr: number,
    ): number => {
      if (fd === SESSION_IN && options.session) {
        const session = options.session;
        // A read with nothing to read blocks until the room writes.
        while (!session.readable()) options.advance!(null);
        let total = 0;
        for (const [ptr, len] of iovecs(iovsPtr, iovsLen)) {
          const bytes = session.read(len);
          new Uint8Array(buf()).set(bytes, ptr);
          total += bytes.length;
          if (bytes.length < len) break;
        }
        new DataView(buf()).setInt32(nreadPtr, total, true);
        return WASI_ESUCCESS;
      }
      if (fd !== 0 || !options.readStdin) return WASI_EBADF;
      try {
        const text = options.readStdin();
        const encoded = encoder.encode(text);
        const memory = new Uint8Array(buf());
        let total = 0;
        for (const [ptr, len] of iovecs(iovsPtr, iovsLen)) {
          if (total === encoded.length) break;
          const toCopy = Math.min(len, encoded.length - total);
          memory.set(encoded.subarray(total, total + toCopy), ptr);
          total += toCopy;
        }
        new DataView(buf()).setInt32(nreadPtr, total, true);
        return WASI_ESUCCESS;
      } catch {
        return WASI_ENOSYS;
      }
    },
    fd_close: (fd: number): number => {
      if (fd === SESSION_OUT) options.session?.end();
      return WASI_ESUCCESS;
    },
    poll_oneoff: (
      inPtr: number,
      outPtr: number,
      nsubscriptions: number,
      neventsPtr: number,
    ): number => {
      const view = () => new DataView(buf());
      const subs: Subscription[] = [];
      for (let i = 0; i < nsubscriptions; i++) {
        const at = inPtr + i * SUBSCRIPTION_BYTES;
        const tag = view().getUint8(at + 8);
        const sub = {
          userdata: view().getBigUint64(at, true),
          tag,
          // The clock, for a clock.
          fd: view().getUint32(at + 16, true),
          deadline: 0n,
        };
        if (tag === EVENTTYPE_CLOCK) {
          const now = clockNs(sub.fd);
          if (now === null) return WASI_EINVAL;
          const timeout = view().getBigUint64(at + 24, true);
          const abstime = view().getUint16(at + 40, true) &
            SUBCLOCKFLAGS_ABSTIME;
          sub.deadline = abstime ? timeout : now + timeout;
        }
        subs.push(sub);
      }
      const ready = (sub: Subscription) => {
        if (sub.tag === EVENTTYPE_FD_READ && sub.fd === SESSION_IN) {
          return options.session?.readable() ?? true;
        }
        return true;
      };
      // Nanoseconds until the first clock fires, or null with no clock.
      const clocks = subs.filter((sub) => sub.tag === EVENTTYPE_CLOCK);
      const wait = () =>
        clocks.length
          ? clocks
            .map((sub) => sub.deadline - clockNs(sub.fd)!)
            .reduce((a, b) => (a < b ? a : b))
          : null;
      let fired: Subscription[];
      for (;;) {
        fired = subs.filter((sub) => sub.tag !== EVENTTYPE_CLOCK && ready(sub));
        if (fired.length > 0) break;
        const ns = wait();
        if (ns !== null && ns <= 0n) {
          fired = clocks.filter((sub) => sub.deadline <= clockNs(sub.fd)!);
          break;
        }
        if (options.advance) {
          // With `now`, every clock reads the same time.
          options.advance(
            ns === null ? null : (clockNs(0)! + ns + 999_999n) / 1_000_000n,
          );
        } else if (ns !== null) {
          realSleep(Math.ceil(Number(ns) / 1_000_000));
        } else return WASI_EINVAL;
      }
      new Uint8Array(buf()).fill(
        0,
        outPtr,
        outPtr + fired.length * EVENT_BYTES,
      );
      fired.forEach((sub, i) => {
        const at = outPtr + i * EVENT_BYTES;
        view().setBigUint64(at, sub.userdata, true);
        view().setUint8(at + 10, sub.tag);
      });
      view().setUint32(neventsPtr, fired.length, true);
      return WASI_ESUCCESS;
    },
    fd_fdstat_get: (fd: number, statPtr: number): number => {
      if (fd === STDOUT || fd === STDERR) {
        // Zero the entire fdstat struct (24 bytes) then set fs_filetype.
        // isatty() checks fs_filetype == 2 AND (fs_rights_base[0] & 0x24) == 0,
        // so uninitialized stack memory in fs_rights_base would make it return false.
        const mem = new Uint8Array(buf());
        mem.fill(0, statPtr, statPtr + 24);
        // 2 = CHARACTER_DEVICE (TTY), 4 = REGULAR_FILE (not a TTY)
        mem[statPtr] = isTty ? 2 : 4;
      } else if (fd === SESSION_IN || fd === SESSION_OUT) {
        if (!options.session) return WASI_EBADF;
        new Uint8Array(buf()).fill(0, statPtr, statPtr + 24);
      }
      return WASI_ESUCCESS;
    },
    // Rust 1.95 asks for this import while initializing its WASI file
    // descriptors. The test pipes are synchronous and need no flag change.
    fd_fdstat_set_flags: (fd: number, _flags: number): number => {
      if (
        fd === 0 || fd === STDOUT || fd === STDERR ||
        ((fd === SESSION_IN || fd === SESSION_OUT) && options.session)
      ) return WASI_ESUCCESS;
      return WASI_EBADF;
    },
    args_sizes_get: (
      argcPtr: number,
      argvBufSizePtr: number,
    ): number => {
      try {
        const dataView = new DataView(buf());
        dataView.setInt32(argcPtr, args.length, true);
        dataView.setInt32(argvBufSizePtr, tableSize(args), true);
        return WASI_ESUCCESS;
      } catch {
        console.error("args_sizes_get failed");
        return WASI_ENOSYS;
      }
    },
    args_get: (argvPtr: number, argvBuf: number): number => {
      try {
        writeTable(args, argvPtr, argvBuf);
        return WASI_ESUCCESS;
      } catch {
        console.error("args_get failed");
        return WASI_ENOSYS;
      }
    },
    random_get: (ptr: number, len: number): number => {
      try {
        const buffer = new Uint8Array(buf(), ptr, len);
        for (let i = 0; i < buffer.length; i++) {
          buffer[i] = Math.floor(Math.random() * 256);
        }
        return WASI_ESUCCESS;
      } catch {
        console.error("random_get failed");
        return WASI_ENOSYS;
      }
    },
    path_open: (): number => {
      console.error("path_open");
      return WASI_ENOSYS;
    },
    path_create_directory: (): number => WASI_ENOSYS,
    path_filestat_get: (): number => WASI_ENOSYS,
    path_readlink: (): number => WASI_ENOSYS,
    fd_filestat_get: (): number => {
      console.error("fd_filestat_get");
      return WASI_ENOSYS;
    },
    fd_prestat_get: (): number => {
      console.error("fd_prestat_get");
      return WASI_ENOSYS;
    },
    fd_prestat_dir_name: (): number => {
      console.error("fd_prestat_dir_name");
      return WASI_ENOSYS;
    },
  };
}
