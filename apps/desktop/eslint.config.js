import ts from "typescript-eslint";
export default ts.config({ ignores: ["dist"] }, ...ts.configs.recommended, {
  files: ["src/**/*.{ts,tsx}"],
  rules: {
    "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
  },
});
