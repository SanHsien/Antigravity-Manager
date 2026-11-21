import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';

// Execute the actual component with only its external effects replaced.
const source = readFileSync(new URL('../src/components/common/ThemeManager.tsx', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX },
}).outputText;

async function mount({ tauri = true, rejectUnminimize = false } = {}) {
    const calls = [];
    const timers = [];
    const effects = [];
    const win = {
        show: async () => calls.push('show'),
        unminimize: async () => {
            calls.push('unminimize');
            if (rejectUnminimize) throw new Error('window cannot be unminimized');
        },
        setFocus: async () => calls.push('focus'),
    };
    const mocks = {
        react: { useEffect: effect => effects.push(effect) },
        '../../stores/useConfigStore': {
            useConfigStore: () => ({ config: null, loadConfig: async () => calls.push('config') }),
        },
        '@tauri-apps/api/window': { getCurrentWindow: () => win },
        '../../utils/env': { isLinux: () => false },
    };
    const exports = {};
    vm.runInNewContext(compiled, {
        exports,
        require: name => {
            assert.ok(name in mocks, `Unexpected dependency: ${name}`);
            return mocks[name];
        },
        window: tauri ? { __TAURI_INTERNALS__: {} } : {},
        setTimeout: (callback, delay) => timers.push({ callback, delay }),
    });
    exports.default();
    effects[0]();
    await new Promise(resolve => setImmediate(resolve));
    assert.deepEqual(calls, ['config'], 'Keep the window hidden until the render delay');
    assert.equal(timers.length, 1);
    assert.ok(timers[0].delay > 0);
    await timers[0].callback();
    return calls;
}

test('restores and focuses the window after showing the rendered content', async () => {
    assert.deepEqual(await mount(), ['config', 'show', 'unminimize', 'focus']);
});

test('still focuses when unminimize is unavailable', async () => {
    assert.deepEqual(await mount({ rejectUnminimize: true }), ['config', 'show', 'unminimize', 'focus']);
});

test('Web mode does not call native window APIs', async () => {
    assert.deepEqual(await mount({ tauri: false }), ['config']);
});

// Upstream #3993 (commit 9bc35068): on Windows, restoring the main window's saved
// position inside a UI-thread callback re-enters Tauri's non-reentrant RwLocks
// through SetWindowPos -> WM_WINDOWPOSCHANGED and freezes the app. The Rust side
// cannot be exercised here, so these checks pin the source-level property: the
// window-state plugin never restores `main` during build(), and every Windows path
// that rebuilds or restores the main window goes through `defer_on_main_thread`.
const rustSource = path => readFileSync(new URL(`../src-tauri/src/${path}`, import.meta.url), 'utf8');

// Strip `#[cfg(not(target_os = "windows"))] { ... }` blocks so a check only sees
// code that is compiled on Windows.
function windowsOnly(source) {
    const marker = '#[cfg(not(target_os = "windows"))]';
    let out = '';
    let i = 0;
    while (true) {
        const at = source.indexOf(marker, i);
        if (at < 0) return out + source.slice(i);
        out += source.slice(i, at);
        const open = source.indexOf('{', at + marker.length);
        let depth = 0;
        let j = open;
        for (; j < source.length; j++) {
            if (source[j] === '{') depth++;
            else if (source[j] === '}' && --depth === 0) break;
        }
        i = j + 1;
    }
}

function callbackBody(source, anchor) {
    const at = source.indexOf(anchor);
    assert.ok(at >= 0, `missing anchor ${anchor}`);
    const open = source.indexOf('{', at);
    let depth = 0;
    for (let j = open; j < source.length; j++) {
        if (source[j] === '{') depth++;
        else if (source[j] === '}' && --depth === 0) return source.slice(open, j + 1);
    }
    throw new Error(`unbalanced braces after ${anchor}`);
}

test('window-state plugin skips the initial restore of main (#3993)', () => {
    assert.match(rustSource('lib.rs'), /\.skip_initial_state\("main"\)/);
});

test('startup restores main after build, deferred on Windows (#3993)', () => {
    const lib = windowsOnly(rustSource('lib.rs'));
    const setup = callbackBody(lib, '.setup(|app|');
    assert.match(setup, /defer_on_main_thread\([^]*?restore_state\(/);
});

test('rebuilt main window restores its state off the build callback on Windows (#3993)', () => {
    const lightweight = windowsOnly(rustSource('modules/lightweight.rs'));
    const ensure = callbackBody(lightweight, 'pub fn ensure_main_window');
    assert.doesNotMatch(ensure.replace(/defer_on_main_thread\([^]*?\}\);/g, ''), /restore_state\(/);
    assert.match(ensure, /defer_on_main_thread\([^]*?restore_state\(/);
    assert.match(lightweight, /fn defer_on_main_thread<F>[^]*?std::thread::Builder::new\(\)[^]*?run_on_main_thread/);
});

test('tray and single-instance callbacks do not rebuild main inline on Windows (#3993)', () => {
    const lib = windowsOnly(rustSource('lib.rs'));
    const single = callbackBody(lib, 'tauri_plugin_single_instance::init(');
    const tray = windowsOnly(rustSource('modules/tray.rs'));
    const bodies = {
        'single-instance': single,
        'tray menu show': callbackBody(tray, '"show" =>'),
        'tray left click': callbackBody(tray, '.on_tray_icon_event('),
    };
    for (const [name, body] of Object.entries(bodies)) {
        assert.match(body, /exit_lightweight_mode/, `${name} should still show the main window`);
        const inline = body.replace(/defer_on_main_thread\([^]*?\}\);/g, '');
        assert.doesNotMatch(inline, /exit_lightweight_mode/, `${name} calls exit_lightweight_mode inline`);
    }
});
