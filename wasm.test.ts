import { type Options, transform } from '@swc/core'
import * as path from 'path'
import * as url from 'url'
import { expect, test } from 'vitest'

const options: Options = {
  jsc: {
    parser: {
      syntax: 'ecmascript',
      jsx: true,
    },
    experimental: {
      plugins: [[
        path.join(
          path.dirname(url.fileURLToPath(import.meta.url)),
          'swc_plugin_vue_jsx.wasm'
        ),
        {},
      ]],
    },
  },
}

test('transform basic JSX', async () => {
  const code = `<div></div>`
  const output = await transform(code, options)
  expect(output.code).toMatchInlineSnapshot(`
    "import { createVNode as _createVNode } from \\"vue\\";
    _createVNode(\\"div\\", null, null);
    "
  `)
})

test('issue #4', async () => {
  const code = `
    let a = {};
    a.b = 1;
    let b = a?.b;
  `
  const output = await transform(code, options)
  expect(output.code).toMatchInlineSnapshot(`
    "var a = {};
    a.b = 1;
    var b = a === null || a === void 0 ? void 0 : a.b;
    "
  `)
})
