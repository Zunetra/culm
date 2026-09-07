/* Enforces the org's exact commit format: `<type>: <description>`, one line,
 * with a closed type list. Deliberately not extending
 * @commitlint/config-conventional, which allows types this org does not use. */
export default {
  rules: {
    "type-enum": [
      2,
      "always",
      ["feat", "fix", "refactor", "chore", "test", "docs", "perf", "style"],
    ],
    "type-case": [2, "always", "lower-case"],
    "subject-empty": [2, "never"],
    "header-max-length": [2, "always", 100],
    "body-max-line-length": [0],
  },
};
