const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let currentStep = 1;
let prereqStatus = null;
let proxyInfo = null;

const PROVIDER_INFO = {
  minimax: {
    hint: "前往 MiniMax 开放平台获取免费 API Key",
    defaultModel: "MiniMax-M1",
    modelHint: "MiniMax 默认使用 MiniMax-M1 模型",
  },
  openai: {
    hint: "前往 platform.openai.com 获取 API Key",
    defaultModel: "gpt-4o",
    modelHint: "推荐使用 gpt-4o 或 gpt-4o-mini",
  },
  anthropic: {
    hint: "前往 console.anthropic.com 获取 API Key",
    defaultModel: "claude-sonnet-4-5-20250929",
    modelHint: "推荐使用 Claude Sonnet 4.5",
  },
  deepseek: {
    hint: "前往 platform.deepseek.com 获取 API Key",
    defaultModel: "deepseek-chat",
    modelHint: "推荐使用 deepseek-chat",
  },
  openrouter: {
    hint: "前往 openrouter.ai 获取 API Key",
    defaultModel: "openai/gpt-4o",
    modelHint: "使用 provider/model 格式，如 openai/gpt-4o",
  },
};

// External links - open in system browser
function setupExternalLinks() {
  const links = {
    "nodejs-download-link": "https://nodejs.org/",
    "telegram-botfather-link": "https://t.me/BotFather",
  };

  for (const [id, url] of Object.entries(links)) {
    const el = document.getElementById(id);
    if (el) {
      el.addEventListener("click", (e) => {
        e.preventDefault();
        window.__TAURI__.shell.open(url);
      });
    }
  }
}

function goToStep(step) {
  document.querySelectorAll(".setup-step").forEach((el) => el.classList.remove("active"));
  document.querySelectorAll(".step-dot").forEach((el) => {
    const s = parseInt(el.dataset.step);
    el.classList.remove("active", "done");
    if (s < step) el.classList.add("done");
    if (s === step) el.classList.add("active");
  });

  const target = document.getElementById(`step-${step}`);
  if (target) target.classList.add("active");
  currentStep = step;

  if (step === 4) {
    detectProxy();
  }

  if (step === 5) {
    updateSummary();
  }
}

function setCheckStatus(id, ok, detail) {
  const item = document.getElementById(id);
  if (!item) return;
  const icon = item.querySelector(".check-icon");
  if (ok) {
    icon.textContent = "\u2713";
    item.classList.add("success");
    item.classList.remove("fail");
  } else {
    icon.textContent = "\u2717";
    item.classList.add("fail");
    item.classList.remove("success");
  }
  const detailEl = document.getElementById(id.replace("check-", "") + "-detail");
  if (detailEl && detail) detailEl.textContent = detail;
}

async function checkPrereqs() {
  try {
    prereqStatus = await invoke("check_prerequisites");

    setCheckStatus(
      "check-node",
      prereqStatus.node_installed,
      prereqStatus.node_installed ? prereqStatus.node_version : "未安装"
    );

    setCheckStatus(
      "check-npm",
      prereqStatus.npm_installed,
      prereqStatus.npm_installed ? "已安装" : "未安装"
    );

    setCheckStatus(
      "check-openclaw",
      prereqStatus.openclaw_installed,
      prereqStatus.openclaw_installed ? prereqStatus.openclaw_version : "未安装"
    );

    // Show help for missing dependencies
    const nodeHelp = document.getElementById("node-help");
    const openclawBox = document.getElementById("openclaw-install-box");
    const actions = document.getElementById("step1-actions");

    if (!prereqStatus.node_installed || !prereqStatus.npm_installed) {
      nodeHelp.style.display = "block";
      openclawBox.style.display = "none";
      actions.style.display = "none";
    } else if (!prereqStatus.openclaw_installed) {
      nodeHelp.style.display = "none";
      openclawBox.style.display = "block";
      actions.style.display = "none";
    } else {
      nodeHelp.style.display = "none";
      openclawBox.style.display = "none";
      actions.style.display = "flex";
    }

    document.querySelector("#step-1 .step-desc").textContent = "环境检测完成";
  } catch (err) {
    document.querySelector("#step-1 .step-desc").textContent =
      "检测失败: " + String(err);
  }
}

async function recheckPrereqs() {
  // Reset icons
  document.querySelectorAll(".check-icon").forEach((el) => (el.textContent = "\u231b"));
  document.querySelectorAll(".check-item").forEach((el) => {
    el.classList.remove("success", "fail");
  });
  document.querySelector("#step-1 .step-desc").textContent = "正在重新检测...";
  await checkPrereqs();
}

async function installOpenclaw() {
  const btn = document.getElementById("install-openclaw-btn");
  const progress = document.getElementById("install-progress");
  const result = document.getElementById("install-result");

  btn.style.display = "none";
  progress.style.display = "flex";
  result.textContent = "";
  result.className = "install-result";

  try {
    const output = await invoke("install_openclaw");
    result.textContent = "安装成功！";
    result.classList.add("success");
    progress.style.display = "none";

    // Re-check prerequisites
    await recheckPrereqs();
  } catch (err) {
    result.textContent = "安装失败: " + String(err);
    result.classList.add("fail");
    progress.style.display = "none";
    btn.style.display = "inline-block";
  }
}

function onProviderChange() {
  const provider = document.getElementById("provider-select").value;
  const info = PROVIDER_INFO[provider];
  if (!info) return;

  document.getElementById("provider-hint").textContent = info.hint;
  document.getElementById("model-input").value = info.defaultModel;
  document.getElementById("model-hint").textContent = info.modelHint;
}

function validateAndNext(nextStep) {
  const apiKey = document.getElementById("api-key-input").value.trim();
  if (!apiKey) {
    document.getElementById("api-key-input").classList.add("input-error");
    document.getElementById("api-key-input").focus();
    return;
  }
  document.getElementById("api-key-input").classList.remove("input-error");
  goToStep(nextStep);
}

async function detectProxy() {
  const statusEl = document.getElementById("proxy-status");
  const detectedEl = document.getElementById("proxy-detected");
  const notDetectedEl = document.getElementById("proxy-not-detected");

  statusEl.style.display = "flex";
  detectedEl.style.display = "none";
  notDetectedEl.style.display = "none";

  try {
    proxyInfo = await invoke("detect_proxy");

    statusEl.style.display = "none";

    if (proxyInfo.detected) {
      detectedEl.style.display = "block";
      let detail = proxyInfo.http;
      if (proxyInfo.socks) {
        detail += " / SOCKS: " + proxyInfo.socks;
      }
      detail += " (来源: " + proxyInfo.source + ")";
      document.getElementById("proxy-detail-text").textContent = detail;
    } else {
      notDetectedEl.style.display = "block";
    }
  } catch (err) {
    statusEl.style.display = "none";
    notDetectedEl.style.display = "block";
  }
}

function getProxyUrl() {
  if (proxyInfo && proxyInfo.detected) {
    return proxyInfo.http;
  }
  const manual = document.getElementById("manual-proxy");
  if (manual) {
    return manual.value.trim();
  }
  return "";
}

function maskKey(key) {
  if (!key || key.length < 8) return "***";
  return key.substring(0, 4) + "****" + key.substring(key.length - 4);
}

function updateSummary() {
  const provider = document.getElementById("provider-select").value;
  const model = document.getElementById("model-input").value.trim();
  const apiKey = document.getElementById("api-key-input").value.trim();
  const telegram = document.getElementById("telegram-token").value.trim();
  const discord = document.getElementById("discord-token").value.trim();
  const proxy = getProxyUrl();

  document.getElementById("summary-provider").textContent = provider;
  document.getElementById("summary-model").textContent = model || "-";
  document.getElementById("summary-apikey").textContent = maskKey(apiKey);
  document.getElementById("summary-telegram").textContent = telegram ? "已配置" : "未配置";
  document.getElementById("summary-discord").textContent = discord ? "已配置" : "未配置";
  document.getElementById("summary-proxy").textContent = proxy || "无";
}

async function saveAndLaunch() {
  const launchBtn = document.getElementById("launch-btn");
  const savingEl = document.getElementById("saving-indicator");
  const errorEl = document.getElementById("save-error");
  const actionsEl = document.getElementById("step5-actions");

  launchBtn.disabled = true;
  savingEl.style.display = "flex";
  errorEl.style.display = "none";

  const provider = document.getElementById("provider-select").value;
  const apiKey = document.getElementById("api-key-input").value.trim();
  const model = document.getElementById("model-input").value.trim();
  const telegram = document.getElementById("telegram-token").value.trim() || null;
  const discord = document.getElementById("discord-token").value.trim() || null;
  const proxy = getProxyUrl() || null;

  try {
    await invoke("save_initial_config", {
      provider,
      apiKey,
      model,
      telegramToken: telegram,
      discordToken: discord,
      proxyUrl: proxy,
    });

    savingEl.querySelector("span").textContent = "配置已保存，正在启动 Gateway...";

    // Start the gateway
    await invoke("start_gateway");

    savingEl.querySelector("span").textContent = "启动成功！正在跳转...";

    // Navigate to the main UI
    setTimeout(() => {
      window.location.replace("index.html");
    }, 500);
  } catch (err) {
    errorEl.textContent = "保存失败: " + String(err);
    errorEl.style.display = "block";
    savingEl.style.display = "none";
    launchBtn.disabled = false;
  }
}

// Initialize
document.addEventListener("DOMContentLoaded", () => {
  setupExternalLinks();
  checkPrereqs();
});
