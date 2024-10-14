import { isEqual } from "./gleam.mjs"
import { inspect } from "./gleam/string.mjs"

export function try_main(main) {
    try {
        main();
    } catch (err) {
        if (err.gleam_error) {
            show_gleam_error(err)
        }
    }
}

export function run_tests(module) {
    globalThis.successes = 0;
    globalThis.failures = 0;
    globalThis.errors = 0;
    console.log("Running tests...");
    for (let fn_name of Object.keys(module)) {
        if (!fn_name.endsWith("_examples")) {
            continue;
        }
        try {
            module[fn_name]();
        } catch (err) {
            show_gleam_error(err);
            globalThis.errors += 1;
        }
    }
    let { successes, failures, errors } = globalThis;
    let total = successes + failures + errors;
    console.log(`${total} tests, ${successes} success(es), ${failures} failure(s) and ${errors} errors.`);
}

export function check_equal(a, b) {
    if (isEqual(a, b)) {
        globalThis.successes += 1;
        return true;
    } else {
        console.log("Failure")
        console.log(`  Actual  : ${inspect(a)}`)
        console.log(`  Expected: ${inspect(b)}`)        
        globalThis.failures += 1;
        return false;
    }
}

export function check_approx(a, b, tolerance) {
    if (Math.abs(a - b) <= tolerance) {
        globalThis.successes += 1;
        return true;
    } else {
        console.log("Failure")
        console.log(`  Actual   : ${inspect(a)}`)
        console.log(`  Expected : ${inspect(b)}`)
        console.log(`  Tolerance: ${inspect(tolerance)}`)
        globalThis.failures += 1;
        return false;
    }
}

export function show_gleam_error(err) {
    console.log(`Runtime error at ${err.module}.${err.fn}:${err.line}.`);
    console.log(`${err.message}`);
    for (let k in err) {
        if (!["message", "gleam_error", "module", "line", "function", "fn"].includes(k)) {
            console.log(`${k}: ${err[k]}`);
        }
    }
}
