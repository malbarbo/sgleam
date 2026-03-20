import { assertEquals } from "jsr:@std/assert";
import { DirtyTracker } from "./dirty.ts";

Deno.test("initially not dirty", () => {
    const t = new DirtyTracker();
    assertEquals(t.dirty, false);
});

Deno.test("edit makes it dirty", () => {
    const t = new DirtyTracker();
    t.onEdit("changed");
    assertEquals(t.dirty, true);
});

Deno.test("edit back to clean code clears dirty", () => {
    const t = new DirtyTracker();
    t.onEdit("changed");
    assertEquals(t.dirty, true);
    t.onEdit("");
    assertEquals(t.dirty, false);
});

Deno.test("load then ready clears dirty", () => {
    const t = new DirtyTracker();
    t.onEdit("code");
    assertEquals(t.dirty, true);
    t.onLoad();
    t.onReady(false, "code");
    assertEquals(t.dirty, false);
});

Deno.test("load with errors keeps dirty", () => {
    const t = new DirtyTracker();
    t.onEdit("code");
    t.onLoad();
    t.onReady(true, "code");
    assertEquals(t.dirty, true);
});

Deno.test("repl run does not clear dirty", () => {
    const t = new DirtyTracker();
    t.onEdit("code");
    assertEquals(t.dirty, true);
    t.onRun();
    t.onReady(false, "code");
    assertEquals(t.dirty, true);
});

Deno.test("initial edit then repl run stays dirty", () => {
    const t = new DirtyTracker();
    // Simulate initial ready: editor has code, no load was done
    t.onEdit("initial code");
    assertEquals(t.dirty, true);
    // User enters a REPL expression without running (loading) the definitions
    t.onRun();
    t.onReady(false, "initial code");
    assertEquals(t.dirty, true);
});

Deno.test("successful load updates clean code", () => {
    const t = new DirtyTracker();
    t.onLoad();
    t.onReady(false, "code");
    assertEquals(t.dirty, false);
    t.onEdit("code");
    assertEquals(t.dirty, false);
    t.onEdit("other");
    assertEquals(t.dirty, true);
});

Deno.test("format updates clean code when not dirty", () => {
    const t = new DirtyTracker();
    t.onLoad();
    t.onReady(false, "code");
    t.onFormatted("formatted");
    assertEquals(t.dirty, false);
    t.onEdit("formatted");
    assertEquals(t.dirty, false);
});

Deno.test("format does not update clean code when dirty", () => {
    const t = new DirtyTracker();
    t.onLoad();
    t.onReady(false, "code");
    t.onEdit("changed");
    t.onFormatted("formatted-changed");
    assertEquals(t.dirty, true);
    t.onEdit("code");
    assertEquals(t.dirty, false);
});
