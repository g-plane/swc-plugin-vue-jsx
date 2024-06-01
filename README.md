# SWC Plugin for Vue JSX

SWC plugin for transforming Vue JSX, mostly ported from [official Babel plugin](https://github.com/vuejs/babel-plugin-jsx).
It only supports Vue.js v3 or newer.

## Installation

```
npm i -D swc-plugin-vue-jsx
```

## Configuration

You can configure your `.swcrc` file.

### Minimum Config

```json
{
  "jsc": {
    "parser": {
      "syntax": "ecmascript",
      "jsx": true
    },
    "experimental": {
      "plugins": [["swc-plugin-vue-jsx", {}]]
    }
  }
}
```

### Options

These options of official Babel plugin are supported:

- `transformOn`
- `optimize`
- `mergeProps`
- `enableObjectSlots`
- `pragma`
- `resolveType` (Types imported from other modules aren't supported.)

For details, please refer to official documentation.

The `isCustomElement` can't be supported directly, because SWC config only allows JSON,
so we introduce the `customElementPatterns` option instead.

It accepts an array of strings which represent regex.

For example:

```json
[
  "swc-plugin-vue-jsx",
  {
    "customElementPatterns": ["^i-"]
  }
]
```

All HTML tags which match the pattern `^i-` will be treated as custom elements.

### Advanced Config Example

```json
{
  "jsc": {
    "parser": {
      "syntax": "ecmascript",
      "jsx": true
    },
    "experimental": {
      "plugins": [
        [
          "swc-plugin-vue-jsx",
          {
            "transformOn": true,
            "optimize": true
          }
        ]
      ]
    }
  }
}
```

## License

MIT License

Copyright (c) 2022-present Pig Fang
