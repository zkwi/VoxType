import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const audit = spawnSync(
  process.execPath,
  [process.env.npm_execpath, "audit", "--registry=https://registry.npmjs.org", "--json"],
  { encoding: "utf8" },
);

let report;
try {
  report = JSON.parse(audit.stdout);
} catch {
  console.error(audit.error ?? (audit.stderr || "npm audit returned an invalid report."));
  process.exit(1);
}

if (audit.status === 0 && !report.error && report.metadata?.vulnerabilities?.total === 0) {
  console.log("npm audit found no vulnerabilities.");
  process.exit(0);
}

const vulnerabilities = Object.values(report.vulnerabilities ?? {});
const advisories = vulnerabilities.flatMap(({ via }) => via.filter((entry) => typeof entry === "object"));
const lock = JSON.parse(readFileSync(new URL("../package-lock.json", import.meta.url), "utf8"));
// vite-plugin-svelte merges trusted build config only. No patched deepmerge
// release is available; accept this exact advisory only for dev-only 4.3.1.
const deepmergeAdvisory = "https://github.com/advisories/GHSA-gcqx-w48m-w2h4";
const accepted = audit.status === 1 && !report.error && advisories.length > 0 &&
  advisories.every((entry) => entry.name === "deepmerge" && entry.url === deepmergeAdvisory) &&
  vulnerabilities.every(({ severity, nodes }) =>
    severity === "low" && nodes.every((node) => lock.packages[node]?.dev === true),
  ) &&
  vulnerabilities.find(({ name }) => name === "deepmerge")?.nodes.every(
    (node) => lock.packages[node]?.version === "4.3.1",
  );

if (accepted) {
  console.warn(`npm audit accepted one dev-only advisory: ${deepmergeAdvisory}`);
} else {
  console.error(audit.stdout);
  process.exitCode = 1;
}
