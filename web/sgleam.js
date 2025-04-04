document.addEventListener('DOMContentLoaded', () => {
    // Initialize CodeFlask editor
    const editorPanel = document.getElementById('editor-panel');
    const flask = new CodeFlask(editorPanel, {
        language: 'js',
        lineNumbers: true
    });

    // Set initial code
    flask.updateCode(`import gleam/io

pub fn main() {
  io.println("Hello, world!")
}`);

    const runButton = document.getElementById('run-button');
    const stopButton = document.getElementById('stop-button');
    const resizeHandle = document.getElementById('resize-handle');
    const editorWrapper = document.getElementById('editor-wrapper');
    const replContainer = document.getElementById('repl-container');

    let replInput;

    replContainer.addEventListener('click', () => {
        if (replInput) {
            replInput.focus()
        }
    });

    // TODO: implement stop
    runButton.addEventListener('click', async () => {
        runButton.disabled = true;
        stopButton.disabled = false;

        replInput = null;
        replContainer.replaceChildren();

        try {
            // TODO: run in worker thread and implement stop
            repl = createRepl(flask.getCode());
        } catch {
            addOutput("Internal error. Reload the page.");
        }

        runButton.disabled = false;
        stopButton.disabled = true;

        createNewInputLine();
    });

    // Panel resizing functionality
    let isResizing = false;
    resizeHandle.addEventListener('mousedown', (e) => {
        isResizing = true;
        document.body.style.cursor = 'col-resize';
        e.preventDefault();
    });

    document.addEventListener('mousemove', (e) => {
        if (!isResizing) return;

        const containerWidth = document.getElementById('container').clientWidth;
        const newWidth = (e.clientX / containerWidth) * 100;

        if (newWidth > 20 && newWidth < 80) {
            editorWrapper.style.width = `${newWidth}%`;
        }
    });

    document.addEventListener('mouseup', () => {
        isResizing = false;
        document.body.style.cursor = '';
    });

    function createNewInputLine() {
        const inputContainer = document.createElement('div');
        inputContainer.className = 'repl-input-container';

        const prompt = document.createElement('div');
        prompt.className = 'repl-prompt';
        prompt.textContent = '>';

        replInput = document.createElement('div');
        replInput.className = 'repl-input';
        replInput.contentEditable = true;
        replInput.spellcheck = false;

        inputContainer.appendChild(prompt);
        inputContainer.appendChild(replInput);
        replContainer.appendChild(inputContainer);

        replInput.focus();

        replInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                const code = replInput.textContent.trim();
                if (code) {
                    replInput.contentEditable = false;
                    runRepl(repl, code);
                    createNewInputLine();
                }
            }
        });
    }

    function addOutput(text, isError = false) {
        const output = document.createElement('div');
        output.className = isError ? 'repl-error repl-line' : 'repl-line';
        output.textContent = text;
        replContainer.appendChild(output);
        scrollToBottom();
    }

    function scrollToBottom() {
        replContainer.scrollTop = replContainer.scrollHeight;
    }

    let repl
    let wasm;

    async function loadWasm() {
        var count = 0;
        const env = ["RUST_BACKTRACE=1"];
        const response = await fetch("sgleam.wasm");
        const bytes = await response.arrayBuffer();
        const { instance } = await WebAssembly.instantiate(bytes, {
            wasi_snapshot_preview1: {
                clock_time_get(clockId, precision, timePtr) {
                    try {
                        const dataView = new DataView(instance.exports.memory.buffer);

                        let timestamp;
                        switch (clockId) {
                            case 0: // __WASI_CLOCK_REALTIME
                                timestamp = BigInt(Date.now()) * 1_000_000n;
                                break;
                            case 1:
                            case 2:
                            case 3: // __WASI_CLOCK_MONOTONIC
                                if (typeof performance !== 'undefined' && typeof performance.now === 'function') {
                                    timestamp = BigInt(Math.round(performance.now() * 1_000_000));
                                } else {
                                    timestamp = BigInt(Date.now()) * 1_000_000n;
                                }
                                break;
                            default:
                                return 28; // __WASI_EINVAL (invalid argument)
                        }

                        // Write the 64-bit timestamp to WASM memory.
                        dataView.setBigUint64(timePtr, timestamp, true); // Little-endian

                        return 0; // __WASI_ESUCCESS
                    } catch (error) {
                        console.error('clock_time_get failed:', error);
                        return 52; // __WASI_EINVAL or other appropriate error.
                    }
                },
                environ_get(environPtr, environBufPtr) {
                    try {
                        const dataView = new DataView(instance.exports.memory.buffer);

                        let currentBufPtr = environBufPtr;
                        const envPtrs = [];

                        for (const envVar of env) {
                            envPtrs.push(currentBufPtr);
                            for (let i = 0; i < envVar.length; i++) {
                                dataView.setUint8(currentBufPtr++, envVar.charCodeAt(i));
                            }
                            dataView.setUint8(currentBufPtr++, 0); // Null terminator
                        }

                        for (let i = 0; i < envPtrs.length; i++) {
                            dataView.setInt32(environPtr + i * 4, envPtrs[i], true);
                        }
                        dataView.setInt32(environPtr + envPtrs.length * 4, 0, true); // Null terminator for the array of pointers.

                        return 0; // __WASI_ESUCCESS
                    } catch (error) {
                        console.error('environ_get failed:', error);
                        return 52; // __WASI_EINVAL
                    }
                },
                environ_sizes_get(environCountPtr, environBufSizePtr) {
                    try {
                        const dataView = new DataView(instance.exports.memory.buffer);
                        const environCount = env.length;
                        let environBufSize = 0;

                        for (const envVar of env) {
                            environBufSize += envVar.length + 1;
                        }

                        dataView.setInt32(environCountPtr, environCount, true);
                        dataView.setInt32(environBufSizePtr, environBufSize, true);

                        return 0; // __WASI_ESUCCESS
                    } catch (error) {
                        console.error('environ_sizes_get failed:', error);
                        return 52; // __WASI_EINVAL
                    }
                },
                proc_exit() {
                    console.trace(count++, arguments);
                    return 0;
                },
                fd_write(fd, iovsPtr, iovsLen, nwrittenPtr) {
                    try {
                        const memory = instance.exports.memory;
                        const dataView = new DataView(memory.buffer);

                        let totalBytesWritten = 0;

                        for (let i = 0; i < iovsLen; i++) {
                            const iovPtr = iovsPtr + i * 8; // iovec structure is 8 bytes (ptr + len).
                            const bufPtr = dataView.getInt32(iovPtr, true);
                            const bufLen = dataView.getInt32(iovPtr + 4, true);

                            const buf = new Uint8Array(memory.buffer, bufPtr, bufLen);
                            const output = new TextDecoder('utf-8').decode(buf);

                            if (fd === 1 || fd === 2) { // stdout or stderr
                                addOutput(output);
                            } else {
                                console.error("fd_write: unsupported file descriptor:", fd);
                                return 8; // __WASI_EBADF (Bad file descriptor)
                            }

                            totalBytesWritten += bufLen;
                        }

                        dataView.setInt32(nwrittenPtr, totalBytesWritten, true); // Write the number of bytes written.
                        return 0; // __WASI_ESUCCESS
                    } catch (error) {
                        console.error('fd_write failed:', error);
                        return 52; // __WASI_EINVAL
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
                fd_prestat_get() {
                    console.trace(count++, arguments);
                    return 0;
                },
                fd_prestat_dir_name() {
                    console.trace(count++, arguments);
                    return 0;
                },
                random_get(buf, len) {
                    const buffer = new Uint8Array(instance.exports.memory.buffer, buf, len);
                    for (let i = 0; i < buffer.length; i++) {
                        buffer[i] = Math.floor(Math.random() * 256);
                    }
                    return 0;
                },
            }
        });
        wasm = instance.exports;
        wasm.use_bigint(true);
        repl = createRepl("");
        runButton.disabled = false;
        createNewInputLine();
    }

    function createString(str) {
        const len = str.length;
        const ptr = wasm.string_allocate(len);
        const bytes = new Uint8Array(wasm.memory.buffer, ptr, len);
        for (let i = 0; i < len; i++) bytes[i] = str.charCodeAt(i);
        return [ptr, len];
    }

    function createRepl(input) {
        const [ptr, len] = createString(input);
        const repl = wasm.repl_new(ptr, len);
        wasm.string_deallocate(ptr);
        return repl
    }

    function runRepl(repl, input) {
        const [ptr, len] = createString(input);
        try {
            const result = wasm.repl_run(repl, ptr, len);
        } catch {
            addOutput("Internal error. Reload the page.");
        }
        wasm.string_deallocate(ptr);
    }

    loadWasm();

});
