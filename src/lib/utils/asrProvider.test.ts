import { describe, expect, it } from "vitest";
import { fallbackConfig } from "$lib/app/defaults";
import { activeAsrConfigFingerprint, hasAsrProviderConfig } from "./asrProvider";

function agentPlanConfig() {
  const config = structuredClone(fallbackConfig);
  Object.assign(config.auth, {
    mode: "agent_plan",
    api_key: "plan-test-key",
    app_key: "",
    access_key: "",
  });
  return config;
}

function consoleApiKeyConfig() {
  const config = structuredClone(fallbackConfig);
  Object.assign(config.auth, {
    mode: "api_key",
    console_api_key: "console-test-key",
    app_key: "",
    access_key: "",
  });
  return config;
}

describe("Doubao authentication modes", () => {
  it("starts a fresh install in speech console API key mode", () => {
    expect(fallbackConfig.auth.mode).toBe("api_key");
    // 新装默认没有密钥，必须判为未配置，首页才会引导去填写。
    expect(hasAsrProviderConfig(fallbackConfig)).toBe(false);
  });

  it("treats a speech console API key as complete Doubao authentication", () => {
    expect(hasAsrProviderConfig(consoleApiKeyConfig())).toBe(true);
  });

  it("accepts any configured resource in speech console API key mode", () => {
    const config = consoleApiKeyConfig();
    config.auth.resource_id = "volc.seedasr.sauc.concurrent";

    expect(hasAsrProviderConfig(config)).toBe(true);
  });

  it("keeps console and Agent Plan keys in separate fields", () => {
    // 两种方式的密钥不通用，切换接入方式不能互相覆盖，也不能互相顶替。
    const config = consoleApiKeyConfig();
    config.auth.api_key = "plan-test-key";
    expect(hasAsrProviderConfig(config)).toBe(true);

    config.auth.console_api_key = "";
    expect(hasAsrProviderConfig(config)).toBe(false);

    config.auth.mode = "agent_plan";
    expect(hasAsrProviderConfig(config)).toBe(true);
  });

  it("treats an Agent Plan API key as complete Doubao authentication", () => {
    expect(hasAsrProviderConfig(agentPlanConfig())).toBe(true);
  });

  it("invalidates the ASR test fingerprint when the authentication mode changes", () => {
    const agentPlan = agentPlanConfig();
    const appAccess = structuredClone(agentPlan);
    Object.assign(appAccess.auth, { mode: "app_access" });

    expect(activeAsrConfigFingerprint(agentPlan)).not.toBe(activeAsrConfigFingerprint(appAccess));
  });

  it("requires the fixed Seed ASR 2.0 resource in Agent Plan mode", () => {
    const config = agentPlanConfig();
    config.auth.resource_id = "volc.seedasr.sauc.concurrent";

    expect(hasAsrProviderConfig(config)).toBe(false);
  });
});
