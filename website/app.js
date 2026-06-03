const baseTitle = "LUX - Local-first Game Automation Control Plane";

const messages = {
  en: {
    skip: "Skip to content",
    brandKicker: "Local automation toolkit",
    navRhythm: "Rhythm",
    navArchitecture: "Architecture",
    navEngines: "Engines",
    navVerify: "Verify",
    heroEyebrow: "English is the base content model",
    heroTitle: "Local-first evidence loop for AI game automation.",
    heroLede:
      "LUX gives coding agents a truthful runtime surface: project intent, engine state, bridge status, logs, screenshots, and verification results before progress is claimed.",
    heroPrimary: "Explore the control plane",
    heroSecondary: "View capability tiers",
    panelTitle: "Live truth route",
    signalTruth: "runtime truth",
    signalGateway: "CLI / HTTP / MCP",
    signalBridge: "engine adapter",
    signalEvidence: "verified result",
    rhythmEyebrow: "Operating rhythm",
    rhythmTitle: "Observe, route, act, verify, project.",
    rhythmIntro: "LUX deliberately slows automation down enough to make each claim inspectable.",
    observeTitle: "Observe",
    observeBody: "Read `.lux/`, engine status, logs, hierarchy, screenshots, and recent run evidence.",
    routeTitle: "Route",
    routeBody: "Choose the verified engine surface instead of assuming every adapter is equally mature.",
    actTitle: "Act",
    actBody: "Run commands through gateway-owned CLI, HTTP/WebSocket, MCP, or bridge paths.",
    verifyTitle: "Verify",
    verifyBody: "Capture compile, test, run, status, screenshot, or log evidence before completion.",
    projectTitle: "Project",
    projectBody: "Publish only proven state into docs, skills, CLI summaries, and `.lux/` projections.",
    architectureEyebrow: "Repository anatomy",
    architectureTitle: "Each layer owns one kind of truth.",
    layerLux: "Canonical runtime truth: specs, tickets, events, roadmap, sessions, and run evidence.",
    layerGateway: "Rust CLI, Axum HTTP/WS server, MCP tools, routing, and engine orchestration.",
    layerBridge: "Ordinary in-repository engine bridge source. No git submodule or external bridge remote.",
    layerSkills: "Manifest-backed workflow library projected into target projects with capability guardrails.",
    layerDocs: "Human-readable projection of runtime and product reality, never the SSoT over `.lux/`.",
    layerScripts: "Local verification, policy scanning, structure checks, and release maintenance.",
    enginesEyebrow: "Capability tiers",
    enginesTitle: "Unity is verified. Everything else says exactly what it can prove.",
    verifiedBadge: "verified",
    partialBadge: "partial",
    plannedBadge: "planned",
    unityBody:
      "Primary public-beta path for bridge install, status, compile, tests, run evidence, and screenshots.",
    godotBody:
      "Detection, bridge install, status, and workflow projection only; build/run/test remain unsupported.",
    threeBody:
      "Adapter files may exist, but runtime automation remains planned until a harness is present and verified.",
    verifyEyebrow: "Evidence surfaces",
    verifySectionTitle: "A claim is not done until a surface proves it.",
    commandsTitle: "Commands",
    evidenceTitle: "What LUX captures",
    evSpec: "Spec and domain decisions",
    evScene: "Scene hierarchy and selected object context",
    evLogs: "Compile, console, and runtime logs",
    evScreens: "Screenshots and visual checks",
    evRuns: "Run state, tickets, and verification history",
    footerOne: "LUX keeps local runtime truth observable.",
    footerDocs: "Read the English base README",
  },
  ko: {
    skip: "본문으로 이동",
    brandKicker: "로컬 자동화 툴킷",
    navRhythm: "리듬",
    navArchitecture: "아키텍처",
    navEngines: "엔진",
    navVerify: "검증",
    heroEyebrow: "영어가 기준 콘텐츠 모델입니다",
    heroTitle: "AI 게임 자동화를 위한 로컬 우선 증거 루프.",
    heroLede:
      "LUX는 agent가 진행을 주장하기 전에 프로젝트 의도, 엔진 상태, bridge 상태, 로그, 스크린샷, 검증 결과를 하나의 신뢰 가능한 런타임 표면으로 제공합니다.",
    heroPrimary: "컨트롤 플레인 보기",
    heroSecondary: "capability tier 보기",
    panelTitle: "Live truth route",
    signalTruth: "런타임 진실",
    signalGateway: "CLI / HTTP / MCP",
    signalBridge: "엔진 어댑터",
    signalEvidence: "검증된 결과",
    rhythmEyebrow: "운영 리듬",
    rhythmTitle: "관측하고, 라우팅하고, 행동하고, 검증하고, 투영합니다.",
    rhythmIntro: "LUX는 모든 완료 주장을 검사 가능하게 만들 만큼 자동화를 의도적으로 늦춥니다.",
    observeTitle: "관측",
    observeBody: "`.lux/`, 엔진 상태, 로그, 계층, 스크린샷, 최근 실행 증거를 읽습니다.",
    routeTitle: "라우팅",
    routeBody: "모든 adapter가 같은 maturity라고 가정하지 않고 검증된 엔진 표면을 선택합니다.",
    actTitle: "행동",
    actBody: "gateway가 소유한 CLI, HTTP/WebSocket, MCP, bridge 경로로 명령을 실행합니다.",
    verifyTitle: "검증",
    verifyBody: "완료 전에 compile, test, run, status, screenshot, log 증거를 캡처합니다.",
    projectTitle: "투영",
    projectBody: "검증된 상태만 docs, skills, CLI summary, `.lux/` projection으로 공개합니다.",
    architectureEyebrow: "저장소 구조",
    architectureTitle: "각 계층은 한 종류의 진실만 소유합니다.",
    layerLux: "정식 런타임 진실: spec, ticket, event, roadmap, session, run evidence.",
    layerGateway: "Rust CLI, Axum HTTP/WS 서버, MCP tools, routing, engine orchestration.",
    layerBridge: "저장소 내부 일반 engine bridge source. git submodule이나 외부 bridge remote가 아닙니다.",
    layerSkills: "capability guardrail과 함께 대상 프로젝트로 투영되는 manifest-backed workflow library.",
    layerDocs: "runtime과 product reality의 사람이 읽는 projection이며 `.lux/`보다 우선하지 않습니다.",
    layerScripts: "로컬 검증, policy scan, structure check, release maintenance.",
    enginesEyebrow: "Capability tiers",
    enginesTitle: "Unity는 검증됨. 나머지는 증명할 수 있는 만큼만 말합니다.",
    verifiedBadge: "verified",
    partialBadge: "partial",
    plannedBadge: "planned",
    unityBody: "bridge install, status, compile, tests, run evidence, screenshot의 기본 public-beta 경로.",
    godotBody: "detection, bridge install, status, workflow projection만 지원하며 build/run/test는 unsupported.",
    threeBody: "adapter file은 있을 수 있지만 harness가 존재하고 검증되기 전까지 runtime automation은 planned.",
    verifyEyebrow: "증거 표면",
    verifySectionTitle: "표면이 증명하기 전까지 완료가 아닙니다.",
    commandsTitle: "명령",
    evidenceTitle: "LUX가 캡처하는 것",
    evSpec: "spec과 domain decision",
    evScene: "scene hierarchy와 selected object context",
    evLogs: "compile, console, runtime log",
    evScreens: "screenshot과 visual check",
    evRuns: "run state, ticket, verification history",
    footerOne: "LUX는 로컬 런타임 진실을 관측 가능하게 유지합니다.",
    footerDocs: "영어 기준 README 읽기",
  },
};

const langButtons = Array.from(document.querySelectorAll("[data-lang-option]"));
const textNodes = Array.from(document.querySelectorAll("[data-i18n]"));

function preferredLanguage() {
  const params = new URLSearchParams(window.location.search);
  const requested = params.get("lang");
  if (requested === "ko" || requested === "en") return requested;
  return "en";
}

function setLanguage(lang, updateUrl = true) {
  const safeLang = lang === "ko" ? "ko" : "en";
  document.documentElement.lang = safeLang;
  document.documentElement.dataset.lang = safeLang;
  document.title = safeLang === "en" ? baseTitle : "LUX - 로컬 우선 게임 자동화 컨트롤 플레인";

  textNodes.forEach((node) => {
    const key = node.dataset.i18n;
    const value = messages[safeLang][key] || messages.en[key];
    if (value) node.textContent = value;
  });

  langButtons.forEach((button) => {
    button.setAttribute("aria-pressed", String(button.dataset.langOption === safeLang));
  });

  if (updateUrl) {
    const url = new URL(window.location.href);
    if (safeLang === "en") {
      url.searchParams.delete("lang");
    } else {
      url.searchParams.set("lang", safeLang);
    }
    window.history.replaceState({}, "", url);
  }
}

langButtons.forEach((button) => {
  button.addEventListener("click", () => setLanguage(button.dataset.langOption));
});

setLanguage(preferredLanguage(), false);

const animated = Array.from(document.querySelectorAll(".step-card, .layer-board article, .engine-card, .evidence-columns > div"));
animated.forEach((node) => node.setAttribute("data-animate", ""));

const observer = new IntersectionObserver(
  (entries) => {
    entries.forEach((entry) => {
      if (entry.isIntersecting) entry.target.classList.add("is-visible");
    });
  },
  { threshold: 0.12 }
);

animated.forEach((node) => observer.observe(node));

const canvas = document.getElementById("evidence-canvas");
const ctx = canvas.getContext("2d");
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
const nodes = [
  { label: ".lux", x: 0.18, y: 0.32, color: "#7ef3a5" },
  { label: "gateway", x: 0.46, y: 0.22, color: "#f4c95d" },
  { label: "bridge", x: 0.72, y: 0.36, color: "#7fd8ff" },
  { label: "engine", x: 0.62, y: 0.68, color: "#ff7d68" },
  { label: "evidence", x: 0.28, y: 0.72, color: "#eef3ef" },
];

let width = 0;
let height = 0;
let frame = 0;

function resizeCanvas() {
  const scale = Math.min(window.devicePixelRatio || 1, 2);
  width = window.innerWidth;
  height = window.innerHeight;
  canvas.width = Math.floor(width * scale);
  canvas.height = Math.floor(height * scale);
  canvas.style.width = `${width}px`;
  canvas.style.height = `${height}px`;
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
}

function draw() {
  frame += reducedMotion.matches ? 0 : 1;
  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = "#07110f";
  ctx.fillRect(0, 0, width, height);

  const points = nodes.map((node, index) => {
    const drift = Math.sin(frame * 0.012 + index) * 10;
    return {
      ...node,
      px: node.x * width + drift,
      py: node.y * height + Math.cos(frame * 0.01 + index) * 8,
    };
  });

  ctx.lineWidth = 1;
  for (let index = 0; index < points.length; index += 1) {
    const current = points[index];
    const next = points[(index + 1) % points.length];
    const pulse = (Math.sin(frame * 0.035 + index) + 1) / 2;
    ctx.strokeStyle = `rgba(126, 243, 165, ${0.12 + pulse * 0.22})`;
    ctx.beginPath();
    ctx.moveTo(current.px, current.py);
    ctx.bezierCurveTo(width * 0.5, current.py, width * 0.5, next.py, next.px, next.py);
    ctx.stroke();
  }

  points.forEach((point, index) => {
    const radius = 24 + Math.sin(frame * 0.025 + index) * 4;
    const gradient = ctx.createRadialGradient(point.px, point.py, 2, point.px, point.py, radius * 3.4);
    gradient.addColorStop(0, `${point.color}cc`);
    gradient.addColorStop(0.24, `${point.color}40`);
    gradient.addColorStop(1, `${point.color}00`);
    ctx.fillStyle = gradient;
    ctx.beginPath();
    ctx.arc(point.px, point.py, radius * 3.4, 0, Math.PI * 2);
    ctx.fill();

    ctx.fillStyle = point.color;
    ctx.beginPath();
    ctx.arc(point.px, point.py, 4.5, 0, Math.PI * 2);
    ctx.fill();
  });

  if (!reducedMotion.matches) requestAnimationFrame(draw);
}

resizeCanvas();
draw();
window.addEventListener("resize", () => {
  resizeCanvas();
  draw();
});
