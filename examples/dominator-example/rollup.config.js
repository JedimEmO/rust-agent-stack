import rust from "@wasm-tool/rollup-plugin-rust";
import dev from "rollup-plugin-dev";
import livereload from "rollup-plugin-livereload";
import {terser} from "rollup-plugin-terser";
import copy from 'rollup-plugin-copy'

const is_watch = !!process.env.ROLLUP_WATCH;

export default {
    input: {
        bundle: "Cargo.toml",
    },
    output: {
        dir: "dist/js",
        format: "es",
        sourcemap: true,
    },
    plugins: [
        rust({
            optimize: {release: false},
            extraArgs: {
                cargo: ["--config", "profile.dev.debug=true"],
                wasmBindgen: ["--debug", "--keep-debug"]
            },
        }),
        copy({
            targets: [
                {rename: "index.html", src: 'index.html', dest: 'dist/'}
            ]
        }),
        is_watch && dev({
            dirs: ["dist"],
            port: 8080,
            host: "0.0.0.0",
            proxy: [
                { from: "/api", to: "http://localhost:3000/api" }
            ],
            spa: true,
        }),

        is_watch && livereload("dist"),

        !is_watch && terser(),
    ],
};