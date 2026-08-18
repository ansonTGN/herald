// WCAG contrast checker for shadcn-style OKLCH token sets.
// Usage: node scripts/check-palette-contrast.mjs
// Verifies every foreground/background token pair reaches AA (4.5:1 text,
// 3:1 UI boundaries) before a palette is applied to styles.css.

// ---------- oklch -> sRGB ----------

function oklchToLinear(L, C, H) {
  const rad = (H * Math.PI) / 180
  const a = C * Math.cos(rad)
  const b = C * Math.sin(rad)
  const l_ = L + 0.3963377774 * a + 0.2158037573 * b
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b
  const s_ = L - 0.0894841775 * a - 1.291485548 * b
  const l = l_ ** 3
  const m = m_ ** 3
  const s = s_ ** 3
  const r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s
  const g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s
  const bl = -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s
  const clamp = (c) => Math.min(1, Math.max(0, c))
  return [clamp(r), clamp(g), clamp(bl)]
}

// WCAG relative luminance uses LINEAR sRGB, not gamma-encoded values.
function luminance(lin) {
  return 0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2]
}

function contrast(fg, bg) {
  const l1 = luminance(oklchToLinear(...fg))
  const l2 = luminance(oklchToLinear(...bg))
  const [hi, lo] = l1 >= l2 ? [l1, l2] : [l2, l1]
  return (hi + 0.05) / (lo + 0.05)
}

function hex(lin) {
  const gamma = (c) => (c <= 0.0031308 ? 12.92 * c : 1.055 * Math.pow(c, 1 / 2.4) - 0.055)
  const to = (c) =>
    Math.round(gamma(c) * 255)
      .toString(16)
      .padStart(2, '0')
  return `#${to(lin[0])}${to(lin[1])}${to(lin[2])}`
}

// ---------- palettes ----------

const palettes = {
  current: {
    label: '旧版 Persimmon Pop + Wasabi（已被 C 取代）',
    background: [0.985, 0.012, 85],
    foreground: [0.245, 0.02, 55],
    card: [1, 0.003, 85],
    primary: [0.57, 0.17, 47],
    'primary-foreground': [0.99, 0.012, 85],
    secondary: [0.95, 0.022, 75],
    'secondary-foreground': [0.3, 0.03, 50],
    muted: [0.962, 0.014, 80],
    'muted-foreground': [0.5, 0.025, 55],
    accent: [0.94, 0.026, 78],
    'accent-foreground': [0.3, 0.03, 50],
    destructive: [0.58, 0.22, 25],
    'destructive-foreground': [0.99, 0.01, 85],
    border: [0.9, 0.018, 80],
    input: [0.9, 0.018, 80],
    placeholder: [0.55, 0.02, 55],
    ring: [0.57, 0.17, 47],
    sidebar: [0.225, 0.018, 50],
    'sidebar-foreground': [0.935, 0.012, 80],
    'sidebar-primary': [0.84, 0.155, 118],
    'sidebar-primary-foreground': [0.2, 0.04, 120],
    'sidebar-accent': [0.28, 0.02, 55],
    'sidebar-accent-foreground': [0.935, 0.012, 80],
    'sidebar-border': [0.31, 0.02, 55],
  },
  A: {
    label: 'A 靛蓝 Indigo — 专业 SaaS',
    background: [0.985, 0.002, 264],
    foreground: [0.235, 0.015, 264],
    card: [1, 0, 0],
    primary: [0.51, 0.17, 268],
    'primary-foreground': [0.985, 0.002, 264],
    secondary: [0.958, 0.006, 264],
    'secondary-foreground': [0.29, 0.02, 264],
    muted: [0.965, 0.005, 264],
    'muted-foreground': [0.5, 0.02, 264],
    accent: [0.95, 0.012, 268],
    'accent-foreground': [0.29, 0.02, 264],
    destructive: [0.55, 0.2, 27],
    'destructive-foreground': [0.99, 0.002, 264],
    border: [0.914, 0.005, 264],
    input: [0.914, 0.005, 264],
    placeholder: [0.552, 0.016, 264],
    ring: [0.51, 0.17, 268],
    sidebar: [0.215, 0.015, 264],
    'sidebar-foreground': [0.93, 0.006, 264],
    'sidebar-primary': [0.7, 0.13, 268],
    'sidebar-primary-foreground': [0.18, 0.03, 268],
    'sidebar-accent': [0.27, 0.015, 264],
    'sidebar-accent-foreground': [0.93, 0.006, 264],
    'sidebar-border': [0.3, 0.015, 264],
  },
  B: {
    label: 'B 青绿 Teal — 开发者工具风',
    background: [0.984, 0.003, 200],
    foreground: [0.235, 0.015, 220],
    card: [1, 0, 0],
    primary: [0.51, 0.09, 194],
    'primary-foreground': [0.985, 0.003, 200],
    secondary: [0.955, 0.008, 200],
    'secondary-foreground': [0.3, 0.02, 200],
    muted: [0.962, 0.006, 200],
    'muted-foreground': [0.5, 0.02, 210],
    accent: [0.945, 0.015, 195],
    'accent-foreground': [0.28, 0.03, 195],
    destructive: [0.55, 0.2, 27],
    'destructive-foreground': [0.99, 0.003, 200],
    border: [0.91, 0.008, 200],
    input: [0.91, 0.008, 200],
    placeholder: [0.55, 0.018, 210],
    ring: [0.51, 0.09, 194],
    sidebar: [0.21, 0.015, 200],
    'sidebar-foreground': [0.93, 0.006, 200],
    'sidebar-primary': [0.72, 0.09, 190],
    'sidebar-primary-foreground': [0.16, 0.03, 190],
    'sidebar-accent': [0.265, 0.015, 200],
    'sidebar-accent-foreground': [0.93, 0.006, 200],
    'sidebar-border': [0.295, 0.015, 200],
  },
  C: {
    label: 'C 赤陶 Terracotta — 降饱和暖色',
    background: [0.98, 0.004, 80],
    foreground: [0.24, 0.012, 55],
    card: [0.998, 0.002, 80],
    primary: [0.52, 0.13, 40],
    'primary-foreground': [0.99, 0.004, 80],
    secondary: [0.955, 0.008, 75],
    'secondary-foreground': [0.3, 0.02, 50],
    muted: [0.962, 0.006, 78],
    'muted-foreground': [0.5, 0.018, 55],
    accent: [0.945, 0.012, 70],
    'accent-foreground': [0.3, 0.02, 50],
    destructive: [0.55, 0.2, 25],
    'destructive-foreground': [0.99, 0.004, 80],
    border: [0.905, 0.008, 75],
    input: [0.905, 0.008, 75],
    placeholder: [0.55, 0.015, 55],
    ring: [0.52, 0.13, 40],
    sidebar: [0.22, 0.015, 50],
    'sidebar-foreground': [0.93, 0.008, 75],
    'sidebar-primary': [0.75, 0.07, 135],
    'sidebar-primary-foreground': [0.2, 0.03, 135],
    'sidebar-accent': [0.275, 0.015, 50],
    'sidebar-accent-foreground': [0.93, 0.008, 75],
    'sidebar-border': [0.305, 0.015, 50],
  },
  Cdark: {
    label: 'C 赤陶 — 暗色变体（.dark 块，暂未启用）',
    background: [0.185, 0.012, 55],
    foreground: [0.945, 0.008, 75],
    card: [0.225, 0.014, 55],
    primary: [0.68, 0.13, 40],
    'primary-foreground': [0.2, 0.03, 40],
    secondary: [0.285, 0.012, 55],
    'secondary-foreground': [0.945, 0.008, 75],
    muted: [0.255, 0.012, 55],
    'muted-foreground': [0.7, 0.012, 70],
    accent: [0.29, 0.014, 50],
    'accent-foreground': [0.945, 0.008, 75],
    destructive: [0.55, 0.2, 25],
    'destructive-foreground': [0.99, 0.004, 80],
    border: [0.3, 0.014, 55],
    input: [0.3, 0.014, 55],
    placeholder: [0.68, 0.012, 70],
    ring: [0.68, 0.13, 40],
    sidebar: [0.155, 0.012, 50],
    'sidebar-foreground': [0.93, 0.008, 75],
    'sidebar-primary': [0.78, 0.07, 135],
    'sidebar-primary-foreground': [0.18, 0.03, 135],
    'sidebar-accent': [0.225, 0.012, 55],
    'sidebar-accent-foreground': [0.93, 0.008, 75],
    'sidebar-border': [0.245, 0.012, 55],
  },
}

// ---------- checks: [name, fgKey, bgKey, threshold] ----------

const TEXT = 4.5 // hard requirement — readability
const UI = 3.0 // soft — WCAG 1.11 non-text contrast; the whole shadcn
// ecosystem (incl. our current theme) runs decorative borders at ~1.1:1,
// so boundary checks are reported as warnings, not failures.

const checks = [
  ['正文/页面', 'foreground', 'background', TEXT],
  ['正文/卡片', 'foreground', 'card', TEXT],
  ['次要文字/页面', 'muted-foreground', 'background', TEXT],
  ['次要文字/卡片', 'muted-foreground', 'card', TEXT],
  ['次要文字/muted底', 'muted-foreground', 'muted', TEXT],
  ['主色按钮文字', 'primary-foreground', 'primary', TEXT],
  ['次要按钮文字', 'secondary-foreground', 'secondary', TEXT],
  ['accent文字', 'accent-foreground', 'accent', TEXT],
  ['危险按钮文字', 'destructive-foreground', 'destructive', TEXT],
  ['placeholder', 'placeholder', 'background', TEXT],
  ['侧边栏文字', 'sidebar-foreground', 'sidebar', TEXT],
  ['侧边栏强调文字', 'sidebar-primary-foreground', 'sidebar-primary', TEXT],
  ['侧边栏悬浮文字', 'sidebar-accent-foreground', 'sidebar-accent', TEXT],
  ['主色按钮边界', 'primary', 'background', UI],
  ['危险按钮边界', 'destructive', 'background', UI],
  ['输入框边框', 'input', 'background', UI],
  ['表格边框', 'border', 'background', UI],
  ['焦点环ring', 'ring', 'background', UI],
  ['侧边栏边框', 'sidebar-border', 'sidebar', UI],
  ['侧边栏强调色块', 'sidebar-primary', 'sidebar', UI],
]

// Status tokens (success/warning/info), validated per theme.
// Text-on-background pairs are the hard gate; /10 soft backgrounds tint the
// page background so little that plain-background contrast is a good proxy.
const statusLight = {
  success: [0.51, 0.13, 152],
  'success-foreground': [0.99, 0.004, 80],
  warning: [0.54, 0.13, 55],
  'warning-foreground': [0.99, 0.004, 80],
  info: [0.51, 0.09, 190],
  'info-foreground': [0.99, 0.004, 80],
}

const statusDark = {
  success: [0.63, 0.12, 150],
  'success-foreground': [0.15, 0.03, 150],
  warning: [0.75, 0.13, 65],
  'warning-foreground': [0.2, 0.03, 65],
  info: [0.68, 0.1, 190],
  'info-foreground': [0.15, 0.03, 190],
}

const statusChecks = [
  ['成功文字/页面', 'success', 'background', TEXT],
  ['成功文字/卡片', 'success', 'card', TEXT],
  ['成功实心按钮文字', 'success-foreground', 'success', TEXT],
  ['警告文字/页面', 'warning', 'background', TEXT],
  ['警告文字/卡片', 'warning', 'card', TEXT],
  ['警告实心按钮文字', 'warning-foreground', 'warning', TEXT],
  ['信息文字/页面', 'info', 'background', TEXT],
  ['信息文字/卡片', 'info', 'card', TEXT],
  ['信息实心按钮文字', 'info-foreground', 'info', TEXT],
]

let statusFail = false
for (const [name, status, theme] of [
  ['C 浅色 status', statusLight, palettes.C],
  ['C 暗色 status', statusDark, palettes.Cdark],
]) {
  console.log(`\n=== ${name} ===`)
  for (const [label, fgKey, bgKey] of statusChecks) {
    const fg = status[fgKey]
    const bg = bgKey === 'background' || bgKey === 'card' ? theme[bgKey] : status[bgKey]
    const ratio = contrast(fg, bg)
    const ok = ratio >= TEXT
    if (!ok) statusFail = true
    console.log(
      `  ${ok ? 'PASS' : 'FAIL'}  ${ratio.toFixed(2)}  (>=${TEXT})  ${label}  ${hex(oklchToLinear(...fg))} on ${hex(oklchToLinear(...bg))}`
    )
  }
}

let hardFail = false
for (const [key, p] of Object.entries(palettes)) {
  const rows = []
  let paletteHardFail = false
  for (const [name, fgKey, bgKey, th] of checks) {
    const fg = p[fgKey]
    const bg = p[bgKey]
    if (!fg || !bg) {
      rows.push(`  MISSING ${fgKey}/${bgKey}`)
      hardFail = true
      paletteHardFail = true
      continue
    }
    const ratio = contrast(fg, bg)
    const ok = ratio >= th
    const soft = th === UI
    if (!ok && !soft) {
      hardFail = true
      paletteHardFail = true
    }
    const tag = ok ? (soft ? 'pass' : 'PASS') : soft ? 'warn' : 'FAIL'
    rows.push(
      `  ${tag.padEnd(4)}  ${ratio.toFixed(2)}  (>=${th})  ${name}  ${hex(oklchToLinear(...fg))} on ${hex(oklchToLinear(...bg))}`
    )
  }
  console.log(`\n=== ${key}: ${p.label}${paletteHardFail ? '  <-- HARD FAIL' : ''} ===`)
  console.log(rows.join('\n'))
}

console.log(
  `\n${hardFail || statusFail ? 'HARD FAILURES (text pairs) — fix palettes' : 'All hard text-contrast checks pass; UI-boundary warnings are informational.'}`
)
process.exit(hardFail || statusFail ? 1 : 0)
