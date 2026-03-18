import {
    KEYDOWN,
    KEYPRESS,
    KEYUP,
    UIChannel,
    WorkerMessage,
} from "./ui_channel.ts";

class Player {
    private readonly channel: UIChannel;
    private readonly display: HTMLElement;
    private readonly status: HTMLElement;
    private readonly error: HTMLElement;

    constructor() {
        this.display = document.getElementById("display")!;
        this.status = document.getElementById("status")!;
        this.error = document.getElementById("error")!;

        const worker = new Worker("worker.js", { type: "module" });
        worker.onmessage = (e: MessageEvent<WorkerMessage>) =>
            this.onMessage(e);
        this.channel = new UIChannel(worker);
    }

    private onMessage(event: MessageEvent<WorkerMessage>): void {
        const data = event.data;
        switch (data.cmd) {
            case "error":
                this.status.style.display = "none";
                this.error.style.display = "block";
                this.error.textContent = data.data;
                break;
            case "progress":
                this.status.textContent = `Loading ${Math.round(data.data)}%`;
                break;
            case "ready":
                this.channel.setBuffer(data.buffer);
                this.status.textContent = "Compiling...";
                this.load();
                break;
            case "write":
                // Show stdout/stderr as errors if no SVG has appeared
                if (data.fd === 2 || !this.display.innerHTML) {
                    this.status.style.display = "none";
                    this.error.style.display = "block";
                    this.error.textContent += data.data;
                }
                break;
            case "svg":
                this.status.style.display = "none";
                this.error.style.display = "none";
                this.display.innerHTML = data.data;
                this.display.focus();
                break;
        }
    }

    private load(): void {
        const code = this.getCode();
        if (!code) {
            this.status.style.display = "none";
            this.error.style.display = "block";
            this.error.textContent =
                "No code provided.\n\nUsage: player.html#BASE64_ENCODED_GLEAM_CODE";
            return;
        }
        this.channel.load(code);
        this.setupKeyboard();
    }

    private getCode(): string | null {
        const hash = window.location.hash.slice(1);
        if (!hash) return null;
        try {
            return atob(hash);
        } catch {
            return null;
        }
    }

    private setupKeyboard(): void {
        const handler = (type: number) => (event: KeyboardEvent) => {
            event.preventDefault();
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
        this.display.addEventListener("keypress", handler(KEYPRESS));
        this.display.addEventListener("keydown", handler(KEYDOWN));
        this.display.addEventListener("keyup", handler(KEYUP));
    }
}

document.addEventListener("DOMContentLoaded", () => new Player());
