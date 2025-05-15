// @ts-check
/// <reference no-default-lib="true"/>
/// <reference lib="esnext" />
/// <reference lib="webworker" /> 

/**
 * @typedef {object} WebAssemblyExports
 * @property {WebAssembly.Memory} memory
 * @property {(len: number) => number} string_allocate
 * @property {(len: number) => void} string_deallocate
 * @property {(str_ptr: number, str_len: number) => number} repl_new
 * @property {(repl_ptr: number, str_ptr: number, str_len: number) => boolean} repl_run
 * @property {(repl_ptr: number) => void} repl_destroy
 * @property {(big_int: boolean) => void} use_bigint
 */


/** @type {Uint8Array} */
let wasmBytes;

/** @type {WebAssembly.Module} */
let wasmModule;

/** @type {WebAssemblyExports} */
let wasmExports;

/** @type {number} */
let repl;

/** @type {Int32Array} */
let stopBuffer;

async function loadAndInstantiateWasm() {
    try {
        const response = await fetch('sgleam.wasm');

        if (!response.ok || !response.body) {
            postError(`Error loading sgleam.wasm: ${response.status}`);
            return;
        }

        const total = parseInt(response.headers.get('Content-Length') ?? '');
        const reader = response.body.getReader();
        if (reader === null) {

        }

        const chunks = [];
        let loaded = 0;
        while (true) {
            const { done, value } = await reader.read();

            if (done) {
                break;
            }

            chunks.push(value);
            loaded += value.length;
            if (total) {
                postProgress((loaded / total) * 100);
            }
        }

        wasmBytes = new Uint8Array(loaded);
        let offset = 0;
        for (const chunk of chunks) {
            wasmBytes.set(chunk, offset);
            offset += chunk.length;
        }

        wasmModule = await WebAssembly.compile(/** @type {ArrayBuffer} */(wasmBytes.buffer));
        await instantiateWasm();
    } catch (error) {
        postError(`${error} `);
    }
}

async function instantiateWasm() {
    var count = 0;
    const __WASI_ESUCCESS = 0;
    const __WASI_EINVAL = 28;
    const __WASI_ENOSYS = 52;
    const __WASI_EBADF = 8;
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();
    const args = ['sgleam'];
    const env = ['RUST_BACKTRACE=1'];
    const instance = await WebAssembly.instantiate(wasmModule, {
        env: {
            import_check_interrupt() {
                return Atomics.exchange(stopBuffer, 0, 0);
            }
        },
        wasi_snapshot_preview1: {
            /**
             * @param {number} clockId 
             * @param {number} _precision 
             * @param {number} timePtr 
             * @returns number
             */
            clock_time_get(clockId, _precision, timePtr) {
                try {
                    const dataView = new DataView(wasmExports.memory.buffer);
                    let timestamp;
                    switch (clockId) {
                        case 0: // __WASI_CLOCK_REALTIME
                            timestamp = BigInt(Date.now()) * 1_000_000n;
                            break;
                        case 1:
                        case 2:
                        case 3:
                            if (typeof performance !== 'undefined' && typeof performance.now === 'function') {
                                timestamp = BigInt(Math.round(performance.now() * 1_000_000));
                            } else {
                                timestamp = BigInt(Date.now()) * 1_000_000n;
                            }
                            break;
                        default:
                            return __WASI_EINVAL;
                    }

                    dataView.setBigUint64(timePtr, timestamp, true);

                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('clock_time_get failed:', error);
                    return __WASI_ENOSYS;
                }
            },
            /**
             * @param {number} environPtr 
             * @param {number} environBufPtr 
             * @returns number
             */
            environ_get(environPtr, environBufPtr) {
                try {
                    const dataView = new DataView(wasmExports.memory.buffer);
                    const envPtrs = [];

                    let currentBufPtr = environBufPtr;
                    for (const envVar of env) {
                        envPtrs.push(currentBufPtr);
                        // FIXME: what about encoding?
                        for (let i = 0; i < envVar.length; i++) {
                            dataView.setUint8(currentBufPtr++, envVar.charCodeAt(i));
                        }
                        dataView.setUint8(currentBufPtr++, 0); // null terminator
                    }

                    for (let i = 0; i < envPtrs.length; i++) {
                        dataView.setInt32(environPtr + i * 4, envPtrs[i], true);
                    }
                    dataView.setInt32(environPtr + envPtrs.length * 4, 0, true); // array terminator.

                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('environ_get failed:', error);
                    return __WASI_ENOSYS;
                }
            },
            /**
             * @param {number} environCountPtr 
             * @param {number} environBufSizePtr 
             * @returns number
             */
            environ_sizes_get(environCountPtr, environBufSizePtr) {
                try {
                    const dataView = new DataView(wasmExports.memory.buffer);

                    let environBufSize = 0;
                    for (const envVar of env) {
                        environBufSize += envVar.length + 1;
                    }

                    dataView.setInt32(environCountPtr, env.length, true);
                    dataView.setInt32(environBufSizePtr, environBufSize, true);

                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('environ_sizes_get failed:', error);
                    return __WASI_ENOSYS;
                }
            },
            proc_exit() {
                console.trace(count++, arguments);
                return 0;
            },
            /**
             * 
             * @param {number} fd 
             * @param {number} iovsPtr 
             * @param {number} iovsLen 
             * @param {number} nwrittenPtr 
             * @returns number
             */
            fd_write(fd, iovsPtr, iovsLen, nwrittenPtr) {
                if (fd !== 1 && fd !== 2) {
                    console.error('fd_write: unsupported file descriptor:', fd);
                    return __WASI_EBADF;
                }

                try {
                    const dataView = new DataView(wasmExports.memory.buffer);

                    let totalBytesWritten = 0;
                    for (let i = 0; i < iovsLen; i++) {
                        const iovPtr = iovsPtr + i * 8; // iovec structure is 8 bytes (ptr + len).
                        const bufPtr = dataView.getInt32(iovPtr, true);
                        const bufLen = dataView.getInt32(iovPtr + 4, true);
                        const buf = new Uint8Array(wasmExports.memory.buffer, bufPtr, bufLen);

                        postOutput(decoder.decode(buf));

                        totalBytesWritten += bufLen;
                    }

                    dataView.setInt32(nwrittenPtr, totalBytesWritten, true);

                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('fd_write failed:', error);
                    return __WASI_ENOSYS;
                }
            },
            fd_seek() {
                console.trace(count++, arguments);
                return 0;
            },
            fd_read() {
                console.trace(count++, arguments);
                return 0;
            },
            fd_close() {
                console.trace(count++, arguments);
                return 0;
            },
            fd_fdstat_get() {
                // called by is_terminal
                return 0;
            },
            /**
             * @param {number} argc_ptr 
             * @param {number} argv_buf_size_ptr 
             * @returns number
             */
            args_sizes_get(argc_ptr, argv_buf_size_ptr) {
                try {
                    let argv_buf_size = 0;
                    for (const arg of args) {
                        argv_buf_size += arg.length + 1;
                    }
                    const dataView = new DataView(wasmExports.memory.buffer);
                    dataView.setInt32(argc_ptr, args.length, true);
                    dataView.setInt32(argv_buf_size_ptr, argv_buf_size, true);
                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('args_sizes_get:', error);
                    return __WASI_ENOSYS;
                }
            },
            /**
             * 
             * @param {number} argv_ptr 
             * @param {number} argv_buf 
             * @returns number
             */
            args_get(argv_ptr, argv_buf) {
                try {
                    let offset = 0;
                    const argPointers = [];
                    const byteView = new Uint8Array(wasmExports.memory.buffer);
                    for (const arg of args) {
                        const encodedArg = encoder.encode(arg);
                        argPointers.push(argv_buf + offset);
                        byteView.set(encodedArg, argv_buf + offset);
                        offset += encodedArg.length;
                        byteView[argv_buf + offset] = 0; // Null terminator
                        offset++;
                    }

                    const dataView = new DataView(wasmExports.memory.buffer);
                    for (let i = 0; i < argPointers.length; i++) {
                        dataView.setInt32(argv_ptr + i * 4, argPointers[i], true);
                    }
                    dataView.setInt32(argv_ptr + args.length * 4, 0, true);

                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('args_sizes_get:', error);
                    return __WASI_ENOSYS;
                }
            },
            /**
             * @param {number} buf 
             * @param {number} len 
             * @returns number
             */
            random_get(buf, len) {
                try {
                    const buffer = new Uint8Array(wasmExports.memory.buffer, buf, len);
                    for (let i = 0; i < buffer.length; i++) {
                        buffer[i] = Math.floor(Math.random() * 256);
                    }
                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('random_get:', error);
                    return __WASI_ENOSYS;
                }
            },
        }
    });
    // @ts-ignore
    wasmExports = instance.exports;
    wasmExports.use_bigint(true);
    self.onmessage = processMsg;
    initRepl('');
}

/**
 * @param {string} data 
 */
function postError(data) {
    self.postMessage({ cmd: 'error', data: data });
}

/**
 * @param {number} data 
 */
function postProgress(data) {
    self.postMessage({ cmd: 'progress', data: data });
}

function postReady() {
    self.postMessage({ cmd: 'ready' });
}

/**
 * @param {string} data 
 */
function postOutput(data) {
    self.postMessage({ cmd: 'output', data: data });
}

/**
 * @param {MessageEvent} event 
 */
function processMsg(event) {
    const data = event.data;
    if (data.cmd == 'init') {
        stopBuffer = new Int32Array(data.data);
    } else if (data.cmd == 'run') {
        runRepl(data.data);
    } else if (data.cmd == 'load') {
        initRepl(data.data);
    } else {
        console.log(`${event} `);
    }
}

/**
 * 
 * @param {string} input 
 */
function initRepl(input) {
    if (repl) {
        wasmExports.repl_destroy(repl);
        repl = 0;
    }
    const [ptr, len] = createString(input);
    repl = wasmExports.repl_new(ptr, len);
    wasmExports.string_deallocate(ptr);
    postReady();
}

/**
 * @param {string} input 
 */
async function runRepl(input) {
    const [ptr, len] = createString(input);
    try {
        if (wasmExports.repl_run(repl, ptr, len)) {
            // :quit
            postOutput('Reloading the repl.');
            initRepl("");
        } else {
            postReady();
        }
    } catch (err) {
        postOutput('Execution error (probably a stackoverflow). Reloading the repl.');
        repl = 0;
        await instantiateWasm();
    } finally {
        wasmExports.string_deallocate(ptr);
    }
}

/**
 * 
 * @param {string} str 
 * @returns number[]
 */
function createString(str) {
    const encoded = new TextEncoder().encode(str);
    const ptr = wasmExports.string_allocate(encoded.length);
    const bytes = new Uint8Array(wasmExports.memory.buffer, ptr, encoded.length);
    bytes.set(encoded)
    return [ptr, encoded.length];
}

loadAndInstantiateWasm();