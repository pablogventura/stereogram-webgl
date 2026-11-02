import eslint from "@eslint/js";
import stylistic from "@stylistic/eslint-plugin";
import checkFile from "eslint-plugin-check-file";
import tseslint from "typescript-eslint";

export default tseslint.config(
    {
        ignores: [
            "docs/**",
            "node_modules/**",
            "**/*generated.*",
        ],
    },
    {
        files: ["src/**/*.ts"],
        extends: [
            eslint.configs.recommended,
            ...tseslint.configs.recommended,
        ],
        plugins: {
            "@stylistic": stylistic,
            "check-file": checkFile,
        },
        rules: {
            "no-console": "off",
            // gl-matrix is loaded as an untyped global script.
            "@typescript-eslint/no-explicit-any": "off",
            "@typescript-eslint/no-unused-vars": [
                "error",
                {
                    "argsIgnorePattern": "^_",
                    "caughtErrorsIgnorePattern": "^_",
                },
            ],
            "no-restricted-syntax": [
                "error",
                {
                    "selector": "ExportDefaultDeclaration",
                    "message": "Use a named export.",
                },
            ],
            "@stylistic/semi": ["error", "always"],
            "@stylistic/indent": ["error", 4, { "SwitchCase": 1 }],
            "check-file/filename-naming-convention": [
                "error",
                {
                    "**/*.ts": "KEBAB_CASE",
                },
            ],
        },
    },
);
