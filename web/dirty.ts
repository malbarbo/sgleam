/// Tracks whether the editor content has been modified since the last
/// successful load (Run).
export class DirtyTracker {
    private cleanCode = "";
    private _dirty = false;
    private _loading = false;

    get dirty(): boolean {
        return this._dirty;
    }

    /// Called when the editor content changes.
    onEdit(code: string): void {
        this._dirty = code !== this.cleanCode;
    }

    /// Called when a load (Run) is initiated.
    onLoad(): void {
        this._loading = true;
    }

    /// Called when a REPL expression is submitted.
    onRun(): void {
        this._loading = false;
    }

    /// Called when the worker signals ready (after load or run).
    onReady(hadErrors: boolean, currentCode: string): void {
        if (!hadErrors && this._loading) {
            this.cleanCode = currentCode;
            this._dirty = false;
        }
        this._loading = false;
    }

    /// Called when format completes.
    onFormatted(formattedCode: string): void {
        if (!this._dirty) {
            this.cleanCode = formattedCode;
        }
    }
}
