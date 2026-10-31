import { Error as GleamError, isEqual, List, Ok } from "../gleam.mjs";
import { inspect } from "../gleam/string.mjs";
import {
  black,
  none as color_none,
  to_source as color_to_source,
} from "../sgleam/color.mjs";
import {
  none as style_none,
  to_source as style_to_source,
} from "../sgleam/style.mjs";
import {
  heightf,
  same,
  to_repl_svg,
  variants,
  widthf,
} from "../sgleam/image.mjs";
import {
  animation_variant,
  run as run_world,
  run_animation,
  variant as world_variant,
} from "../sgleam/world.mjs";
import {
  run as run_mworld,
  variant as mworld_variant,
} from "../sgleam/mworld.mjs";
import { from_host as env_from_host } from "../sgleam/env.mjs";

// What an image prints when no display can show it.
const IMAGE_TEXT = "Image";

// Where a place in a module the repl wrote falls in the input behind it. The
// file is one the user never opened, at a line that counts the imports and
// bindings the repl put above theirs. Nothing for a line the repl wrote.
function repl_location(file, line) {
  const input_line = globalThis.repl_files?.get(file)?.[line] ?? 0;
  return input_line === 0 ? undefined : `${sgleam.repl_path}:${input_line}`;
}

// The path the user gave, from the path of its copy.
function user_path(file) {
  const prefix = sgleam.source_prefix;
  return file.startsWith(prefix) ? file.slice(prefix.length) : file;
}

// What `echo` prints in front of the value: the path of another file, or the
// input behind a module of the repl's.
globalThis.gleam_echo_location = (file, line) =>
  globalThis.repl_files?.has(file)
    ? repl_location(file, line) ?? sgleam.repl_path
    : `${user_path(file)}:${line}`;

export function try_main(main, input_kind, show_output, repl_files) {
  if (repl_files.length !== 0) {
    if (!globalThis.repl_files) {
      globalThis.repl_files = new Map();
    }
    // The old entry stays, as an fn from an earlier input still raises
    // there.
    for (const [file, lines] of repl_files) {
      globalThis.repl_files.set(file, lines);
    }
  }
  try {
    const r = main(...(main_arguments[input_kind]?.() ?? []));
    if (show_output && !show_value(r)) {
      if (typeof r === "string") {
        console.log(r);
      } else if (r !== undefined && r !== null) {
        console.log(inspect(r));
      }
    }
  } catch (err) {
    // The footer of a display in the terminal has no room for an error.
    close_display();
    show_error(err);
    throw err;
  }
}

// The arguments of each kind of `smain` that takes some, by the kind in
// `SMAIN_SIGNATURES`.
const main_arguments = {
  SmainStdin: () => [read_lines().join("\n")],
  SmainStdinLines: () => [List.fromArray(read_lines())],
  SmainEnv: () => [env_from_host()],
};

function read_lines() {
  let r = [];
  while (true) {
    let line = sgleam.getline();
    if (line == null) {
      return r;
    }
    r.push(line);
  }
}

// The test being run, or null outside one. A check runs inside the user's
// code, which carries nothing of the run.
let running = null;

function print(line) {
  console.log(running ? `    ${line}` : line);
}

// Whether every example passed.
export function run_tests(modules) {
  const tests = [];
  for (const [file, module] of modules) {
    for (const fname of Object.keys(module)) {
      if (fname.endsWith("_examples")) {
        tests.push({ file, fn: fname, run: module[fname] });
      }
    }
  }
  // No examples, no report.
  if (tests.length === 0) {
    return true;
  }

  globalThis.successes = 0;
  globalThis.failures = 0;
  globalThis.errors = 0;
  // This prints before the test runs, so a test that hangs still says
  // where.
  console.log("Running tests...");
  for (const test of tests) {
    console.log(`  ${test.file}/${test.fn}`);
    running = test;
    try {
      test.run();
    } catch (err) {
      // A `panic`, an `assert` or a `let assert` in the body of the test
      // raises here, where no check has counted it.
      show_error(err);
      globalThis.errors += 1;
    } finally {
      running = null;
    }
  }

  const { successes, failures, errors } = globalThis;
  const total = successes + failures + errors;
  console.log(
    `${plural(total, "test", "tests")}, ` +
      `${plural(successes, "success", "successes")}, ` +
      `${plural(failures, "failure", "failures")} and ` +
      `${plural(errors, "error", "errors")}.`,
  );
  return failures + errors === 0;
}

function plural(n, one, many) {
  return `${n} ${n === 1 ? one : many}`;
}

// What the repl binds. Gleam has no module level value, so a value is a
// function that runs once. The key is the module that holds the input — a name
// nothing reuses — so the run of that input is the one that fills it, and a
// later read never runs the expression again.
export function repl_memo(key, thunk) {
  if (!globalThis.repl_vars) {
    globalThis.repl_vars = Object.create(null);
  }
  const vars = globalThis.repl_vars;
  if (!(key in vars)) {
    vars[key] = thunk();
  }
  return vars[key];
}

let image_classes;

function is_image(value) {
  image_classes ??= new Set(variants().toArray().map((v) => v.constructor));
  return value != null && image_classes.has(value.constructor);
}

let world_class;

function is_world(value) {
  world_class ??= world_variant().constructor;
  return value != null && value.constructor === world_class;
}

let mworld_class;

function is_mworld(value) {
  mworld_class ??= mworld_variant().constructor;
  return value != null && value.constructor === mworld_class;
}

let animation_class;

function is_animation(value) {
  animation_class ??= animation_variant().constructor;
  return value != null && value.constructor === animation_class;
}

// The values that show other than as their text, each with how it shows. A
// value with a short text takes the screen until it ends, so a let in the
// repl prints the text and binds the value without running it. Its fields
// are the functions of the library, so inspect and echo print the text too.
const shows = [
  [is_image, show_repl_image],
  [is_world, run_world, "//world"],
  [is_mworld, run_mworld, "//mworld"],
  [is_animation, run_animation, "//animation"],
];

let color_classes;

function is_color(value) {
  color_classes ??= new Set([color_none.constructor, black.constructor]);
  return value != null && color_classes.has(value.constructor);
}

function is_style(value) {
  return value != null && value.constructor === style_none.constructor;
}

// The values whose text is an expression the user could write, in place of
// the constructors of an opaque type.
const sources = [
  [is_color, color_to_source],
  [is_style, style_to_source],
];

globalThis.gleam_inspect = (value) =>
  shows.find(([is]) => is(value))?.[2] ??
    sources.find(([is]) => is(value))?.[1](value);

// Returns `true` if `value` has a kind in `shows`, which shows it, `false`
// otherwise. The repl and the result of `smain` show a value this way.
function show_value(value) {
  const show = shows.find(([is]) => is(value))?.[1];
  show?.(value);
  return show !== undefined;
}

export function repl_print(value) {
  if (!show_value(value)) {
    console.log(`${inspect(value)}`);
  }
  return value;
}

export function repl_print_binding(value) {
  const [, show, text] = shows.find(([is]) => is(value)) ?? [];
  if (text !== undefined) {
    console.log(text);
  } else if (show) {
    show(value);
  } else {
    console.log(`${inspect(value)}`);
  }
  return value;
}

function show_repl_image(image) {
  if (sgleam.draw_svg) {
    sgleam.draw_svg(`${to_repl_svg(image)}`);
  } else if (!print_repl_image(image)) {
    console.log(IMAGE_TEXT);
  }
}

// The canvas of to_repl_svg. The width and height of an image round as
// Gleam's float.round does, and Int may be a BigInt, so the sizes start as
// floats.
function print_repl_image(image) {
  const width = Math.round(widthf(image)) + 1;
  const height = Math.round(heightf(image)) + 1;
  return sgleam.print_image?.(image, width, height) ?? false;
}

// The canvas has the size of the image, as in to_svg. Only the repl adds a
// column and a row, for an outline on the right or bottom edge.
export function show_image(image) {
  const width = Math.round(widthf(image));
  const height = Math.round(heightf(image));
  if (!sgleam.show_image(image, width, height)) {
    console.log(IMAGE_TEXT);
  }
}

export function open_mworld(min_players, max_players) {
  return List.fromArray(
    sgleam.open_mworld(Number(min_players), Number(max_players)),
  );
}

export function show_player_image(player, image) {
  const width = Math.round(widthf(image));
  const height = Math.round(heightf(image));
  if (!sgleam.show_player_image(Number(player), image, width, height)) {
    console.log(IMAGE_TEXT);
  }
}

export function close_display() {
  sgleam.close_display();
}

export function check_equal(a, b, path, function_name, line_number) {
  try {
    const a_ = a();
    const b_ = b();
    if (isEqual(a_, b_)) {
      globalThis.successes += 1;
      return true;
    } else {
      show_check_failure(a_, b_, null, path, function_name, line_number);
      globalThis.failures += 1;
      return false;
    }
  } catch (err) {
    show_check_error(err, path, function_name, line_number);
    globalThis.errors += 1;
    return false;
  }
}

export function check_same(a, b, path, function_name, line_number) {
  try {
    const a_ = a();
    const b_ = b();
    if (same(a_, b_)) {
      globalThis.successes += 1;
      return true;
    } else {
      show_check_failure(a_, b_, null, path, function_name, line_number);
      globalThis.failures += 1;
      return false;
    }
  } catch (err) {
    show_check_error(err, path, function_name, line_number);
    globalThis.errors += 1;
    return false;
  }
}

export function check_true(a, path, function_name, line_number) {
  return check_equal(a, () => true, path, function_name, line_number);
}

export function check_false(a, path, function_name, line_number) {
  return check_equal(a, () => false, path, function_name, line_number);
}

export function check_approx(
  a,
  b,
  tolerance,
  path,
  function_name,
  line_number,
) {
  try {
    const a_ = a();
    const b_ = b();
    const tolerance_ = tolerance();
    if (Math.abs(a_ - b_) <= tolerance_) {
      globalThis.successes += 1;
      return true;
    } else {
      show_check_failure(
        a_,
        b_,
        tolerance_,
        path,
        function_name,
        line_number,
      );
      globalThis.failures += 1;
      return false;
    }
  } catch (err) {
    show_check_error(err, path, function_name, line_number);
    globalThis.errors += 1;
    return false;
  }
}

function show_check_failure(a, b, tolerance, path, function_name, line_number) {
  const space = (tolerance !== null) ? " " : "";
  const file = user_path(path);
  print(`Failure at ${location(file, function_name, line_number)}`);
  show_check_value(`  Actual  ${space}:`, a);
  show_check_value(`  Expected${space}:`, b);
  if (tolerance !== null) {
    print(`  Tolerance: ${inspect(tolerance)}`);
  }
}

// An image draws under its label, and goes out as text where nothing draws.
function show_check_value(label, value) {
  if (is_image(value) && (sgleam.draw_svg || sgleam.prints_images?.())) {
    print(label);
    show_repl_image(value);
  } else {
    print(`${label} ${inspect(value)}`);
  }
}

function show_check_error(err, path, function_name, line_number) {
  if (!err.gleam_error) {
    err.gleam_error = true;
    err.file = path;
    err.fn = function_name;
    err.line = line_number;
  }
  show_error(err);
}

// Only what the name of the running test did not already say.
function location(file, fname, line_number) {
  if (fname === "") {
    return file;
  }
  if (!running || file !== running.file) {
    return `${file} (${fname}:${line_number})`;
  }
  return fname === running.fn
    ? `line ${line_number}`
    : `${fname}:${line_number}`;
}

// The values the check evaluated, which are what explains the failure. The
// remaining fields are byte offsets into the generated module, which say
// nothing to the user.
function error_details(err) {
  if (err.gleam_error === "let_assert") {
    return [["value", inspect(err.value)]];
  }
  if (err.gleam_error !== "assert") {
    return [];
  }
  switch (err.kind) {
    case "binary_operator":
      return [
        ["operator", err.operator],
        ["left", asserted_value(err.left)],
        ["right", asserted_value(err.right)],
      ];
    case "function_call":
      return err.arguments.map((
        argument,
        i,
      ) => [`argument ${i + 1}`, asserted_value(argument)]);
    // The value of a plain expression is always `False`, and a literal is
    // already in the message.
    default:
      return [];
  }
}

function asserted_value(expression) {
  // `&&` and `||` can fail before evaluating their right side.
  if (expression.kind === "unevaluated") {
    return "not evaluated";
  }
  return inspect(expression.value);
}

function show_error(err) {
  if (err.gleam_error) {
    const at = repl_location(err.file, err.line);
    if (at) {
      print(`Error at ${at}`);
      print(`  ${err.message}`);
    } else if (globalThis.repl_files?.has(err.file)) {
      // The repl wrote that line, so there is nowhere to point at.
      print(`Error: ${err.message}`);
    } else {
      const file = user_path(err.file);
      print(`Error at ${location(file, err.fn, err.line)}`);
      print(`  ${err.message}`);
    }
    for (const [name, value] of error_details(err)) {
      print(`  ${name}: ${value}`);
    }
  } else if (err.message == "Maximum call stack size exceeded") {
    const stack = err.stack.split("\n").filter((f) => f.trim() !== "");
    print("Stack overflow");
    for (const f of first_cycle(stack.map(source_frame))) {
      print(`  ${f}`);
    }
    print("  ...");
  } else {
    print(`${err}`);
  }
}

// The frames up to the end of the first round of the cycle that repeats until
// the end of the trace. A trace that starts in the cycle starts at its
// smallest rotation. A trace with frames before the cycle keeps the caller of
// those frames under them. Returns every frame if no cycle repeats twice.
function first_cycle(frames) {
  for (let start = 0; start < frames.length; start++) {
    for (let period = 1; start + 2 * period <= frames.length; period++) {
      if (repeats(frames, start, period)) {
        return start === 0
          ? smallest_rotation(frames.slice(0, period))
          : frames.slice(0, start + period);
      }
    }
  }
  return frames;
}

// The rotation of `round` that comes first in the order of the text. The
// stack runs out at a different frame on each platform, and this keeps the
// trace the same on all of them.
function smallest_rotation(round) {
  let smallest = round;
  for (let i = 1; i < round.length; i++) {
    const rotation = [...round.slice(i), ...round.slice(0, i)];
    if (rotation.join("\n") < smallest.join("\n")) {
      smallest = rotation;
    }
  }
  return smallest;
}

// Returns `true` if each frame from `start` on is equal to the frame `period`
// places after it, `false` otherwise.
function repeats(frames, start, period) {
  for (let i = start; i + period < frames.length; i++) {
    if (frames[i] !== frames[i + period]) {
      return false;
    }
  }
  return true;
}

// A frame of a stack trace with the Gleam file and line in place of the
// generated module. A module without a map keeps only the name of the
// function.
function source_frame(frame) {
  const parts = frame.match(/^\s*at (.+) \((.+):(\d+):(\d+)\)$/);
  if (!parts) {
    return frame.trim();
  }
  const [, fname, path, line, column] = parts;
  const at = sgleam.source_location(path, Number(line), Number(column));
  if (!at) {
    return `at ${fname}`;
  }
  const [file, source_line] = at;
  return `at ${fname} (${globalThis.gleam_echo_location(file, source_line)})`;
}

export function cos_deg(angle) {
  return Math.cos(angle * Math.PI / 180.0);
}

export function sin_deg(angle) {
  return Math.sin(angle * Math.PI / 180.0);
}

export function hypot(a, b) {
  return Math.hypot(a, b);
}

export function cos(angle) {
  return Math.cos(angle);
}

export function sin(angle) {
  return Math.sin(angle);
}

export function atan2(y, x) {
  return Math.atan2(y, x);
}

export function sqrt(x) {
  return Math.sqrt(x);
}

export function float_to_string_6(v) {
  let s = parseFloat(v.toFixed(6)).toString();
  if (s.indexOf(".") < 0) {
    s += ".0";
  }
  return s;
}

export function image_to_svg(img, width, height) {
  return sgleam.image_to_svg(img, width, height);
}

export function path_box(commands) {
  return sgleam.path_box(commands);
}

export function same_pixels(a, b, width, height) {
  return sgleam.same_pixels(a, b, width, height);
}

export function sleep(ms) {
  sgleam.sleep(Number(ms));
}

export function now_ms() {
  return to_int(sgleam.now_ms());
}

function to_int(v) {
  return globalThis.__sgleam_bigint ? BigInt(v) : Number(v);
}

// The run sets `sgleam.args` before it calls `smain`. The repl sets nothing.
export function host_args() {
  return List.fromArray(sgleam.args ?? []);
}

export function host_seed() {
  return to_int(Math.floor(Math.random() * 2 ** 32));
}

// Two steps of mulberry32, for the 53 bits of a float. The state is any Int,
// and only its low 32 bits count.
export function random_step(state) {
  let s = typeof state === "bigint"
    ? Number(BigInt.asUintN(32, state))
    : state >>> 0;
  const next = () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return (t ^ (t >>> 14)) >>> 0;
  };
  const high = next() >>> 5;
  const low = next() >>> 6;
  return [(high * 2 ** 26 + low) / 2 ** 53, to_int(s)];
}

export function wait_event(ms) {
  return List.fromArray(sgleam.wait_event(Number(ms)));
}

export function text_metrics(text, font) {
  return sgleam.text_metrics(text, font);
}

export function load_image(data_uri) {
  const image = sgleam.load_image(data_uri);
  return image ? new Ok(image) : new GleamError(undefined);
}

export function no_image() {
  return undefined;
}
