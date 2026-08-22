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
}

export function makeWasi(options: WasiOptions) {
  const args = options.args ?? [];
  const env = options.env ?? [];
  const isTty = options.isTty ?? true;
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();
  const buf = () => options.getBuffer();

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
        const dataView = new DataView(buf());
        const now = options.now;
        let timestamp: bigint;
        switch (clockId) {
          case 0: // CLOCK_REALTIME
            timestamp = now
              ? now() * 1_000_000n
              : BigInt(Date.now()) * 1_000_000n;
            break;
          case 1:
          case 2:
          case 3:
            timestamp = now
              ? now() * 1_000_000n
              : BigInt(Math.round(performance.now() * 1_000_000));
            break;
          default:
            return WASI_EINVAL;
        }
        dataView.setBigUint64(timePtr, timestamp, true);
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
      if (fd !== STDOUT && fd !== STDERR) {
        console.error("fd_write: unsupported file descriptor:", fd);
        return WASI_EBADF;
      }
      try {
        const dataView = new DataView(buf());
        let totalBytesWritten = 0;
        for (let i = 0; i < iovsLen; i++) {
          const iovPtr = iovsPtr + i * 8; // iovec is 8 bytes (ptr + len)
          const bufPtr = dataView.getInt32(iovPtr, true);
          const bufLen = dataView.getInt32(iovPtr + 4, true);
          const chunk = new Uint8Array(buf(), bufPtr, bufLen);
          options.write(fd, decoder.decode(chunk));
          totalBytesWritten += bufLen;
        }
        dataView.setInt32(nwrittenPtr, totalBytesWritten, true);
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
      if (fd !== 0 || !options.readStdin) return WASI_EBADF;
      try {
        const text = options.readStdin();
        const encoded = encoder.encode(text);
        const dataView = new DataView(buf());
        const memory = new Uint8Array(buf());
        let totalRead = 0;
        let srcOffset = 0;
        for (
          let i = 0;
          i < iovsLen && srcOffset < encoded.length;
          i++
        ) {
          const iovPtr = iovsPtr + i * 8;
          const bufPtr = dataView.getInt32(iovPtr, true);
          const bufLen = dataView.getInt32(iovPtr + 4, true);
          const toCopy = Math.min(
            bufLen,
            encoded.length - srcOffset,
          );
          memory.set(
            encoded.subarray(srcOffset, srcOffset + toCopy),
            bufPtr,
          );
          srcOffset += toCopy;
          totalRead += toCopy;
        }
        dataView.setInt32(nreadPtr, totalRead, true);
        return WASI_ESUCCESS;
      } catch {
        return WASI_ENOSYS;
      }
    },
    fd_close: (): number => 0,
    fd_fdstat_get: (fd: number, statPtr: number): number => {
      if (fd === STDOUT || fd === STDERR) {
        // Zero the entire fdstat struct (24 bytes) then set fs_filetype.
        // isatty() checks fs_filetype == 2 AND (fs_rights_base[0] & 0x24) == 0,
        // so uninitialized stack memory in fs_rights_base would make it return false.
        const mem = new Uint8Array(buf());
        mem.fill(0, statPtr, statPtr + 24);
        // 2 = CHARACTER_DEVICE (TTY), 4 = REGULAR_FILE (not a TTY)
        mem[statPtr] = isTty ? 2 : 4;
      }
      return WASI_ESUCCESS;
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
