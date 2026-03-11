import { assertEquals } from "jsr:@std/assert";
import { ansiToHtml } from "./ansi.ts";

const ESC = "\x1b[";

Deno.test("plain text passes through unchanged", () => {
    assertEquals(ansiToHtml("hello world"), "hello world");
});

Deno.test("html entities are escaped", () => {
    assertEquals(ansiToHtml("a < b & c > d"), "a &lt; b &amp; c &gt; d");
});

Deno.test("bold", () => {
    assertEquals(
        ansiToHtml(`${ESC}1mhello${ESC}0m`),
        `<span style="font-weight:bold">hello</span>`,
    );
});

Deno.test("italic", () => {
    assertEquals(
        ansiToHtml(`${ESC}3mhello${ESC}0m`),
        `<span style="font-style:italic">hello</span>`,
    );
});

Deno.test("underline", () => {
    assertEquals(
        ansiToHtml(`${ESC}4mhello${ESC}0m`),
        `<span style="text-decoration:underline">hello</span>`,
    );
});

Deno.test("standard foreground colors", () => {
    const colors = [
        "#4d4d4d",
        "#cc0000",
        "#4e9a06",
        "#c4a000",
        "#3465a4",
        "#75507b",
        "#06989a",
        "#d3d7cf",
    ];
    for (let i = 0; i < 8; i++) {
        assertEquals(
            ansiToHtml(`${ESC}${30 + i}mtext${ESC}0m`),
            `<span style="color:${colors[i]}">text</span>`,
        );
    }
});

Deno.test("bright foreground colors", () => {
    const colors = [
        "#555753",
        "#ef2929",
        "#8ae234",
        "#fce94f",
        "#729fcf",
        "#ad7fa8",
        "#34e2e2",
        "#eeeeec",
    ];
    for (let i = 0; i < 8; i++) {
        assertEquals(
            ansiToHtml(`${ESC}${90 + i}mtext${ESC}0m`),
            `<span style="color:${colors[i]}">text</span>`,
        );
    }
});

Deno.test("combined params: bold + color", () => {
    assertEquals(
        ansiToHtml(`${ESC}1;35mhello${ESC}0m`),
        `<span style="font-weight:bold;color:#75507b">hello</span>`,
    );
});

Deno.test("empty params reset style", () => {
    assertEquals(
        ansiToHtml(`${ESC}1mhello${ESC}mworld`),
        `<span style="font-weight:bold">hello</span>world`,
    );
});

Deno.test("reset in the middle restores plain text", () => {
    assertEquals(
        ansiToHtml(`${ESC}31mred${ESC}0mnormal`),
        `<span style="color:#cc0000">red</span>normal`,
    );
});

Deno.test("text before and after escape sequence", () => {
    assertEquals(
        ansiToHtml(`before${ESC}1mbold${ESC}0mafter`),
        `before<span style="font-weight:bold">bold</span>after`,
    );
});

Deno.test("256-color: standard range (0-7)", () => {
    assertEquals(
        ansiToHtml(`${ESC}38;5;1mtext${ESC}0m`),
        `<span style="color:#cc0000">text</span>`,
    );
});

Deno.test("256-color: bright range (8-15)", () => {
    assertEquals(
        ansiToHtml(`${ESC}38;5;9mtext${ESC}0m`),
        `<span style="color:#ef2929">text</span>`,
    );
});

Deno.test("256-color: color cube", () => {
    // color 16 = rgb(0,0,0), color 231 = rgb(255,255,255)
    assertEquals(
        ansiToHtml(`${ESC}38;5;16mtext${ESC}0m`),
        `<span style="color:rgb(0,0,0)">text</span>`,
    );
    assertEquals(
        ansiToHtml(`${ESC}38;5;231mtext${ESC}0m`),
        `<span style="color:rgb(255,255,255)">text</span>`,
    );
});

Deno.test("256-color: grayscale ramp", () => {
    // color 232 = rgb(8,8,8), color 255 = rgb(238,238,238)
    assertEquals(
        ansiToHtml(`${ESC}38;5;232mtext${ESC}0m`),
        `<span style="color:rgb(8,8,8)">text</span>`,
    );
    assertEquals(
        ansiToHtml(`${ESC}38;5;255mtext${ESC}0m`),
        `<span style="color:rgb(238,238,238)">text</span>`,
    );
});

Deno.test("truecolor", () => {
    assertEquals(
        ansiToHtml(`${ESC}38;2;255;128;0mtext${ESC}0m`),
        `<span style="color:rgb(255,128,0)">text</span>`,
    );
});

Deno.test("multiple styled segments", () => {
    assertEquals(
        ansiToHtml(`${ESC}31mred${ESC}0m ${ESC}32mgreen${ESC}0m`),
        `<span style="color:#cc0000">red</span> <span style="color:#4e9a06">green</span>`,
    );
});

Deno.test("bold reset with code 22", () => {
    assertEquals(
        ansiToHtml(`${ESC}1mbold${ESC}22mnormal`),
        `<span style="font-weight:bold">bold</span>normal`,
    );
});

Deno.test("default fg color code 39 resets color", () => {
    assertEquals(
        ansiToHtml(`${ESC}31mred${ESC}39mnormal`),
        `<span style="color:#cc0000">red</span>normal`,
    );
});
