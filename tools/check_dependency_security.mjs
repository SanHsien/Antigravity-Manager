import assert from 'node:assert/strict';
import katex from 'katex';
import selectorParser from 'postcss-selector-parser';

assert.match(katex.renderToString('x^2 + y^2'), /class="katex"/);
assert.match(
  katex.renderToString(String.raw`\href{https://example.com}{link}`, { trust: true }),
  /href="https:\/\/example.com"/,
);

// A polluted prototype must not turn the renderer's default trust option on.
const originalTrust = Object.getOwnPropertyDescriptor(Object.prototype, 'trust');
try {
  Object.defineProperty(Object.prototype, 'trust', {
    value: true, configurable: true, writable: true,
  });
  const html = katex.renderToString(String.raw`\href{javascript:alert(1)}{link}`);
  assert.doesNotMatch(html, /href=/);
} finally {
  if (originalTrust) Object.defineProperty(Object.prototype, 'trust', originalTrust);
  else delete Object.prototype.trust;
}

for (const selector of ['.a:hover > #b', ':is(.a, .b)', '[data-value="a b"]']) {
  assert.equal(selectorParser().processSync(selector), selector);
}
const flatSelector = '.a'.repeat(20_000);
assert.equal(selectorParser().processSync(flatSelector), flatSelector);
console.log('SECURITY DEPENDENCY REGRESSIONS GREEN');
