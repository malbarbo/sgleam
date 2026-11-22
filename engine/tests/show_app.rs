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
fn an_event_from_an_older_frame_is_dropped() {
    run(
        &format!(
            r#"{IMPORTS}
type Msg {{ Click }}
pub fn main() {{
  setup()
  app.create(fn() {{ #(0, task.none()) }},
    fn(n) {{ button.new(int.to_string(n)) |> button.on_press(Click) |> button.done }},
    fn(n, _) {{ #(n + 1, task.none()) }})
  |> app.stop_when(fn(n) {{ n >= 2 }})
  |> app.run
  verify()
}}
"#
        ),
        &format!(
            r#"{PAGE}
// Two clicks on the first frame, read after the first one drew the second.
click(1);
click(1);
let waits = 0;
function on_wait(ms) {{
    waits++;
    click(frame);
}}
export function verify() {{
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
