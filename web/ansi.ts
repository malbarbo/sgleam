// Converts ANSI SGR escape sequences to HTML spans with inline styles.
// Handles: reset, bold, italic, underline, standard/bright fg colors (30-37,
// 90-97), 256-color fg (38;5;n), and truecolor fg (38;2;r;g;b).

const ANSI_RE = /\x1b\[([0-9;]*)m/g;

// deno-fmt-ignore
const STANDARD_COLORS = [
    "#4d4d4d", "#cc0000", "#4e9a06", "#c4a000",
    "#3465a4", "#75507b", "#06989a", "#d3d7cf",
];

// deno-fmt-ignore
const BRIGHT_COLORS = [
    "#555753", "#ef2929", "#8ae234", "#fce94f",
    "#729fcf", "#ad7fa8", "#34e2e2", "#eeeeec",
];

interface Style {
    bold: boolean;
    italic: boolean;
    underline: boolean;
    fg: string | null;
}

function defaultStyle(): Style {
    return { bold: false, italic: false, underline: false, fg: null };
}

function applyParams(style: Style, params: string): Style {
    const codes = params === "" ? [0] : params.split(";").map(Number);
    const s = { ...style };
    let i = 0;
    while (i < codes.length) {
        const c = codes[i];
        if (c === 0) {
            Object.assign(s, defaultStyle());
        } else if (c === 1) {
            s.bold = true;
        } else if (c === 3) {
            s.italic = true;
        } else if (c === 4) {
            s.underline = true;
        } else if (c === 22) {
            s.bold = false;
        } else if (c === 23) {
            s.italic = false;
        } else if (c === 24) {
            s.underline = false;
        } else if (c >= 30 && c <= 37) {
            s.fg = STANDARD_COLORS[c - 30];
        } else if (c === 38) {
            if (codes[i + 1] === 5 && i + 2 < codes.length) {
                s.fg = color256(codes[i + 2]);
                i += 2;
            } else if (codes[i + 1] === 2 && i + 4 < codes.length) {
                s.fg = `rgb(${codes[i + 2]},${codes[i + 3]},${codes[i + 4]})`;
                i += 4;
            }
        } else if (c === 39) {
            s.fg = null;
        } else if (c >= 90 && c <= 97) {
            s.fg = BRIGHT_COLORS[c - 90];
        }
        i++;
    }
    return s;
}

function styledSpan(text: string, style: Style): string {
    const css: string[] = [];
    if (style.bold) css.push("font-weight:bold");
    if (style.italic) css.push("font-style:italic");
    if (style.underline) css.push("text-decoration:underline");
    if (style.fg) css.push(`color:${style.fg}`);
    if (css.length === 0) return text;
    return `<span style="${css.join(";")}">${text}</span>`;
}

function escapeHtml(text: string): string {
    return text
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;");
}

function color256(n: number): string {
    if (n < 8) return STANDARD_COLORS[n];
    if (n < 16) return BRIGHT_COLORS[n - 8];
    if (n < 232) {
        const v = n - 16;
        const r = Math.floor(v / 36);
        const g = Math.floor(v / 6) % 6;
        const b = v % 6;
        const ch = (x: number) => (x === 0 ? 0 : 55 + x * 40);
        return `rgb(${ch(r)},${ch(g)},${ch(b)})`;
    }
    const gray = 8 + (n - 232) * 10;
    return `rgb(${gray},${gray},${gray})`;
}

export function ansiToHtml(text: string): string {
    let result = "";
    let style = defaultStyle();
    let last = 0;

    for (const match of text.matchAll(ANSI_RE)) {
        const plain = text.slice(last, match.index);
        if (plain) result += styledSpan(escapeHtml(plain), style);
        style = applyParams(style, match[1]);
        last = match.index! + match[0].length;
    }

    const tail = text.slice(last);
    if (tail) result += styledSpan(escapeHtml(tail), style);

    return result;
}
