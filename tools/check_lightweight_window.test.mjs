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
