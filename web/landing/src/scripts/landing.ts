const messages = [
  "Refactor complete — 42 files changed, all 218 tests passing. Take a look.",
  "Blocked: need your call on the DB migration strategy.",
  "PR #128 ready for review.",
  "CI is green on main. Deploy went out clean.",
  "Flaky test in auth suite — retried twice, needs eyes.",
];

const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

function updateLockScreen() {
  const now = new Date();
  const time = document.querySelector<HTMLElement>("#lock-time");
  const date = document.querySelector<HTMLElement>("#lock-date");

  if (time) {
    time.textContent = new Intl.DateTimeFormat(undefined, {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    }).format(now);
  }

  if (date) {
    date.textContent = new Intl.DateTimeFormat(undefined, {
      weekday: "long",
      month: "long",
      day: "numeric",
    }).format(now);
  }
}

function createNotification(message: string, warning: boolean) {
  const notification = document.createElement("article");
  notification.className = "notification notification-entering";

  const mark = document.createElement("div");
  mark.className = "telegram-mark";
  mark.textContent = "T";

  const copy = document.createElement("div");
  copy.className = "notification-copy";

  const header = document.createElement("header");
  const name = document.createElement("strong");
  name.textContent = "talbot";
  const timestamp = document.createElement("span");
  timestamp.textContent = "now";
  header.append(name, timestamp);

  const body = document.createElement("p");
  body.textContent = `${warning ? "⚠" : "✓"} ${message}`;
  copy.append(header, body);
  notification.append(mark, copy);

  return notification;
}

function startNotifications() {
  const container = document.querySelector<HTMLElement>("#notifications");
  if (!container || reducedMotion.matches) return;

  let messageIndex = 0;
  const addNotification = () => {
    const message = messages[messageIndex % messages.length];
    const warning = message.startsWith("Blocked") || message.startsWith("Flaky");
    messageIndex += 1;

    container.prepend(createNotification(message, warning));
    while (container.children.length > 3) {
      container.lastElementChild?.remove();
    }
  };

  window.setTimeout(addNotification, 1400);
  window.setInterval(addNotification, 4800);
}

function drawMeadow() {
  const canvas = document.querySelector<HTMLCanvasElement>("#meadow-canvas");
  const context = canvas?.getContext("2d");
  if (!canvas || !context) return;

  const width = canvas.width;
  const height = canvas.height;
  context.imageSmoothingEnabled = false;

  const random = (x: number, y: number) => {
    const value = Math.sin(x * 12.9898 + y * 78.233) * 43758.5453;
    return value - Math.floor(value);
  };

  const cloud = (x: number, y: number, scale: number) => {
    const left = Math.floor(x);
    context.fillStyle = "#ffffff";
    context.fillRect(left, y + 4 * scale, 26 * scale, 6 * scale);
    context.fillRect(left + 4 * scale, y + 2 * scale, 10 * scale, 4 * scale);
    context.fillRect(left + 12 * scale, y, 10 * scale, 6 * scale);
    context.fillRect(left + 20 * scale, y + 2 * scale, 6 * scale, 4 * scale);
  };

  const render = (elapsed: number) => {
    const drift = elapsed / 1000;
    const bands: Array<[string, number]> = [
      ["#4ea8e0", 0],
      ["#5fb3e6", 22],
      ["#79c2ec", 42],
      ["#97d1f0", 60],
      ["#b7e0f4", 74],
      ["#d3ecf8", 86],
    ];

    for (let index = 0; index < bands.length; index += 1) {
      const next = bands[index + 1]?.[1] ?? 100;
      context.fillStyle = bands[index][0];
      context.fillRect(0, bands[index][1], width, next - bands[index][1]);
    }

    context.fillStyle = "#fff3bd";
    context.fillRect(30, 14, 14, 14);
    context.fillRect(27, 17, 20, 8);
    context.fillRect(33, 11, 8, 20);

    cloud(((60 + drift * 10) % (width + 90)) - 70, 16, 2);
    cloud(((200 + drift * 7) % (width + 90)) - 70, 38, 1.4);
    cloud(((300 + drift * 13) % (width + 90)) - 70, 8, 1);

    for (let x = 0; x < width; x += 1) {
      const first = 96 - 7 * Math.sin((x + 30) / 34) - 4 * Math.sin(x / 13 + 1);
      context.fillStyle = "#aed393";
      context.fillRect(x, Math.floor(first), 1, height);

      const second = 116 - 8 * Math.sin(x / 46 + 2) - 4 * Math.sin(x / 19);
      context.fillStyle = "#d9cf5e";
      context.fillRect(x, Math.floor(second), 1, height);

      const third = 138 - 9 * Math.sin(x / 52 + 4) - 5 * Math.sin(x / 23 + 1);
      context.fillStyle = "#6fb05a";
      context.fillRect(x, Math.floor(third), 1, height);

      const fourth = 164 - 5 * Math.sin(x / 31 + 1.5);
      context.fillStyle = "#55984a";
      context.fillRect(x, Math.floor(fourth), 1, height);
    }

    for (let y = 100; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const noise = random(x, y);
        if (y > 104 && y < 132 && noise < 0.1) {
          context.fillStyle = noise < 0.05 ? "#e9e070" : "#c4bb4e";
          context.fillRect(x, y, 1, 1);
        } else if (y >= 132 && y < 162 && noise < 0.07) {
          context.fillStyle = "#63a250";
          context.fillRect(x, y, 1, 1);
        } else if (y >= 162 && noise < 0.09) {
          context.fillStyle = "#478741";
          context.fillRect(x, y, 1, 1);
        }
      }
    }

    const flowerColors = ["#ffffff", "#ffe08a", "#f6a5c0", "#e86a5f"];
    for (let index = 0; index < 40; index += 1) {
      const x = Math.floor(random(index, 7) * width);
      const y = Math.floor(140 + random(index, 9) * 36);
      context.fillStyle = flowerColors[index % flowerColors.length];
      context.fillRect(x, y, 2, 2);
      context.fillStyle = "#f8d54c";
      context.fillRect(x, y, 1, 1);
    }
  };

  let previousFrame = -Infinity;
  const animate = (elapsed: number) => {
    if (elapsed - previousFrame >= 120) {
      render(elapsed);
      previousFrame = elapsed;
    }
    window.requestAnimationFrame(animate);
  };

  render(0);
  if (!reducedMotion.matches) window.requestAnimationFrame(animate);
}

function initialiseCopyButton() {
  const button = document.querySelector<HTMLButtonElement>("#copy-install");
  const status = document.querySelector<HTMLElement>("#copy-status");
  if (!button || !status) return;

  button.addEventListener("click", async () => {
    const command = button.dataset.command;
    if (!command) return;

    try {
      await navigator.clipboard.writeText(command);
      status.textContent = "copied to clipboard";
      button.classList.add("copied");
    } catch {
      const commandText = button.querySelector("span");
      const selection = window.getSelection();
      if (commandText && selection) {
        const range = document.createRange();
        range.selectNodeContents(commandText);
        selection.removeAllRanges();
        selection.addRange(range);
      }
      status.textContent = "copy unavailable — command selected";
    }

    window.setTimeout(() => {
      status.textContent = "";
      button.classList.remove("copied");
    }, 1800);
  });
}

updateLockScreen();
drawMeadow();
startNotifications();
initialiseCopyButton();
