import path from "path";
import { defineConfig } from "@rspack/cli";
import { VueLoaderPlugin } from "vue-loader";
import { rspack } from "@rspack/core";

const swcLoaderConfig = {
  sourceMaps: true,
  jsc: {
    parser: {
      syntax: "typescript",
      tsx: true,
      decorators: true,
    },
    transform: {
      legacyDecorator: true,
      decoratorMetadata: true,
      react: {
        pragma: "h",
        throwIfNamespace: true,
        development: false,
        useBuiltins: false,
      },
    },
    experimental: {
      plugins: [[path.resolve(process.cwd(), "./swc_plugin_vue_jsx.wasm"), {}]],
      // plugins: [['swc-plugin-vue-jsx', {}]] // build fail, is expected
    },
  },
};

const config = defineConfig({
  context: process.cwd(),
  entry: {
    main: "./src/index.ts",
  },
  output: {
    path: path.resolve(process.cwd(), "./dist"),
    filename: "[id].js",
    clean: true, // 构建前清理 dist 目录
  },
  resolve: {
    extensions: [".js", ".jsx", ".ts", ".tsx", ".vue"],
  },
  plugins: [
    new VueLoaderPlugin(),
    new rspack.HtmlRspackPlugin({
      template: "./public/index.html",
    }),
  ],
  module: {
    rules: [
      {
        test: /\.vue$/,
        use: [
          {
            loader: "vue-loader",
            options: {
              // Note, for the majority of features to be available, make sure this option is `true`
              experimentalInlineMatchResource: true,
            },
          },
        ],
      },
      {
        test: /\.(ts|jsx|tsx)$/,
        exclude: [/node_modules/],
        use: {
          loader: "builtin:swc-loader",
          options: swcLoaderConfig,
        },
      },
    ],
  },
  devServer: {
    port: 3000,
    open: true,
  },
});

export default config;
