import { isEqual, List } from './gleam.mjs';
import { inspect } from './gleam/string.mjs';

export function try_main(main, input_kind, show_output) {
    try {
        let r;
        if (input_kind === "Stdin") {
            r = main(read_lines().join("\n"));
        } else if (input_kind === "StdinLines") {
            r = main(List.fromArray(read_lines()));
        } else {
            r = main();
        }
        if (show_output) {
            if (typeof r == "string") {
                console.log(r);
            } else if (r) {
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
        let line = console.getline();
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

export function check_equal(a, b, module_name, function_name, line_number) {
    try {
        const a_ = a();
        const b_ = b();
        if (isEqual(a_, b_)) {
            globalThis.successes += 1;
            return true;
        } else {
            show_check_failure(a_, b_, null, module_name, function_name, line_number)
            globalThis.failures += 1;
            return false;
        }
    } catch (err) {
        show_check_error(err, module_name, function_name, line_number);
        globalThis.errors += 1;
        return false;
    }
}

export function check_true(a, module_name, function_name, line_number) {
    check_equal(a, () => true, module_name, function_name, line_number)
}

export function check_false(a, module_name, function_name, line_number) {
    check_equal(a, () => false, module_name, function_name, line_number)
}

export function check_approx(a, b, tolerance, module_name, function_name, line_number) {
    try {
        const a_ = a();
        const b_ = b();
        const tolerance_ = tolerance();
        if (Math.abs(a_ - b_) <= tolerance_) {
            globalThis.successes += 1;
            return true;
        } else {
            show_check_failure(a_, b_, tolerance_, module_name, function_name, line_number)
            globalThis.failures += 1;
            return false;
        }
    } catch (err) {
        show_check_error(err, module_name, function_name, line_number);
        globalThis.errors += 1;
        return false;
    }
}

function show_check_failure(a, b, tolerance, module_name, function_name, line_number) {
    let space = (tolerance !== null) ? ' ' : '';
    console.log(`Failure at ${location(module_name, function_name, line_number)}`);
    console.log(`  Actual  ${space}: ${inspect(a)}`);
    console.log(`  Expected${space}: ${inspect(b)}`);
    if (tolerance !== null) {
        console.log(`  Tolerance: ${inspect(tolerance)}`);
    }
}

function show_check_error(err, module_name, function_name, line_number) {
    if (!err.gleam_error) {
        err.gleam_error = true;
        err.module = module_name;
        err.fn = function_name;
        err.line = line_number;
    }
    show_error(err);
}

function location(module_name, function_name, line_number) {
    let fname = (function_name !== '') ? '.' + function_name : '';
    return `${module_name}${fname}:${line_number}`
}

function show_error(err) {
    if (err.gleam_error) {
        console.log(`Error at ${location(err.module, err.fn, err.line)}`);
        console.log(`  ${err.message}`);
        for (const k in err) {
            if (!['message', 'gleam_error', 'module', 'line', 'function', 'fn'].includes(k)) {
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
