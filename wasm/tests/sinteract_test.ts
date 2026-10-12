import { assertEquals } from "jsr:@std/assert";

// deno.json cannot read Cargo.lock, so the revision of the room is written
// there by hand, and this test fails when the two drift apart.
Deno.test("the room of sinteract is at the revision of Cargo.lock", async () => {
  const read = (path: string) =>
    Deno.readTextFile(new URL(path, import.meta.url));
  const lock = await read("../../Cargo.lock");
  const config = await read("deno.json");
  const locked = lock.match(
    /name = "sinteract"\n[^\[]*?source = "git\+[^#]*#([0-9a-f]{40})"/,
  )?.[1];
  const imported = config.match(/malbarbo\/sinteract\/([0-9a-f]{40})\//)?.[1];
  assertEquals(imported, locked);
});
