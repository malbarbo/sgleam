import { ansiToHtml } from "./ansi.ts";
import {
    KEYDOWN,
    KEYPRESS,
    KEYUP,
    UIChannel,
    WorkerMessage,
} from "./ui_channel.ts";

// deno-lint-ignore no-explicit-any
declare const Prism: {
    highlight(code: string, grammar: any, language: string): string;
    languages: Record<string, any>;
};

declare class CodeFlask {
    constructor(
        el: HTMLElement,
        options: { language: string; lineNumbers: boolean },
    );
    // deno-lint-ignore no-explicit-any
    addLanguage(name: string, grammar: Record<string, any>): void;
    updateCode(code: string): void;
    getCode(): string;
    onUpdate(callback: (code: string) => void): void;
}

type AppState =
    | { kind: "loading"; progress: number }
    | { kind: "error"; message: string }
    | {
        kind: "ready";
        running: boolean;
        dirty: boolean;
        layout: "horizontal" | "vertical";
        view: "split" | "editor" | "repl";
        splitSize: number;
        helpVisible: boolean;
        resizing: boolean;
    };

class App {
    private state: AppState = { kind: "loading", progress: 0 };
    private runAfterFormat = false;
    private cleanCode = "";
    private replInput: HTMLTextAreaElement | null = null;
    private replPrompt: HTMLDivElement | null = null;
    private lastSvg: HTMLDivElement | null = null;
    private lastActive: HTMLElement | null = null;

    private readonly flask: CodeFlask;
    private readonly channel: UIChannel;
    private readonly main: HTMLElement;
    private readonly loading: HTMLElement;
    private readonly runButton: HTMLButtonElement;
    private readonly stopButton: HTMLButtonElement;
    private readonly resizeHandle: HTMLElement;
    private readonly editorPanel: HTMLElement;
    private readonly replPanel: HTMLElement;
    private readonly helpOverlay: HTMLElement;
    private readonly help: HTMLElement;
    private readonly layoutHorizontal: HTMLButtonElement;
    private readonly layoutVertical: HTMLButtonElement;

    private readonly themeToggle: HTMLButtonElement;
    private readonly fileInput: HTMLInputElement;

    private readonly shortcuts = new Map<string, () => void>([
        ["ctrl+?", () => this.showHelp()],
        ["ctrl+j", () => this.focusEditor()],
        ["ctrl+k", () => this.focusRepl()],
        ["ctrl+r", () => this.formatThenRun()],
        ["ctrl+f", () => this.format()],
        ["ctrl+s", () => this.saveFile()],
        ["ctrl+o", () => this.loadFile()],
        ["ctrl+d", () => this.toggleEditor()],
        ["ctrl+i", () => this.toggleRepl()],
        ["ctrl+l", () => this.toggleLayout()],
        ["ctrl+t", () => this.toggleTheme()],
    ]);

    constructor() {
        this.main = document.getElementById("main")!;
        this.loading = document.getElementById("loading")!;
        this.runButton = document.getElementById(
            "run-button",
        )! as HTMLButtonElement;
        this.stopButton = document.getElementById(
            "stop-button",
        )! as HTMLButtonElement;
        this.resizeHandle = document.getElementById("resize-handle")!;
        this.editorPanel = document.getElementById("editor-panel")!;
        this.replPanel = document.getElementById("repl-panel")!;
        this.helpOverlay = document.getElementById("help-overlay")!;
        this.help = document.getElementById("help")!;
        this.layoutHorizontal = document.getElementById(
            "layout-horizontal",
        )! as HTMLButtonElement;
        this.layoutVertical = document.getElementById(
            "layout-vertical",
        )! as HTMLButtonElement;

        this.themeToggle = document.getElementById(
            "theme-toggle",
        )! as HTMLButtonElement;

        this.fileInput = document.createElement("input");
        this.fileInput.type = "file";
        this.fileInput.accept = ".gleam,.txt";
        this.fileInput.style.display = "none";
        document.body.appendChild(this.fileInput);
        this.fileInput.addEventListener("change", () => this.onFileSelected());

        const savedTheme = localStorage.getItem("sgleam-theme") ?? "light";
        document.documentElement.setAttribute("data-theme", savedTheme);

        this.flask = new CodeFlask(this.editorPanel, {
            language: "gleam",
            lineNumbers: true,
        });
        this.flask.addLanguage("gleam", {
            "comment": /\/\/.*/,
            "string": { pattern: /"(?:\\.|[^"\\])*"/, greedy: true },
            "keyword":
                /\b(?:as|assert|case|const|else|external|fn|if|import|let|opaque|panic|pub|todo|type|use)\b/,
            "boolean": /\b(?:True|False|Nil)\b/,
            "class-name": /\b[A-Z][a-zA-Z0-9_]*\b/,
            "function": /\b[a-z_][a-z0-9_]*(?=\s*\()/,
            "number": /\b\d[\d_]*(?:\.[\d_]+)?\b/,
            "operator":
                /->|\|>|<>|\.\.|\+\.|-\.|\/\.|<=\.|>=\.|<\.|>\.|==|!=|<=|>=|&&|\|\||[+\-*/%<>|]/,
            "punctuation": /[{}()\[\]:,.#]/,
        });
        this.flask.updateCode(
            document.getElementById("default-code")?.textContent ?? "",
        );
        this.flask.onUpdate((code: string) => {
            if (this.state.kind === "ready" && !this.state.running) {
                const dirty = code !== this.cleanCode;
                if (dirty !== this.state.dirty) {
                    this.state.dirty = dirty;
                    this.render();
                }
            }
        });

        const worker = new Worker("worker.js", { type: "module" });
        worker.onmessage = (e: MessageEvent<WorkerMessage>) =>
            this.onWorkerMessage(e);

        this.channel = new UIChannel(worker);

        this.setupListeners();
        this.render();
    }

    private setupListeners(): void {
        this.themeToggle.addEventListener("click", () => this.toggleTheme());
        document.getElementById("open-button")!
            .addEventListener("click", () => this.loadFile());
        document.getElementById("save-button")!
            .addEventListener("click", () => this.saveFile());
        document.getElementById("help-button")!
            .addEventListener("click", () => this.showHelp());
        this.runButton.addEventListener("click", () => this.formatThenRun());
        this.stopButton.addEventListener("click", () => this.stop());
        this.layoutHorizontal.addEventListener(
            "click",
            () => this.setLayout("horizontal"),
        );
        this.layoutVertical.addEventListener(
            "click",
            () => this.setLayout("vertical"),
        );
        this.replPanel.addEventListener("click", () => this.onReplPanelClick());
        this.resizeHandle.addEventListener(
            "mousedown",
            (e) => this.startResize(e),
        );
        document.addEventListener("mousemove", (e) => this.resize(e));
        document.addEventListener("mouseup", () => this.stopResize());
        this.resizeHandle.addEventListener(
            "touchstart",
            (e) => this.startResize(e),
        );
        document.addEventListener("touchmove", (e) => this.resize(e));
        document.addEventListener("touchend", () => this.stopResize());
        document.addEventListener("keydown", (e) => this.onKeyDown(e));
    }

    private onWorkerMessage(event: MessageEvent<WorkerMessage>): void {
        const data = event.data;
        switch (data.cmd) {
            case "error":
                this.state = { kind: "error", message: data.data };
                this.render();
                break;
            case "progress":
                this.state = { kind: "loading", progress: data.data };
                this.render();
                break;
            case "ready": {
                const prev = this.state;
                this.state = prev.kind === "ready"
                    ? {
                        ...prev,
                        running: false,
                        dirty: data.hadErrors ? prev.dirty : false,
                    }
                    : {
                        kind: "ready",
                        running: false,
                        dirty: true,
                        layout: window.innerWidth >= window.innerHeight
                            ? "horizontal"
                            : "vertical",
                        view: "split",
                        splitSize: 50,
                        helpVisible: false,
                        resizing: false,
                    };
                if (!this.state.dirty) {
                    this.cleanCode = this.flask.getCode();
                }
                this.render();
                this.lastSvg = null;
                if (prev.kind !== "ready") {
                    this.replPanel.replaceChildren();
                }
                this.channel.setBuffer(data.buffer);
                this.addInputLine();
                break;
            }
            case "formatted":
                if (
                    this.state.kind === "ready" && !this.state.dirty
                ) {
                    this.cleanCode = data.data;
                }
                this.flask.updateCode(data.data);
                if (this.runAfterFormat) {
                    this.runAfterFormat = false;
                    this.run();
                }
                break;
            case "write":
                this.addOutput(data.fd, data.data);
                break;
            case "svg":
                this.addSvg(data.data);
                break;
        }
    }

    private onKeyDown(event: KeyboardEvent): void {
        if (event.key === "Escape") {
            event.preventDefault();
            if (this.state.kind === "ready") this.hideHelp();
            return;
        }
        if (this.state.kind !== "ready" || this.state.helpVisible) return;
        const combo = `${event.ctrlKey ? "ctrl+" : ""}${event.key}`;
        const action = this.shortcuts.get(combo);
        if (action) {
            event.preventDefault();
            action();
        }
    }

    private onReplPanelClick(): void {
        if (window.getSelection()?.toString().length !== 0) return;
        this.focusRepl();
    }

    private render(): void {
        const s = this.state;
        if (s.kind === "loading") {
            this.loading.style.display = "block";
            this.loading.textContent = `Loading ${Math.round(s.progress)}%`;
        } else if (s.kind === "error") {
            this.loading.style.display = "block";
            this.loading.textContent = s.message;
        } else {
            this.loading.style.display = "none";
        }

        this.runButton.disabled = s.kind !== "ready" || s.running;
        this.stopButton.disabled = s.kind !== "ready" || !s.running;

        if (s.kind !== "ready") return;

        this.main.style.flexDirection = s.layout === "horizontal"
            ? "row"
            : "column";
        this.layoutHorizontal.disabled = s.layout === "horizontal";
        this.layoutVertical.disabled = s.layout === "vertical";
        this.resizeHandle.style.cursor = s.layout === "horizontal"
            ? "col-resize"
            : "row-resize";
        this.resizeHandle.style.width = s.layout === "horizontal"
            ? "8px"
            : "100%";
        this.resizeHandle.style.height = s.layout === "horizontal"
            ? "100%"
            : "8px";

        this.editorPanel.style.display = s.view !== "repl" ? "flex" : "none";
        this.replPanel.style.display = s.view !== "editor" ? "flex" : "none";
        this.resizeHandle.style.display = s.view === "split"
            ? "initial"
            : "none";

        this.helpOverlay.style.display = s.helpVisible ? "block" : "none";
        this.help.style.display = s.helpVisible ? "block" : "none";

        document.body.style.cursor = s.resizing
            ? s.layout === "horizontal" ? "col-resize" : "row-resize"
            : "";

        if (s.layout === "horizontal") {
            this.editorPanel.style.height = "100%";
            this.editorPanel.style.width = `${s.splitSize}%`;
        } else if (s.view !== "editor") {
            this.editorPanel.style.width = "100%";
            this.editorPanel.style.height = `${s.splitSize}%`;
        } else {
            this.editorPanel.style.width = "100%";
            this.editorPanel.style.height = "100%";
        }

        if (this.replPrompt) {
            this.replPrompt.textContent = s.dirty ? "\u25CF >" : ">";
        }
    }

    private postLoad(): void {
        if (this.state.kind !== "ready") return;
        this.state.running = true;
        this.render();
        this.channel.load(this.flask.getCode());
    }

    private postRun(code: string): void {
        if (this.state.kind !== "ready") return;
        this.state.running = true;
        this.render();
        this.channel.run(code);
    }

    private focusEditor(): void {
        const s = this.state;
        if (s.kind !== "ready") return;
        const input = this.editorPanel.querySelector<HTMLTextAreaElement>(
            "textarea:not([disabled])",
        );
        if (s.view !== "repl" && input) input.focus();
    }

    private focusRepl(): void {
        const s = this.state;
        if (s.kind !== "ready") return;
        if (s.view !== "editor" && this.replInput) this.replInput.focus();
    }

    private showHelp(): void {
        if (this.state.kind !== "ready") return;
        this.state.helpVisible = true;
        this.render();
        if (document.activeElement instanceof HTMLElement) {
            this.lastActive = document.activeElement;
            this.lastActive.blur();
        } else {
            this.lastActive = null;
        }
    }

    private hideHelp(): void {
        if (this.state.kind !== "ready") return;
        this.state.helpVisible = false;
        this.render();
        this.lastActive?.focus();
    }

    private run(): void {
        const s = this.state;
        if (s.kind === "ready" && !s.running) {
            this.replInput = null;
            this.replPanel.replaceChildren();
            this.postLoad();
        }
    }

    private formatThenRun(): void {
        if (this.state.kind !== "ready") return;
        this.runAfterFormat = true;
        this.format();
    }

    private stop(): void {
        const s = this.state;
        if (s.kind === "ready" && s.running) {
            s.running = false;
            this.render();
            this.channel.stop();
        }
    }

    private format(): void {
        this.channel.format(this.flask.getCode());
    }

    private toggleEditor(): void {
        if (this.state.kind !== "ready") return;
        const hiding = this.state.view !== "repl";
        this.state.view = hiding ? "repl" : "split";
        this.render();
        if (hiding) this.focusRepl();
    }

    private toggleRepl(): void {
        if (this.state.kind !== "ready") return;
        const hiding = this.state.view !== "editor";
        this.state.view = hiding ? "editor" : "split";
        this.render();
        if (hiding) this.focusEditor();
    }

    private async saveFile(): Promise<void> {
        const code = this.flask.getCode();
        const blob = new Blob([code], { type: "text/plain" });

        // deno-lint-ignore no-explicit-any
        if (typeof (globalThis as any).showSaveFilePicker === "function") {
            try {
                // deno-lint-ignore no-explicit-any
                const handle = await (globalThis as any).showSaveFilePicker({
                    suggestedName: "code.gleam",
                    types: [{
                        description: "Gleam files",
                        accept: { "text/plain": [".gleam"] },
                    }],
                });
                const writable = await handle.createWritable();
                await writable.write(blob);
                await writable.close();
                return;
            } catch (e) {
                if (e instanceof DOMException && e.name === "AbortError") {
                    return;
                }
            }
        }

        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "code.gleam";
        a.click();
        URL.revokeObjectURL(url);
    }

    private loadFile(): void {
        this.fileInput.click();
    }

    private onFileSelected(): void {
        const file = this.fileInput.files?.[0];
        if (!file) return;
        const reader = new FileReader();
        reader.onload = () => {
            if (typeof reader.result === "string") {
                this.flask.updateCode(reader.result);
                if (this.state.kind === "ready") {
                    this.state.dirty = true;
                    this.render();
                }
            }
        };
        reader.readAsText(file);
        this.fileInput.value = "";
    }

    private toggleTheme(): void {
        const current = document.documentElement.getAttribute("data-theme") ??
            "light";
        const next = current === "light" ? "dark" : "light";
        document.documentElement.setAttribute("data-theme", next);
        localStorage.setItem("sgleam-theme", next);
    }

    private toggleLayout(): void {
        if (this.state.kind !== "ready") return;
        this.setLayout(
            this.state.layout === "horizontal" ? "vertical" : "horizontal",
        );
    }

    private setLayout(layout: "horizontal" | "vertical"): void {
        if (this.state.kind !== "ready") return;
        this.state.layout = layout;
        this.render();
    }

    private startResize(e: Event): void {
        e.preventDefault();
        if (this.state.kind !== "ready") return;
        this.state.resizing = true;
        this.render();
    }

    private resize(e: Event): void {
        const s = this.state;
        if (s.kind !== "ready" || !s.resizing) return;

        let clientX: number;
        let clientY: number;

        if (typeof TouchEvent !== "undefined" && e instanceof TouchEvent) {
            clientX = e.touches[0].clientX;
            clientY = e.touches[0].clientY;
        } else {
            clientX = (e as MouseEvent).clientX;
            clientY = (e as MouseEvent).clientY;
        }

        let newSize: number;
        if (s.layout === "horizontal") {
            newSize = (clientX / this.main.clientWidth) * 100;
        } else {
            newSize = ((clientY - this.main.getBoundingClientRect().top) /
                this.main.clientHeight) *
                100;
        }

        if (newSize > 20 && newSize < 80) {
            s.splitSize = newSize;
            this.render();
        }
    }

    private stopResize(): void {
        if (this.state.kind !== "ready") return;
        this.state.resizing = false;
        this.render();
    }

    private highlightGleam(code: string): string {
        if (Prism.languages.gleam) {
            return Prism.highlight(code, Prism.languages.gleam, "gleam");
        }
        return code;
    }

    private addInputLine(): void {
        const inputContainer = document.createElement("div");
        inputContainer.className = "repl-input-container";

        const dirty = this.state.kind === "ready" && this.state.dirty;
        const prompt = (this.replPrompt = document.createElement("div"));
        prompt.className = "repl-prompt";
        prompt.textContent = dirty ? "\u25CF >" : ">";

        const wrapper = document.createElement("div");
        wrapper.className = "repl-input-wrapper";

        const highlight = document.createElement("pre");
        highlight.className = "repl-input-highlight";

        const textarea = (this.replInput = document.createElement("textarea"));
        textarea.className = "repl-input-edit";
        textarea.rows = 1;
        textarea.spellcheck = false;

        wrapper.appendChild(highlight);
        wrapper.appendChild(textarea);
        inputContainer.appendChild(prompt);
        inputContainer.appendChild(wrapper);
        this.replPanel.appendChild(inputContainer);

        textarea.focus();

        const syncHighlight = () => {
            highlight.innerHTML = this.highlightGleam(textarea.value) || "\n";
            textarea.style.height = "auto";
            textarea.style.height = textarea.scrollHeight + "px";
        };
        syncHighlight();
        textarea.addEventListener("input", syncHighlight);

        textarea.addEventListener("keydown", (e: KeyboardEvent) => {
            if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                const code = textarea.value.trim();
                if (code) {
                    textarea.disabled = true;
                    this.postRun(code);
                }
            }
        });
    }

    private addOutput(_fd: number, text: string): void {
        const html = ansiToHtml(text);
        const output = document.createElement("div");
        output.className = "repl-line";
        output.innerHTML = html;
        this.replPanel.appendChild(output);
        this.replPanel.scrollTop = this.replPanel.scrollHeight;
    }

    private addSvg(svg: string): void {
        if (!this.lastSvg) {
            this.lastSvg = document.createElement("div");
            this.lastSvg.style.fontSize = "0";
            this.lastSvg.style.outline = "none";
            this.replPanel.appendChild(this.lastSvg);
            this.lastSvg.tabIndex = 0;
            const handler = (type: number) => (event: KeyboardEvent) => {
                this.channel.enqueueKeyEvent({
                    type,
                    key: event.key,
                    alt: event.altKey,
                    ctrl: event.ctrlKey,
                    shift: event.shiftKey,
                    meta: event.metaKey,
                    repeat: event.repeat,
                });
            };
            this.lastSvg.addEventListener("keypress", handler(KEYPRESS));
            this.lastSvg.addEventListener("keydown", handler(KEYDOWN));
            this.lastSvg.addEventListener("keyup", handler(KEYUP));
            this.lastSvg.focus();
            this.replPanel.scrollTop = this.replPanel.scrollHeight;
        }
        this.lastSvg.innerHTML = svg;
    }
}

document.addEventListener("DOMContentLoaded", () => new App());
