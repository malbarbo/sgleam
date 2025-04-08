document.addEventListener('DOMContentLoaded', () => {
    // TODO: add gleam lang
    const flask = new CodeFlask(document.getElementById('editor-panel'), {
        language: 'js',
        lineNumbers: true
    });

    flask.updateCode(`import gleam/io

pub fn main() {
  io.println("Hello, world!")
}`);

    let replInput;
    const runButton = document.getElementById('run-button');
    const stopButton = document.getElementById('stop-button');
    const resizeHandle = document.getElementById('resize-handle');
    const editorWrapper = document.getElementById('editor-wrapper');
    const replContainer = document.getElementById('repl-container');
    const repl = new Worker('repl.js');

    let sharedBuffer = new SharedArrayBuffer(4);
    let buffer = new Int32Array(sharedBuffer);
    Atomics.store(buffer, 0, 0);
    repl.onmessage = (event) => {
        const data = event.data;
        if (data.cmd == 'ready') {
            addInputLine();
            runButton.disabled = false;
            stopButton.disabled = true;
            repl.postMessage({ cmd: 'init', data: sharedBuffer });
        } else if (data.cmd == 'output') {
            addOutput(data.data);
        }
    }

    function postLoad() {
        runButton.disabled = true;
        stopButton.disabled = false;
        repl.postMessage({ cmd: 'load', data: flask.getCode() });
    }

    function postRun(data) {
        runButton.disabled = true;
        stopButton.disabled = false;
        repl.postMessage({ cmd: 'run', data: data });
    }


    // Buttons

    runButton.addEventListener('click', () => {
        replInput = null;
        replContainer.replaceChildren();
        postLoad()
    });

    stopButton.addEventListener('click', () => {
        stopButton.disabled = true;
        let buffer = new Int32Array(sharedBuffer);
        Atomics.store(buffer, 0, 1);
    });


    // Input / output

    function addInputLine() {
        const inputContainer = document.createElement('div');
        inputContainer.className = 'repl-input-container';

        const prompt = document.createElement('div');
        prompt.className = 'repl-prompt';
        prompt.textContent = '>';

        // TODO: add syntax highlight
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
                    postRun(code)
                }
            }
        });
    }

    function addOutput(text) {
        const output = document.createElement('div');
        output.className = 'repl-line';
        output.textContent = text;
        replContainer.appendChild(output);
        replContainer.scrollTop = replContainer.scrollHeight;
    }


    // Focus

    replContainer.addEventListener('click', () => {
        if (window.getSelection().toString().length !== 0) {
            return;
        }
        if (replInput) {
            replInput.focus();
        }
    });


    // Panel resizing

    let isResizing = false;
    resizeHandle.addEventListener('mousedown', (e) => {
        isResizing = true;
        document.body.style.cursor = 'col-resize';
        e.preventDefault();
    });

    document.addEventListener('mousemove', (e) => {
        if (!isResizing) {
            return;
        }

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
});
