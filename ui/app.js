const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const RETRY_INTERVAL_MS = 5000;
let retryTimer = null;
let countdownTimer = null;

function showScreen(id) {
  document.querySelectorAll(".screen").forEach((el) => el.classList.remove("active"));
  const screen = document.getElementById(id);
  if (screen) screen.classList.add("active");
}

function setLoadingStatus(msg) {
  const el = document.getElementById("loading-status");
  if (el) el.textContent = msg;
}

function showError(title, message) {
  document.getElementById("error-title").textContent = title;
  document.getElementById("error-message").textContent = message;
  showScreen("error");
  startRetryCountdown();

  // Show proxy hint for connection errors
  const hint = document.getElementById("proxy-hint");
  if (hint) hint.style.display = "block";
}

function startRetryCountdown() {
  clearTimers();
  let remaining = RETRY_INTERVAL_MS / 1000;
  const el = document.getElementById("retry-countdown");

  countdownTimer = setInterval(() => {
    remaining--;
    if (remaining > 0) {
      el.textContent = `Auto-retry in ${remaining}s`;
    } else {
      el.textContent = "";
    }
  }, 1000);

  retryTimer = setTimeout(() => {
    clearTimers();
    retryConnection();
  }, RETRY_INTERVAL_MS);
}

function clearTimers() {
  if (retryTimer) clearTimeout(retryTimer);
  if (countdownTimer) clearInterval(countdownTimer);
  retryTimer = null;
  countdownTimer = null;
  const el = document.getElementById("retry-countdown");
  if (el) el.textContent = "";
}

async function retryConnection() {
  clearTimers();
  showScreen("loading");
  setLoadingStatus("Checking gateway...");
  await connectToGateway();
}

async function connectToGateway() {
  try {
    setLoadingStatus("Reading gateway configuration...");
    const info = await invoke("get_gateway_info");

    setLoadingStatus(`Checking gateway at port ${info.port}...`);
    const online = await invoke("check_gateway_status");

    if (online) {
      setLoadingStatus("Connected! Redirecting...");
      // Small delay so user sees the success state
      setTimeout(() => {
        window.location.replace(info.full_url);
      }, 300);
    } else {
      showError(
        "Gateway Offline",
        `Cannot reach OpenClaw gateway at 127.0.0.1:${info.port}. Make sure the service is running.`
      );
    }
  } catch (err) {
    showError("Configuration Error", String(err));
  }
}

// Listen for background health check events
listen("gateway-status", async (event) => {
  const status = event.payload;
  // If we're on the error screen and gateway comes online, auto-reconnect
  if (status === "online" && document.getElementById("error").classList.contains("active")) {
    clearTimers();
    showScreen("loading");
    setLoadingStatus("Gateway is back online! Reconnecting...");
    await connectToGateway();
  }
});

// Start connection on load
connectToGateway();
