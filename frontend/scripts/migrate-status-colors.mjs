// One-shot migration: raw Tailwind palette status colors -> semantic tokens.
// Usage: node scripts/migrate-status-colors.mjs [--write]
// Without --write it only reports what would change (dry run).
//
// Rules:
// - red -> destructive, green -> success, amber/yellow/orange -> warning,
//   teal/cyan/purple -> info, gray -> muted/muted-foreground/foreground/border
// - dark:-prefixed palette classes are the dark half of a light+dark pair;
//   semantic tokens adapt per theme, so those tokens are deleted entirely.
// - Unknown palette classes are reported, never silently skipped.
// - password-strength-meter.tsx keeps its red/orange/yellow/green scale
//   (discrete strength scale, like chart colors); only its grays migrate.

import { readFileSync, writeFileSync } from 'node:fs'
import { execSync } from 'node:child_process'
import path from 'node:path'

const WRITE = process.argv.includes('--write')
const ROOT = path.resolve(import.meta.dirname, '..')

const files = execSync('git ls-files "src/**/*.ts" "src/**/*.tsx"', {
  cwd: ROOT,
  encoding: 'utf8',
})
  .split('\n')
  .filter(Boolean)

const SOLID = {
  red: 'destructive',
  green: 'success',
  amber: 'warning',
  yellow: 'warning',
  orange: 'warning',
  teal: 'info',
  cyan: 'info',
  purple: 'info',
}

function mapToken(util, color, shade) {
  if (util === 'text') {
    const n = Number(shade)
    if (color === 'red' && n >= 400 && n <= 800) return 'text-destructive'
    if (color === 'green' && n >= 400 && n <= 900) return 'text-success'
    if (['amber', 'yellow'].includes(color) && n >= 200 && n <= 950) return 'text-warning'
    if (['teal', 'cyan'].includes(color) && n >= 400 && n <= 800) return 'text-info'
    if (color === 'purple' && n === 800) return 'text-info'
    if (color === 'orange' && [600, 800].includes(n)) return 'text-warning'
    if (color === 'gray') {
      if (n === 900) return 'text-foreground'
      if (n >= 400 && n <= 800) return 'text-muted-foreground'
    }
    return null
  }
  if (util === 'bg') {
    const n = Number(shade)
    if (color === 'gray' && n >= 50 && n <= 950) return 'bg-muted'
    if (n === 500 || n === 600 || n === 700) return `bg-${SOLID[color]}`
    if (n === 50 || n === 100 || (n >= 800 && n <= 950)) return `bg-${SOLID[color]}/10`
    return null
  }
  if (util === 'border' || util.startsWith('border-')) {
    const n = Number(shade)
    if (color === 'gray' && n === 200) return util === 'border' ? 'border-border' : `${util}-border`
    if (n === 200 || n === 300) return `${util}-${SOLID[color]}/20`
    if (n === 500) return `${util}-${SOLID[color]}`
    return null
  }
  return null
}

const TOKEN_RE =
  /((?:[a-zA-Z0-9-]+:)*)(!?)((?:border-t|border-b|border-l|border-r|border-x|border-y|border|bg|text|ring|divide-x|divide-y|fill|stroke))-(red|green|amber|yellow|orange|teal|cyan|purple|gray)-(\d{2,3})(?:\/(\d{1,3}))?/g

const UNMAPPED = []
let totalReplaced = 0
let totalDeleted = 0
const touched = []

for (const rel of files) {
  const file = path.join(ROOT, rel)
  let src = readFileSync(file, 'utf8')
  let replaced = 0
  let deleted = 0

  const out = src.replace(TOKEN_RE, (full, variants, bang, util, color, shade, opacity, offset) => {
    // password-strength-meter keeps its discrete strength scale colors
    if (rel.endsWith('password-strength-meter.tsx') && color !== 'gray') return full

    const mapped = mapToken(util, color, shade)
    if (mapped === null) {
      const line = src.slice(0, offset).split('\n').length
      UNMAPPED.push(`${rel}:${line}  ${full}`)
      return full
    }
    if (variants.includes('dark:')) {
      deleted++
      // swallow one following space to keep className tidy
      return ''
    }
    replaced++
    const keepOpacity = !mapped.includes('/') && opacity
    return `${variants}${bang}${mapped}${keepOpacity ? `/${opacity}` : ''}`
  })

  if (replaced || deleted) {
    // Deletions may leave harmless double spaces inside className strings —
    // browsers split classes on any whitespace, so no cleanup is applied.
    if (WRITE) writeFileSync(file, out)
    touched.push(`${rel}  (-${deleted} dark, ~${replaced} mapped)`)
    totalReplaced += replaced
    totalDeleted += deleted
  }
}

console.log(`files touched: ${touched.length}`)
console.log(touched.join('\n'))
console.log(`\nmapped: ${totalReplaced}, dark-pairs deleted: ${totalDeleted}`)
if (UNMAPPED.length) {
  console.log(`\nUNMAPPED (need manual review): ${UNMAPPED.length}`)
  console.log(UNMAPPED.join('\n'))
  process.exitCode = 1
}
if (!WRITE) console.log('\n(dry run — pass --write to apply)')
