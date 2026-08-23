#!/usr/bin/env python3
"""Generate assets/hero.svg for the TALBot README.

Recreates the landing-page hero: the pixel meadow is rendered once with the
exact algorithm from the page's canvas drawScene(), embedded as a base64 PNG;
clouds and the notification stack are animated with SMIL/CSS inside the SVG
so it plays when GitHub serves it as an image.
"""
import base64, math, struct, zlib

W, H = 320, 180

# ---- pixel buffer (base = lowest sky band, like the canvas over the page bg)
px = [[(0xd3, 0xec, 0xf8)] * W for _ in range(H)]

def hexc(s):
    s = s.lstrip('#')
    return (int(s[0:2], 16), int(s[2:4], 16), int(s[4:6], 16))

def fill_rect(x, y, w, h, c):
    c = hexc(c)
    for yy in range(max(0, int(y)), min(H, int(y + h))):
        for xx in range(max(0, int(x)), min(W, int(x + w))):
            px[yy][xx] = c

def rnd(x, y):
    r = math.sin(x * 12.9898 + y * 78.233) * 43758.5453
    return r - math.floor(r)

bands = [('#4ea8e0', 0), ('#5fb3e6', 22), ('#79c2ec', 42), ('#97d1f0', 60), ('#b7e0f4', 74), ('#d3ecf8', 86)]
for i, (col, y0) in enumerate(bands):
    y1 = bands[i + 1][1] if i + 1 < len(bands) else 100
    fill_rect(0, y0, W, y1 - y0, col)

# sun
fill_rect(30, 14, 14, 14, '#fff3bd')
fill_rect(27, 17, 20, 8, '#fff3bd')
fill_rect(33, 11, 8, 20, '#fff3bd')

# rolling hills
for x in range(W):
    y1 = int(96 - 7 * math.sin((x + 30) / 34) - 4 * math.sin(x / 13 + 1))
    fill_rect(x, y1, 1, H, '#aed393')
    y2 = int(116 - 8 * math.sin(x / 46 + 2) - 4 * math.sin(x / 19))
    fill_rect(x, y2, 1, H, '#d9cf5e')
    y3 = int(138 - 9 * math.sin(x / 52 + 4) - 5 * math.sin(x / 23 + 1))
    fill_rect(x, y3, 1, H, '#6fb05a')
    y4 = int(164 - 5 * math.sin(x / 31 + 1.5))
    fill_rect(x, y4, 1, H, '#55984a')

# grass speckle
for y in range(100, H):
    for x in range(W):
        n = rnd(x, y)
        if 104 < y < 132 and n < 0.1:
            px[y][x] = hexc('#e9e070' if n < 0.05 else '#c4bb4e')
        elif 132 <= y < 162 and n < 0.07:
            px[y][x] = hexc('#63a250')
        elif y >= 162 and n < 0.09:
            px[y][x] = hexc('#478741')

# flowers
cols = ['#ffffff', '#ffe08a', '#f6a5c0', '#e86a5f']
for i in range(40):
    fx = int(rnd(i, 7) * W)
    fy = int(140 + rnd(i, 9) * 36)
    fill_rect(fx, fy, 2, 2, cols[i % 4])
    fill_rect(fx, fy, 1, 1, '#f8d54c')

# ---- minimal PNG encoder ----------------------------------------------------
def png_chunk(tag, data):
    c = struct.pack('>I', len(data)) + tag + data
    return c + struct.pack('>I', zlib.crc32(tag + data) & 0xffffffff)

raw = b''.join(b'\x00' + b''.join(struct.pack('BBB', *px[y][x]) for x in range(W)) for y in range(H))
png = (b'\x89PNG\r\n\x1a\n'
       + png_chunk(b'IHDR', struct.pack('>IIBBBBB', W, H, 8, 2, 0, 0, 0))
       + png_chunk(b'IDAT', zlib.compress(raw, 9))
       + png_chunk(b'IEND', b''))
meadow_b64 = base64.b64encode(png).decode()

# ---- SVG --------------------------------------------------------------------
S = 3  # pixel scale: scene is 960x540

def cloud(s):
    r = []
    for (x, y, w, h) in [(0, 4, 26, 6), (4, 2, 10, 4), (12, 0, 10, 6), (20, 2, 6, 4)]:
        r.append(f'<rect x="{x*s*S:g}" y="{y*s*S:g}" width="{w*s*S:g}" height="{h*s*S:g}"/>')
    return ''.join(r)

MSGS = [
    "✅ Refactor complete — 42 files changed, all 218 tests passing. Take a look.",
    "⚠️ Blocked: need your call on the DB migration strategy.",
    "✅ PR #128 ready for review.",
    "✅ CI is green on main. Deploy went out clean.",
    "⚠️ Flaky test in auth suite — retried twice, needs eyes.",
]

def wrap2(text, limit=44):
    words, lines, cur = text.split(), [], ''
    for w in words:
        t = (cur + ' ' + w).strip()
        if len(t) <= limit or not cur:
            cur = t
        else:
            lines.append(cur); cur = w
    lines.append(cur)
    if len(lines) > 2:
        lines = [lines[0], ' '.join(lines[1:])]
        if len(lines[1]) > limit:
            lines[1] = lines[1][:limit - 1].rstrip() + '…'
    return lines

CARD_W, CARD_H, GAP = 252, 46, 10
STEP = 4.8                     # seconds per message, same cadence as the page
CYCLE = STEP * len(MSGS)
dy1, dy2 = CARD_H + GAP, (CARD_H + GAP) * 2
P = 100 * STEP / CYCLE         # 20% of the cycle per slot

notif_cards = []
for i, text in enumerate(MSGS):
    lines = wrap2(text)
    tspans = ''.join(
        f'<tspan x="42" dy="{0 if j == 0 else 12}">{ln}</tspan>' for j, ln in enumerate(lines)
    )
    body_y = 26 if len(lines) > 1 else 31
    delay = 1.4 + i * STEP     # page shows the first notification at 1.4s
    notif_cards.append(f'''
  <g class="notif" style="animation-delay:{delay:g}s">
      <rect width="{CARD_W}" height="{CARD_H}" rx="12" fill="rgba(250,250,252,0.92)"/>
      <rect x="9" y="9" width="24" height="24" rx="7" fill="#2AABEE"/>
      <text x="21" y="26" text-anchor="middle" font-family="-apple-system,'Segoe UI',sans-serif" font-size="13" font-weight="700" fill="#ffffff">T</text>
      <text x="42" y="17" font-family="-apple-system,'Segoe UI',sans-serif" font-size="9" font-weight="600" fill="#1c1c1e">talbot</text>
      <text x="{CARD_W - 10}" y="17" text-anchor="end" font-family="-apple-system,'Segoe UI',sans-serif" font-size="8" fill="#6b6b70">now</text>
      <text y="{body_y}" font-family="-apple-system,'Segoe UI',sans-serif" font-size="8.5" fill="#2c2c2e">{tspans}</text>
  </g>''')

# Cloud tracks: the loop restart happens outside the viewBox so the drift
# reads as continuous.
clouds_svg = f'''
  <g fill="#ffffff">
    <g><animateTransform attributeName="transform" type="translate" from="-160 48" to="1000 48" dur="55s" begin="-38s" repeatCount="indefinite"/>{cloud(2)}</g>
    <g><animateTransform attributeName="transform" type="translate" from="-120 114" to="1000 114" dur="80s" begin="-20s" repeatCount="indefinite"/>{cloud(1.4)}</g>
    <g><animateTransform attributeName="transform" type="translate" from="-90 24" to="1000 24" dur="42s" begin="-5s" repeatCount="indefinite"/>{cloud(1)}</g>
  </g>'''

svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 960 620" font-family="Georgia, 'Times New Roman', serif">
  <style>
    .notif {{
      animation: stack {CYCLE:g}s cubic-bezier(0.2,0.8,0.2,1) infinite both;
      transform-box: fill-box;
      transform-origin: 50% 100%;
      opacity: 0;
    }}
    @keyframes stack {{
      0%      {{ transform: translateY(22px) scale(0.94);      opacity: 0; }}
      2.3%    {{ transform: translateY(0) scale(1);            opacity: 1; }}
      {P:g}%     {{ transform: translateY(0) scale(1);            opacity: 1; }}
      {P+3:g}%     {{ transform: translateY({-dy1}px) scale(0.95); opacity: 0.78; }}
      {2*P:g}%     {{ transform: translateY({-dy1}px) scale(0.95); opacity: 0.78; }}
      {2*P+3:g}%     {{ transform: translateY({-dy2}px) scale(0.90); opacity: 0.56; }}
      {3*P-3:g}%     {{ transform: translateY({-dy2}px) scale(0.90); opacity: 0.56; }}
      {3*P:g}%     {{ transform: translateY({-dy2-14}px) scale(0.88); opacity: 0; }}
      100%    {{ transform: translateY({-dy2-14}px) scale(0.88); opacity: 0; }}
    }}
    @media (prefers-reduced-motion: reduce) {{
      .notif {{ animation: none; }}
      .notif:first-of-type {{ opacity: 1; }}
    }}
  </style>

  <!-- page background: keep it opaque so dark-mode GitHub doesn't show through -->
  <rect width="960" height="620" fill="#fdfdfb"/>

  <!-- pixel meadow, rendered once from the landing page's canvas algorithm -->
  <image href="data:image/png;base64,{meadow_b64}" width="960" height="540" style="image-rendering:pixelated" preserveAspectRatio="none"/>
{clouds_svg}

  <!-- phone lying in the meadow (flattened to fake the CSS 3D tilt) -->
  <g transform="translate(700,455)">
    <ellipse cx="0" cy="18" rx="120" ry="26" fill="rgba(25,50,30,0.32)"/>
    <g transform="rotate(-9) scale(1,0.42) rotate(20)">
      <rect x="-78" y="-150" width="156" height="300" rx="26" fill="rgba(255,255,255,0.22)" stroke="#1a1c20" stroke-width="9"/>
      <rect x="-78" y="-150" width="156" height="300" rx="26" fill="none" stroke="#35383e" stroke-width="1.5"/>
      <rect x="-26" y="-138" width="52" height="14" rx="7" fill="#101114"/>
      <text x="0" y="-92" text-anchor="middle" font-family="-apple-system,'Segoe UI',sans-serif" font-size="10" font-weight="600" fill="#ffffff" opacity="0.92">Wednesday, August 19</text>
      <text x="0" y="-48" text-anchor="middle" font-family="-apple-system,'Segoe UI',sans-serif" font-size="44" font-weight="600" fill="#ffffff">09:41</text>
      <rect x="-30" y="136" width="60" height="4" rx="2" fill="rgba(255,255,255,0.9)"/>
    </g>
  </g>

  <!-- notification stack floating above the phone -->
  <g transform="translate(584,330)">
{''.join(notif_cards)}
  </g>

  <!-- wordmark -->
  <g text-anchor="middle">
    <text x="480" y="588" font-size="30" letter-spacing="16" fill="#3a3f35">talbot</text>
    <text x="480" y="612" font-size="13" font-style="italic" fill="#555b4e">stop watching your agents work; it'll text you</text>
  </g>

  <!-- frame -->
  <rect x="1.5" y="1.5" width="957" height="617" fill="none" stroke="#3a3f35" stroke-width="3"/>
</svg>
'''

open('/root/dev/TALBot/assets/hero.svg', 'w').write(svg)
print('wrote assets/hero.svg,', len(svg) // 1024, 'KB')
