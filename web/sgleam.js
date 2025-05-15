// @ts-check
/// <reference lib="dom" />

/**
 * @enum {number}
 */
const Layout = {
    HORIZONTAL: 1,
    VERTICAL: 2,
}

/**
 * @enum {number}
 */
const Panels = {
    BOTH: 1,
    ONLY_DEFS: 2,
    ONLY_REPL: 3,
}

/**
 * @enum {number}
 */
const Exec = {
    LOADING: 1,
    READY: 2,
    RUNNING: 3,
    PENDING_STOP: 4,
}

/**
 * @typedef {Object} State
 * @property {Layout} layout
 * @property {Panels} panels
 * @property {Exec} exec
 * @property {number} size
 * @property {boolean} resizing
 * @property {string} msg
 * @property {boolean} showingHelp
 */

/** @type {State} */
let state = {
    layout: Layout.HORIZONTAL,
    panels: Panels.BOTH,
    exec: Exec.LOADING,
    size: 50,
    resizing: false,
    msg: 'Loading...',
    showingHelp: false,
};

document.addEventListener('DOMContentLoaded', () => {
    // TODO: add gleam lang
    // @ts-ignore
    const flask = new CodeFlask(document.getElementById('editor-panel'), {
        language: 'js',
        lineNumbers: true
    });

    flask.updateCode(`import gleam/io

pub fn main() {
  io.println("Hello, world!")
}`);

    /**
     * @param {string} id 
     * @returns HTMLElement
     */
    function getElementById(id) {
        return /** @type {HTMLInputElement} */ (document.getElementById(id));
    }

    let replInput;
    const main = getElementById('main');
    const loading = getElementById('loading');
    const runButton = getElementById('run-button');
    const stopButton = getElementById('stop-button');
    const resizeHandle = getElementById('resize-handle');
    const editorPanel = getElementById('editor-panel');
    const replPanel = getElementById('repl-panel');
    const helpOverlay = getElementById('help-overlay');
    const help = getElementById('help');
    const horizontalButton = getElementById('layout-horizontal');
    const verticalButton = getElementById('layout-vertical');
    const repl = new Worker('repl.js');

    let sharedBuffer = new SharedArrayBuffer(4);
    let stopBuffer = new Int32Array(sharedBuffer);
    Atomics.store(stopBuffer, 0, 0);

    // Events
    repl.onmessage = (event) => {
        const data = event.data;
        if (data.cmd == 'error') {
            state.msg = data.data;
        } else if (data.cmd == 'progress') {
            state.msg = `Loading ${Math.round(data.data)}%`;
        } else if (data.cmd == 'ready') {
            if (state.exec === Exec.LOADING) {
                replPanel.replaceChildren();
            }
            state.exec = Exec.READY;
            addInputLine();
            repl.postMessage({ cmd: 'init', data: sharedBuffer });
        } else if (data.cmd == 'output') {
            addOutput(data.data);
        }
        updateDom()
    }

    runButton.addEventListener('click', run);

    stopButton.addEventListener('click', stop);

    verticalButton.addEventListener('click', () => {
        state.layout = Layout.VERTICAL;
        updateDom()
    });

    horizontalButton.addEventListener('click', () => {
        state.layout = Layout.HORIZONTAL;
        updateDom();
    });

    replPanel.addEventListener('click', () => {
        const selection = window.getSelection();
        if (!selection || selection.toString().length !== 0) {
            return;
        }
        focusRepl();
    });

    resizeHandle.addEventListener('mousedown', startResize);
    document.addEventListener('mousemove', resize)
    document.addEventListener('mouseup', stopResize);

    resizeHandle.addEventListener('touchstart', startResize);
    document.addEventListener('touchmove', resize)
    document.addEventListener('touchend', stopResize);

    document.addEventListener('keydown', (event) => {
        if (event.key === 'Escape') {
            event.preventDefault();
            hideHelp()
        } else if (state.showingHelp) {
            return;
        } else if (event.ctrlKey && event.key === '?') {
            event.preventDefault();
            showHelp();
        } else if (event.ctrlKey && event.key === 'j') {
            event.preventDefault();
            focusEditor();
        } else if (event.ctrlKey && event.key === 'k') {
            event.preventDefault();
            focusRepl();
        } else if (event.ctrlKey && event.key === 'r') {
            event.preventDefault();
            run();
        } else if (event.ctrlKey && event.key === 'd') {
            event.preventDefault();
            toogleEditor();
        } else if (event.ctrlKey && event.key === 'i') {
            event.preventDefault();
            toogleRepl();
        } else if (event.ctrlKey && event.key === 'l') {
            event.preventDefault();
            toogleLayout();
        }
    });

    function updateDom() {
        if (state.exec == Exec.LOADING) {
            loading.textContent = state.msg;
            return;
        }

        // Cursor
        if (state.resizing) {
            if (state.layout == Layout.HORIZONTAL) {
                document.body.style.cursor = 'col-resize';
            } else {
                document.body.style.cursor = 'row-resize';
            }
        } else {
            document.body.style.cursor = 'initial';
        }

        // Buttons
        runButton.disabled = state.exec !== Exec.READY;
        stopButton.disabled = state.exec !== Exec.RUNNING && state.exec !== Exec.PENDING_STOP;
        horizontalButton.disabled = state.layout === Layout.HORIZONTAL;
        verticalButton.disabled = state.layout === Layout.VERTICAL;

        // Help
        if (state.showingHelp) {
            helpOverlay.style.display = 'block';
            help.style.display = 'block';
        } else {
            helpOverlay.style.display = 'none';
            help.style.display = 'none';
        }

        // Layout
        if (isEditorVisible()) {
            editorPanel.style.display = 'flex';
        } else {
            editorPanel.style.display = 'none';
        }

        if (isReplVisible()) {
            replPanel.style.display = 'flex';
        } else {
            replPanel.style.display = 'none';
        }

        resizeHandle.style.display = 'initial';
        if (state.layout === Layout.HORIZONTAL) {
            main.style.flexDirection = 'row';

            resizeHandle.style.cursor = 'col-resize';
            resizeHandle.style.width = '8px';
            resizeHandle.style.height = '100%';

            editorPanel.style.height = '100%';
            if (state.panels == Panels.BOTH) {
                editorPanel.style.width = `${state.size}%`;
            } else {
                editorPanel.style.width = `100%`;
                resizeHandle.style.display = 'none';
            }
        } else {
            main.style.flexDirection = 'column';

            resizeHandle.style.cursor = 'row-resize';
            resizeHandle.style.width = '100%';
            resizeHandle.style.height = '8px';

            editorPanel.style.width = '100%';
            if (state.panels == Panels.BOTH) {
                editorPanel.style.height = `${state.size}%`;
            } else {
                editorPanel.style.height = `100%`;
                resizeHandle.style.display = 'none';
            }
        }

        if (state.panels == Panels.ONLY_REPL) {
            focusRepl();
        } else if (state.panels == Panels.ONLY_DEFS) {
            focusEditor();
        }
    }

    function postLoad() {
        state.exec = Exec.RUNNING;
        repl.postMessage({ cmd: 'load', data: flask.getCode() });
        updateDom();
    }

    function postRun(data) {
        state.exec = Exec.RUNNING;
        repl.postMessage({ cmd: 'run', data: data });
        updateDom();
    }

    function focusEditor() {
        const input = editorPanel.querySelector('textarea:not([disabled])');
        if (isEditorVisible() && input) {
            // @ts-ignore
            input.focus();
        }
    }

    function focusRepl() {
        if (isReplVisible() && replInput) {
            replInput.focus();
        }
    }

    let lastActive;

    function showHelp() {
        if (document.activeElement && document.activeElement.blur) {
            lastActive = document.activeElement;
            document.activeElement.blur();
        } else {
            lastActive = null;
        }
        state.showingHelp = true;
        updateDom();
    }

    function hideHelp() {
        if (lastActive) {
            lastActive.focus();
        }
        state.showingHelp = false;
        updateDom();
    }

    function run() {
        if (state.exec === Exec.READY) {
            replInput = null;
            replPanel.replaceChildren();
            postLoad();
            updateDom();
        }
    }

    function stop() {
        console.log('stop');
        if (state.exec === Exec.RUNNING) {
            console.log('setting');
            state.exec = Exec.PENDING_STOP;
            Atomics.store(stopBuffer, 0, 1);
            updateDom();
        }
    }

    function isReplVisible() {
        return state.panels !== Panels.ONLY_DEFS;
    }

    function isEditorVisible() {
        return state.panels !== Panels.ONLY_REPL;
    }

    function toogleEditor() {
        if (state.panels === Panels.ONLY_REPL) {
            state.panels = Panels.BOTH;
        } else {
            state.panels = Panels.ONLY_REPL;
        }
        updateDom();
    }

    function toogleRepl() {
        if (state.panels === Panels.ONLY_DEFS) {
            state.panels = Panels.BOTH;
        } else {
            state.panels = Panels.ONLY_DEFS;
        }
        updateDom();
    }

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
        replInput.contentEditable = "true";
        replInput.spellcheck = false;

        inputContainer.appendChild(prompt);
        inputContainer.appendChild(replInput);
        replPanel.appendChild(inputContainer);

        replInput.focus();

        replInput.addEventListener('paste', function (event) {
            event.preventDefault();

            const selection = window.getSelection();
            if (!selection || !selection.rangeCount) {
                return;
            }

            const texto = event.clipboardData.getData('text/plain');
            const linhas = texto.split('\n');
            const range = selection.getRangeAt(0);
            range.deleteContents();

            let ultimoNo = null;

            linhas.forEach((linha, i) => {
                if (i > 0) {
                    const br = document.createElement('br');
                    range.insertNode(br);
                    ultimoNo = br;
                }

                const textNode = document.createTextNode(linha);
                range.insertNode(textNode);
                ultimoNo = textNode;
            });

            if (ultimoNo) {
                const novoRange = document.createRange();
                novoRange.setStartAfter(ultimoNo);
                novoRange.collapse(true);

                selection.removeAllRanges();
                selection.addRange(novoRange);
            }
        });

        replInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                const text = replInput.cloneNode(true);
                text.querySelectorAll('br').forEach(br => br.replaceWith('\n'))
                const code = text.textContent.trim();
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
        replPanel.appendChild(output);
        replPanel.scrollTop = replPanel.scrollHeight;
    }

    // Panel resizing and layout

    function toogleLayout() {
        if (state.layout === Layout.HORIZONTAL) {
            state.layout = Layout.VERTICAL;
        } else {
            state.layout = Layout.HORIZONTAL;
        }
        updateDom();
    }

    function startResize(e) {
        state.resizing = true;
        e.preventDefault();
        updateDom();
    }

    function resize(e) {
        if (!state.resizing) {
            return;
        }

        e.preventDefault();

        let clientX
        let clientY;

        if (e.type.startsWith('touch')) {
            clientX = e.touches[0].clientX;
            clientY = e.touches[0].clientY;
        } else {
            clientX = e.clientX;
            clientY = e.clientY;
        }

        let newSize;
        if (state.layout === Layout.HORIZONTAL) {
            newSize = (clientX / main.clientWidth) * 100;
        } else {
            newSize = ((clientY - main.getBoundingClientRect().top) / main.clientHeight) * 100;
        }

        if (newSize > 20 && newSize < 80) {
            state.size = newSize;
        }

        updateDom();
    };

    function stopResize() {
        state.resizing = false;
        updateDom();
    }
});
