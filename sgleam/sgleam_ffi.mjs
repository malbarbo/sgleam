import { isEqual, List } from '../gleam.mjs';
import { inspect } from '../gleam/string.mjs';
import { rectangle, ellipse, line, crop, beside, text, empty, to_svg } from '../sgleam/image.mjs';

export function try_main(main, input_kind, show_output) {
    try {
        let r;
        if (input_kind === "SmainStdin") {
            r = main(read_lines().join("\n"));
        } else if (input_kind === "SmainStdinLines") {
            r = main(List.fromArray(read_lines()));
        } else {
            r = main();
        }
        if (show_output) {
            if (typeof r === "string") {
                console.log(r);
            } else if (r !== undefined && r !== null) {
                console.log(inspect(r));
            }
        }
    } catch (err) {
        show_error(err);
    }
}

function read_lines() {
    let r = [];
    while (true) {
        let line = sgleam.getline();
        if (line == null) {
            return r;
        }
        r.push(line)
    }
}

export function run_tests(modules) {
    globalThis.successes = 0;
    globalThis.failures = 0;
    globalThis.errors = 0;
    console.log('Running tests...');
    for (let i = 0; i < modules.length; i++) {
        const module = modules[i]
        for (const fname of Object.keys(module)) {
            if (!fname.endsWith('_examples')) {
                continue;
            }

            try {
                module[fname]();
            } catch (err) {
                console.log("sgleam internal error: please create a bug report.");
                throw err;
            }
        }
    }

    const { successes, failures, errors } = globalThis;
    const total = successes + failures + errors;
    console.log(`${total} tests, ${successes} success(es), ${failures} failure(s) and ${errors} error(s).`);
}

export function repl_save(value) {
    if (!globalThis.repl_vars) {
        globalThis.repl_vars = []
    }
    globalThis.repl_vars.push(value);
    return value;
}

export function repl_load(index) {
    return globalThis.repl_vars[index]
}

export function repl_print(value) {
    if (value && (value.constructor == rectangle(0.0, 0.0).constructor ||
        value.constructor == ellipse(0.0, 0.0).constructor ||
        value.constructor == line(0.0, 0.0).constructor || // Polygon
        value.constructor == beside(empty, empty).constructor || // Combinatin
        value.constructor == text("", 0.0).constructor ||
        value.constructor == crop(empty).constructor)) {
        if (sgleam.draw_svg) {
            sgleam.draw_svg(`${to_svg(value)}`);
        } else {
            console.log("Image");
        }
    } else {
        console.log(`${inspect(value)}`);
    }
}

export function show_svg(svg) {
    if (sgleam.draw_svg) {
        sgleam.draw_svg(svg);
    } else {
        console.log("Image");
    }
}

export function check_equal(a, b, path, function_name, line_number) {
    try {
        const a_ = a();
        const b_ = b();
        if (isEqual(a_, b_)) {
            globalThis.successes += 1;
            return true;
        } else {
            show_check_failure(a_, b_, null, path, function_name, line_number)
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
    check_equal(a, () => true, path, function_name, line_number)
}

export function check_false(a, path, function_name, line_number) {
    check_equal(a, () => false, path, function_name, line_number)
}

export function check_approx(a, b, tolerance, path, function_name, line_number) {
    try {
        const a_ = a();
        const b_ = b();
        const tolerance_ = tolerance();
        if (Math.abs(a_ - b_) <= tolerance_) {
            globalThis.successes += 1;
            return true;
        } else {
            show_check_failure(a_, b_, tolerance_, path, function_name, line_number)
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
    const space = (tolerance !== null) ? ' ' : '';
    // remove src/
    const file = path.slice(4)
    console.log(`Failure at ${location(file, function_name, line_number)}`);
    console.log(`  Actual  ${space}: ${inspect(a)}`);
    console.log(`  Expected${space}: ${inspect(b)}`);
    if (tolerance !== null) {
        console.log(`  Tolerance: ${inspect(tolerance)}`);
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

function location(file, fname, line_number) {
    if (fname !== '') {
        return `${file} (${fname}:${line_number})`;
    } else {
        return `${file}`;
    }
}

function show_error(err) {
    if (err.gleam_error) {
        // remove src/
        const file = err.file.slice(4)
        console.log(`Error at ${location(file, err.fn, err.line)}`);
        console.log(`  ${err.message}`);
        for (const k in err) {
            if (!['message', 'gleam_error', 'module', 'file', 'line', 'function', 'fn'].includes(k)) {
                console.log(`  ${k}: ${err[k]}`);
            }
        }
    } else if (err.message == 'stack overflow') {
        let stack = err.stack.split('\n');
        stack = stack.slice(-20, -3).reverse();
        stack.push('    ...');
        console.log('Stack overflow')
        for (const f of stack) {
            console.log(`${f.slice(2)}`);
        }
    } else {
        console.log(`${err}`);
    }
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

let clipid = 0;

export function next_clip_id() {
    return clipid++;
}

export function sleep(ms) {
    // spend time on the interpreter so check_interrupt is called
    let msn = Number(ms);
    let start = Date.now();
    while (msn > (Date.now() - start)) {
        sgleam.sleep(1);
    }
}

export function get_key_event() {
    if (sgleam.get_key_event) {
        return List.fromArray(sgleam.get_key_event());
    } else {
        return List.fromArray([]);
    }
}

export function text_width(text, font, size) {
    return sgleam.text_width(text, font, size)
}

export function text_height(text, font, size) {
    return sgleam.text_height(text, font, size)
}