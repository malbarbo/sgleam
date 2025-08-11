const stdout = 1;
const stderr = 2;
const stopIndex = 0;
const sleepIndex = 1;

document.addEventListener('DOMContentLoaded', () => {
    // TODO: add gleam lang
    const flask = new CodeFlask(document.getElementById('editor-panel'), {
        language: 'js',
        lineNumbers: true
    });

    flask.updateCode(`import sgleam/check

pub fn hello(name: String) -> String {
  "Hello " <> name <> "!"
}

pub fn hello_examples() {
  check.eq(hello("World"), "Hello World!")
}
`);

    let replInput;
    let lastSvg;
    const main = document.getElementById('main');
    const loading = document.getElementById('loading');
    const runButton = document.getElementById('run-button');
    const stopButton = document.getElementById('stop-button');
    const resizeHandle = document.getElementById('resize-handle');
    const editorPanel = document.getElementById('editor-panel');
    const replPanel = document.getElementById('repl-panel');
    const helpOverlay = document.getElementById('help-overlay');
    const help = document.getElementById('help');
    const repl = new Worker('repl.js');
    let first = true;
    let runAfterFormat = false;

    let sharedBuffer = new SharedArrayBuffer(8);
    let sharedIntBuffer = new Int32Array(sharedBuffer);
    sharedIntBuffer.fill(0);
    repl.onmessage = (event) => {
        const data = event.data;
        if (data.cmd == 'error') {
            loading.textContent = data.data;
        } else if (data.cmd == 'progress') {
            loading.textContent = `Loading ${Math.round(data.data)}%`;
        } else if (data.cmd == 'ready') {
            lastSvg = null;
            if (first) {
                replPanel.replaceChildren()
                first = false;
            }
            addInputLine();
            runButton.disabled = false;
            stopButton.disabled = true;
            repl.postMessage({ cmd: 'init', data: sharedBuffer });
        } else if (data.cmd == 'format') {
            flask.updateCode(data.data);
            if (runAfterFormat) {
                runAfterFormat = false;
                run();
            }
        } else if (data.cmd == 'output') {
            addOutput(data.fd, data.data);
        } else if (data.cmd == 'svg') {
            addSvg(data.data);
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

    function focusEditor() {
        const input = editorPanel.querySelector('textarea:not([disabled])');
        if (isEditorVisible() && input) {
            input.focus();
        }
    }

    function focusRepl() {
        if (isReplVisible() && replInput) {
            replInput.focus();
        }
    }

    // Shortcuts

    document.addEventListener('keydown', (event) => {
        if (event.key === 'Escape') {
            event.preventDefault();
            hideHelp()
        } else if (helpOverlay.style.display == 'block') {
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
            formatThanRun();
        } else if (event.ctrlKey && event.key === 'f') {
            event.preventDefault();
            format();
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

    // Actions

    let lastActive;

    function showHelp() {
        if (document.activeElement && document.activeElement.blur) {
            lastActive = document.activeElement;
            document.activeElement.blur();
        } else {
            lastActive = null;
        }
        helpOverlay.style.display = 'block';
        help.style.display = 'block';
    }

    function hideHelp() {
        if (lastActive) {
            lastActive.focus();
        }
        helpOverlay.style.display = 'none';
        help.style.display = 'none';
    }

    function run() {
        if (!runButton.disabled) {
            replInput = null;
            replPanel.replaceChildren();
            postLoad();
        }
    }

    function formatThanRun() {
        runAfterFormat = true;
        format();
    }

    function stop() {
        if (!stop.disabled) {
            stopButton.disabled = true;
            Atomics.store(sharedIntBuffer, stopIndex, 1);
            Atomics.notify(sharedIntBuffer, sleepIndex, 1);
        }
    }

    function format() {
        repl.postMessage({ cmd: 'format', data: flask.getCode() });
    }

    function isReplVisible() {
        return replPanel.style.display !== 'none';
    }

    function isEditorVisible() {
        return editorPanel.style.display !== 'none';
    }

    function toogleEditor() {
        if (!isEditorVisible()) {
            editorPanel.style.display = 'flex';
            resizeHandle.style.display = 'initial';
        } else {
            replPanel.style.display = 'flex';
            editorPanel.style.display = 'none';
            resizeHandle.style.display = 'none';
            focusRepl();
        }
        updateEditorSize();
    }

    function toogleRepl() {
        if (!isReplVisible()) {
            replPanel.style.display = 'flex';
            resizeHandle.style.display = 'initial';
        } else {
            editorPanel.style.display = 'flex';
            replPanel.style.display = 'none';
            resizeHandle.style.display = 'none';
            focusEditor();
        }
        updateEditorSize();
    }

    function updateEditorSize() {
        if (isHorizontal()) {
            editorPanel.style.height = '100%';
            editorPanel.style.width = size;
        } else if (isReplVisible()) {
            editorPanel.style.width = '100%';
            editorPanel.style.height = size;
        } else {
            editorPanel.style.width = '100%';
            editorPanel.style.height = '100%';
        }
    }

    // Buttons

    runButton.addEventListener('click', formatThanRun);
    stopButton.addEventListener('click', stop);


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
        replPanel.appendChild(inputContainer);

        replInput.focus();

        replInput.addEventListener('paste', function (event) {
            event.preventDefault();

            const selection = window.getSelection();
            if (!selection.rangeCount) {
                return;
            }

            const text = event.clipboardData.getData('text/plain');
            const range = selection.getRangeAt(0);
            range.deleteContents();
            range.insertNode(document.createTextNode(text));
            selection.collapseToEnd();
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

    function addOutput(fd, text) {
        const output = document.createElement('div');
        output.className = 'repl-line';
        output.textContent = text;
        replPanel.appendChild(output);
        replPanel.scrollTop = replPanel.scrollHeight;
    }

    function addSvg(svg) {
        if (lastSvg) {
            lastSvg.innerHTML = svg;
        } else {
            lastSvg = document.createElement('div');
            lastSvg.innerHTML = svg;
            lastSvg.style.fontSize = "0";
            replPanel.appendChild(lastSvg);
            replPanel.scrollTop = replPanel.scrollHeight;
        }
    }

    // Focus

    replPanel.addEventListener('click', () => {
        if (window.getSelection().toString().length !== 0) {
            return;
        }
        focusRepl();
    });


    // Panel resizing and layout

    const layoutHorizontal = document.getElementById('layout-horizontal');
    const layoutVertical = document.getElementById('layout-vertical');
    let resizing = false;
    let size = '50%';

    layoutHorizontal.addEventListener('click', enableHorizontal);
    layoutVertical.addEventListener('click', enableVertical)

    function isHorizontal() {
        return window.getComputedStyle(main).flexDirection === 'row';
    }

    function toogleLayout() {
        if (isHorizontal()) {
            enableVertical();
        } else {
            enableHorizontal();
        }
    }

    function enableHorizontal() {
        main.style.flexDirection = 'row';

        resizeHandle.style.cursor = 'col-resize';
        resizeHandle.style.width = '8px';
        resizeHandle.style.height = '100%';

        updateEditorSize();

        layoutHorizontal.disabled = true;
        layoutVertical.disabled = false;
    }

    function enableVertical() {
        main.style.flexDirection = 'column';

        resizeHandle.style.cursor = 'row-resize';
        resizeHandle.style.width = '100%';
        resizeHandle.style.height = '8px';

        updateEditorSize();

        layoutHorizontal.disabled = false;
        layoutVertical.disabled = true;
    }

    function startResize(e) {
        resizing = true;
        if (isHorizontal()) {
            document.body.style.cursor = 'col-resize';
        } else {
            document.body.style.cursor = 'row-resize';
        }
        e.preventDefault();
    }

    function resize(e) {
        if (!resizing) {
            return;
        }

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
        if (layoutHorizontal.disabled) {
            newSize = (clientX / main.clientWidth) * 100;
        } else {
            newSize = ((clientY - main.getBoundingClientRect().top) / main.clientHeight) * 100;
        }

        if (newSize > 20 && newSize < 80) {
            size = `${newSize}%`;
            if (layoutHorizontal.disabled) {
                editorPanel.style.width = size;
            } else {
                editorPanel.style.height = size;
            }
        }
    };

    function stopResize() {
        resizing = false;
        document.body.style.cursor = '';
    }

    resizeHandle.addEventListener('mousedown', startResize);
    document.addEventListener('mousemove', resize)
    document.addEventListener('mouseup', stopResize);

    resizeHandle.addEventListener('touchstart', startResize);
    document.addEventListener('touchmove', resize)
    document.addEventListener('touchend', stopResize);
});
