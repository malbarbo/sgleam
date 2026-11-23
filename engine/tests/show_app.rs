//! The loop of `sgleam/show/app` against a page that the test fakes in js. A
//! native run has no page, so each test puts the functions of one on `sgleam`
//! before the app starts.

use engine::{
    engine::{Engine, MainFunction},
    gleam::Project,
    host::HostOptions,
    quickjs::QuickJsEngine,
};

fn run(app: &str, page: &str) {
    let mut project = Project::default();
    project.write_source("show_app.gleam", app);
    project.compile(false).expect("compile the app");
    project.write_out("page.mjs", page);
    QuickJsEngine::new(project.fs, HostOptions::default())
        .expect("start the engine")
        .run_main("show_app", MainFunction::Main)
        .expect("run the app");
}

const IMPORTS: &str = r#"
import gleam/int
import sgleam/show/app
import sgleam/show/button
import sgleam/task
@external(javascript, "./page.mjs", "setup")
fn setup() -> Nil
@external(javascript, "./page.mjs", "verify")
fn verify() -> Nil
"#;

// A page whose frames are numbered from 1 and whose `log` holds every frame,
// wait and request. `events` waits to be read.
const PAGE: &str = r#"
export const log = [];
export const events = [];
export let frame = 0;
export function setup() {
    sgleam.draw_view = html => { log.push(html); return ++frame; };
    sgleam.next_view_event = () => events.length ? JSON.stringify(events.shift()) : "";
    sgleam.wait_view_event = ms => { log.push("wait " + ms); on_wait(ms); };
}
export function click(f) { events.push({kind: "ui", f, h: 0, e: "click"}); }
"#;

#[test]
fn a_wake_up_comes_between_events_and_does_not_hold_them() {
    run(
        &format!(
            r#"{IMPORTS}
type Msg {{ Tick Click }}
pub fn main() {{
  setup()
  app.create(fn() {{ #(0, task.wake_up(30, fn(_) {{ task.done(Tick) }})) }},
    fn(n) {{ button.new(int.to_string(n)) |> button.on_press(Click) |> button.done }},
    fn(n, msg) {{
      case msg {{
        Tick -> #(n + 1, task.wake_up(30, fn(_) {{ task.done(Tick) }}))
        Click -> #(n + 100, task.none())
      }}
    }})
  |> app.stop_when(fn(n) {{ n % 100 >= 3 }})
  |> app.run
  verify()
}}
"#
        ),
        &format!(
            r#"{PAGE}
click(1);
function on_wait(ms) {{
    if (ms < 0) throw new Error("waits forever with a wake-up pending");
    sgleam.sleep(ms);
}}
export function verify() {{
    const frames = log.filter(entry => !entry.startsWith("wait"));
    if (!frames.at(-1).includes(">103<")) throw new Error(JSON.stringify(log));
}}
"#
        ),
    );
}

#[test]
fn an_event_runs_the_handler_of_the_frame_it_came_from() {
    run(
        &format!(
            r#"{IMPORTS}
import sgleam/show/layout
type Msg {{ Up Down }}
pub fn main() {{
  setup()
  app.create(fn() {{ #(0, task.none()) }},
    // Handler 0 is Up on the first frame and Down on every other.
    fn(n) {{
      let up = button.new("up") |> button.on_press(Up) |> button.done
      let down = button.new("down") |> button.on_press(Down) |> button.done
      case n {{
        0 -> layout.column([up, down]) |> layout.done
        _ -> layout.column([down, up]) |> layout.done
      }}
    }},
    fn(n, msg) {{
      case msg {{
        Up -> #(n + 10, task.none())
        Down -> #(n + 1, task.none())
      }}
    }})
  |> app.stop_when(fn(n) {{ n >= 21 }})
  |> app.run
  verify()
}}
"#
        ),
        &format!(
            r#"{PAGE}
// Two clicks on Up of the first frame, the second read after the first drew
// the next frame, and one from a frame the engine never drew.
click(1);
click(1);
events.push({{kind: "ui", f: 999, h: 0, e: "click"}});
function on_wait(ms) {{
    click(frame);
}}
export function verify() {{
    const waits = log.filter(entry => entry.startsWith("wait")).length;
    if (waits !== 1) throw new Error(JSON.stringify(log));
}}
"#
        ),
    );
}

#[test]
fn a_request_gives_its_body_or_the_error_of_its_answer() {
    run(
        &format!(
            r#"{IMPORTS}
import gleam/string
pub fn main() {{
  setup()
  let get = fn(url, next) {{ task.get(url, fn(r) {{ next(string.inspect(r)) }}) }}
  let all = {{
    use a <- get("ok")
    use b <- get("404")
    use c <- get("bad")
    use d <- get("slow")
    use e <- get("down")
    use f <- task.post("echo", task.string_body("text/plain", "hi"))
    task.done(string.join([a, b, c, d, e, string.inspect(f)], ";"))
  }}
  app.create(fn() {{ #("", all) }},
    fn(s) {{ button.new(s) |> button.done }},
    fn(_, s) {{ #(s, task.none()) }})
  |> app.stop_when(fn(s) {{ s != "" }})
  |> app.run
  verify()
}}
"#
        ),
        &format!(
            r#"{PAGE}
const answers = {{
    ok: {{ok: true, status: 200, body: "a"}},
    404: {{ok: true, status: 404, body: ""}},
    bad: {{ok: false, error: "bad_url"}},
    slow: {{ok: false, error: "timeout"}},
    down: {{ok: false, error: "network"}},
}};
sgleam.http_request = json => {{
    const request = JSON.parse(json);
    log.push(json);
    return JSON.stringify(answers[request.url] ??
        {{ok: true, status: 201, body: request.method + " " + request.content_type + " " + request.body}});
}};
function on_wait() {{ throw new Error("waited"); }}
export function verify() {{
    const want = 'Ok(&quot;a&quot;);Error(BadStatus(status: 404));Error(BadUrl(url: &quot;bad&quot;));' +
        'Error(Timeout);Error(NetworkError);Ok(&quot;POST text/plain hi&quot;)';
    if (!log.at(-1).includes(want)) throw new Error(JSON.stringify(log));
    if (log.filter(e => e.startsWith("{{")).some(e => JSON.parse(e).url === "ok" && "body" in JSON.parse(e))) {{
        throw new Error("a GET sent a body");
    }}
}}
"#
        ),
    );
}

#[test]
fn a_long_run_does_not_grow_the_stack() {
    run(
        &format!(
            r#"{IMPORTS}
pub fn main() {{
  setup()
  let tick = fn(_) {{ task.done(Nil) }}
  app.create(fn() {{ #(0, task.wake_up(0, tick)) }},
    fn(n) {{ button.new(int.to_string(n)) |> button.done }},
    fn(n, _) {{ #(n + 1, task.wake_up(0, tick)) }})
  |> app.stop_when(fn(n) {{ n >= 20000 }})
  |> app.run
  verify()
}}
"#
        ),
        &format!(
            r#"{PAGE}
function on_wait(ms) {{ if (ms > 0) throw new Error("waited " + ms); }}
export function verify() {{
    if (!log.at(-1).includes(">20000<")) throw new Error(log.at(-1));
}}
"#
        ),
    );
}

#[test]
fn a_wake_up_that_is_always_due_does_not_hold_events() {
    run(
        &format!(
            r#"{IMPORTS}
type Msg {{ Tick Click }}
pub fn main() {{
  setup()
  let tick = fn(_) {{ task.done(Tick) }}
  app.create(fn() {{ #(#(0, 0), task.wake_up(0, tick)) }},
    fn(m: #(Int, Int)) {{ button.new(int.to_string(m.1)) |> button.on_press(Click) |> button.done }},
    fn(m, msg) {{
      case msg {{
        Tick -> #(#(m.0 + 1, m.1), task.wake_up(0, tick))
        Click -> #(#(m.0, m.1 + 1), task.none())
      }}
    }})
  |> app.stop_when(fn(m) {{ m.1 == 3 || m.0 >= 1000 }})
  |> app.run
  verify()
}}
"#
        ),
        &format!(
            r#"{PAGE}
click(1);
click(1);
click(1);
function on_wait(ms) {{ if (ms > 0) throw new Error("waited " + ms); }}
export function verify() {{
    if (!log.at(-1).includes(">3<")) throw new Error(log.at(-1));
}}
"#
        ),
    );
}
