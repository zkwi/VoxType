#!/usr/bin/env node
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(new URL("./check-governance.mjs", import.meta.url));

function runGovernance(cwd) {
  return spawnSync(process.execPath, [scriptPath], {
    cwd,
    encoding: "utf8",
  });
}

function writeFile(filePath, content) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content, "utf8");
}

function withProject(callback) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "voxtype-governance-"));
  try {
    createValidProject(dir);
    callback(dir);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

function createValidProject(dir) {
  writeFile(path.join(dir, "package.json"), JSON.stringify({ version: "1.2.3" }, null, 2));
  writeFile(
    path.join(dir, "package-lock.json"),
    JSON.stringify(
      {
        name: "voxtype-desktop",
        version: "1.2.3",
        lockfileVersion: 3,
        packages: {
          "": {
            name: "voxtype-desktop",
            version: "1.2.3",
          },
        },
      },
      null,
      2,
    ),
  );
  writeFile(path.join(dir, "CHANGELOG.md"), "# Changelog\n\n## [1.2.3]\n");
  writeFile(
    path.join(dir, "src-tauri", "Cargo.toml"),
    '[package]\nname = "voxtype-desktop"\nversion = "1.2.3"\n\n[dependencies]\n',
  );
  writeFile(
    path.join(dir, "src-tauri", "Cargo.lock"),
    'version = 4\n\n[[package]]\nname = "voxtype-desktop"\nversion = "1.2.3"\n',
  );
  writeFile(
    path.join(dir, "src-tauri", "tauri.conf.json"),
    JSON.stringify({ version: "1.2.3" }, null, 2),
  );
  writeI18nFiles(dir);

  writeFile(
    path.join(dir, "README.md"),
    [
      "# Test",
      "",
      "[Setup](https://github.com/zkwi/VoxType/wiki/Setup-Guide)",
      "",
      '<img src="https://raw.githubusercontent.com/zkwi/VoxType/main/screenshots/home.png" alt="home">',
      "",
    ].join("\n"),
  );
  writeFile(path.join(dir, "screenshots", "home.png"), "not a real png");
  writeFile(
    path.join(dir, "docs", "audits", "2026-07-13-release-1.2.3-test-audit.md"),
    "# 1.2.3 release audit\n",
  );
  writeFile(
    path.join(dir, "docs", "README.md"),
    "# Docs\n\n[Current release audit](audits/2026-07-13-release-1.2.3-test-audit.md)\n",
  );

  for (const page of [
    "_Sidebar",
    "Home",
    "Setup-Guide",
    "Setup-Guide-English",
    "Feature-Guide",
    "Feature-Guide-English",
    "Troubleshooting",
    "Troubleshooting-English",
  ]) {
    writeFile(path.join(dir, "docs", "wiki", `${page}.md`), `# ${page}\n`);
  }
}

function writeI18nFiles(dir, overrides = {}) {
  const files = {
    "zh-CN.ts": 'export const zhCN = {\n  "appName": "声写",\n  "setup": {\n    "title": "配置",\n    "description": "说明"\n  } as const\n} as const;\n',
    "zh-TW.ts": 'import type { TranslationCopy } from "./types";\nexport const zhTW = {\n  "appName": "聲寫",\n  "setup": {\n    "title": "配置",\n    "description": "說明"\n  }\n} satisfies TranslationCopy;\n',
    "en.ts": 'import type { TranslationCopy } from "./types";\nexport const en = {\n  "appName": "VoxType",\n  "setup": {\n    "title": "Setup",\n    "description": "Description"\n  }\n} satisfies TranslationCopy;\n',
    ...overrides,
  };
  for (const [filename, content] of Object.entries(files)) {
    writeFile(path.join(dir, "src", "lib", "i18n", filename), content);
  }
  // 文案键必须在 i18n 目录之外有引用，否则会被判为无人使用。
  writeFile(path.join(dir, "src", "lib", "app.ts"), 'export const title = t("appName");\n');
}

withProject((dir) => {
  const result = runGovernance(dir);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stdout, /\[governance\] checks passed/);
});

withProject((dir) => {
  writeFile(
    path.join(dir, "src-tauri", "Cargo.lock"),
    'version = 4\n\n[[package]]\nname = "voxtype-desktop"\nversion = "9.9.9"\n',
  );
  const result = runGovernance(dir);
  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(result.stdout, /src-tauri\/Cargo\.lock.*9\.9\.9/);
});

withProject((dir) => {
  fs.rmSync(path.join(dir, "docs", "audits", "2026-07-13-release-1.2.3-test-audit.md"));
  const result = runGovernance(dir);
  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(result.stdout, /missing release audit for version 1\.2\.3/);
});

withProject((dir) => {
  writeFile(path.join(dir, "docs", "README.md"), "# Docs\n");
  const result = runGovernance(dir);
  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(result.stdout, /docs\/README\.md.*release audit.*1\.2\.3/);
});

withProject((dir) => {
  writeFile(path.join(dir, "src-tauri", "tauri.conf.json"), JSON.stringify({ version: "9.9.9" }, null, 2));
  const result = runGovernance(dir);
  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(result.stdout, /version mismatch/);
});

withProject((dir) => {
  const packageLock = JSON.parse(fs.readFileSync(path.join(dir, "package-lock.json"), "utf8"));
  packageLock.packages[""].version = "9.9.9";
  writeFile(path.join(dir, "package-lock.json"), JSON.stringify(packageLock, null, 2));
  const result = runGovernance(dir);
  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(result.stdout, /package-lock\.json packages\[""\]\.version=9\.9\.9/);
});

withProject((dir) => {
  fs.rmSync(path.join(dir, "docs", "wiki", "Setup-Guide.md"));
  const result = runGovernance(dir);
  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(result.stdout, /missing required Wiki mirror Setup-Guide\.md/);
  assert.match(result.stdout, /Wiki link lacks local mirror: Setup-Guide/);
});

withProject((dir) => {
  // 目标文件在仓库里存在，本地链接检查能过；但 Wiki 镜像页推到线上后这个相对链接会失效。
  writeFile(path.join(dir, "docs", "audits", "note.md"), "# note\n");
  writeFile(
    path.join(dir, "docs", "wiki", "Setup-Guide.md"),
    "# Setup-Guide\n\n[note](../audits/note.md)\n\n[home](Home)\n",
  );
  const result = runGovernance(dir);
  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(
    result.stdout,
    /docs\/wiki\/Setup-Guide\.md: relative link leaves the Wiki and breaks online: \.\.\/audits\/note\.md/,
  );
  // 同一 Wiki 内的页面互链不受影响。
  assert.doesNotMatch(result.stdout, /breaks online: Home/);
});

withProject((dir) => {
  writeI18nFiles(dir, {
    "en.ts": 'import type { TranslationCopy } from "./types";\nexport const en = {\n  "appName": "VoxType",\n  "setup": {\n    "title": "Setup"\n  }\n} satisfies TranslationCopy;\n',
  });
  const result = runGovernance(dir);
  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(result.stdout, /i18n keys missing/);
  assert.match(result.stdout, /setup\.description/);
});

withProject((dir) => {
  writeI18nFiles(dir, {
    "en.ts": 'import type { TranslationCopy } from "./types";\nexport const en = {\n  "appName": "VoxType",\n  "setup": {\n    "title": "Setup",\n    "description": "Description"\n  },\n  "extra": "Unexpected"\n} satisfies TranslationCopy;\n',
  });
  const result = runGovernance(dir);
  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(result.stdout, /i18n keys extra/);
  assert.match(result.stdout, /\bextra\b/);
});

withProject((dir) => {
  // 三份语言文件键集一致，但有一条文案在代码里已经没人引用。
  writeI18nFiles(dir, {
    "zh-CN.ts": 'export const zhCN = {\n  "appName": "声写",\n  "orphanLabel": "没人用的文案"\n} as const;\n',
    "zh-TW.ts": 'import type { TranslationCopy } from "./types";\nexport const zhTW = {\n  "appName": "聲寫",\n  "orphanLabel": "沒人用的文案"\n} satisfies TranslationCopy;\n',
    "en.ts": 'import type { TranslationCopy } from "./types";\nexport const en = {\n  "appName": "VoxType",\n  "orphanLabel": "Unused copy"\n} satisfies TranslationCopy;\n',
  });
  const result = runGovernance(dir);
  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(result.stdout, /i18n keys not referenced by any source file: orphanLabel/);
  assert.doesNotMatch(result.stdout, /appName/);
});

withProject((dir) => {
  // 文案键通过标签映射表间接使用时，同样算作已引用。
  writeI18nFiles(dir, {
    "zh-CN.ts": 'export const zhCN = {\n  "appName": "声写",\n  "navHome": "首页"\n} as const;\n',
    "zh-TW.ts": 'import type { TranslationCopy } from "./types";\nexport const zhTW = {\n  "appName": "聲寫",\n  "navHome": "首頁"\n} satisfies TranslationCopy;\n',
    "en.ts": 'import type { TranslationCopy } from "./types";\nexport const en = {\n  "appName": "VoxType",\n  "navHome": "Home"\n} satisfies TranslationCopy;\n',
  });
  writeFile(
    path.join(dir, "src", "lib", "components", "Nav.svelte"),
    "<script>\n  const labels = { Home: 'navHome' };\n</script>\n\n{t(labels.Home)}\n",
  );
  const result = runGovernance(dir);
  assert.equal(result.status, 0, result.stdout + result.stderr);
});

console.log("[test-governance] all checks passed");
