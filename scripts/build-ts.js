#!/usr/bin/env node

const esbuild = require('esbuild');
const crypto = require('crypto');
const fs = require('fs');
const path = require('path');

const isWatch = process.argv.includes('--watch');
const srcDir = 'static/ts';
const outDir = 'static/js/dist';
const manifestPath = 'static/js/dist/manifest.json';

// Get all TypeScript entry points
function getEntryPoints() {
  const entries = [];
  const files = fs.readdirSync(srcDir);
  for (const file of files) {
    if (file.endsWith('.ts') && !file.endsWith('.d.ts')) {
      entries.push(path.join(srcDir, file));
    }
  }
  return entries;
}

// Generate hash from file contents
function hashContent(content) {
  return crypto.createHash('md5').update(content).digest('hex').slice(0, 8);
}

// Clean old hashed files
function cleanOldFiles() {
  if (!fs.existsSync(outDir)) {
    fs.mkdirSync(outDir, { recursive: true });
    return;
  }
  const files = fs.readdirSync(outDir);
  for (const file of files) {
    // Keep htmx.min.js and manifest.json, remove hashed files
    if (file.match(/\.[a-f0-9]{8}\.js$/)) {
      fs.unlinkSync(path.join(outDir, file));
    }
  }
}

// Build and generate manifest
async function build() {
  const entryPoints = getEntryPoints();

  if (entryPoints.length === 0) {
    console.log('No TypeScript files found in', srcDir);
    return;
  }

  cleanOldFiles();

  const manifest = {};

  for (const entry of entryPoints) {
    const baseName = path.basename(entry, '.ts');

    try {
      const result = await esbuild.build({
        entryPoints: [entry],
        bundle: true,
        minify: true,
        sourcemap: false,
        target: ['es2020'],
        format: 'iife',
        write: false,
      });

      const code = result.outputFiles[0].text;
      const hash = hashContent(code);
      const hashedName = `${baseName}.${hash}.js`;
      const outPath = path.join(outDir, hashedName);

      fs.writeFileSync(outPath, code);
      manifest[`${baseName}.js`] = hashedName;

      console.log(`Built: ${entry} -> ${hashedName}`);
    } catch (error) {
      console.error(`Error building ${entry}:`, error.message);
      process.exit(1);
    }
  }

  fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));
  console.log(`Manifest written to ${manifestPath}`);
}

// Watch mode
async function watch() {
  console.log('Watching for changes...');

  const entryPoints = getEntryPoints();

  if (entryPoints.length === 0) {
    console.log('No TypeScript files found in', srcDir);
    return;
  }

  // Initial build
  await build();

  // Watch for changes
  const chokidar = require('chokidar') || null;

  // Simple polling-based watch if chokidar not available
  let lastModified = {};

  const checkForChanges = async () => {
    const files = fs.readdirSync(srcDir);
    let changed = false;

    for (const file of files) {
      if (file.endsWith('.ts')) {
        const filePath = path.join(srcDir, file);
        const stat = fs.statSync(filePath);
        const mtime = stat.mtimeMs;

        if (!lastModified[file] || lastModified[file] < mtime) {
          lastModified[file] = mtime;
          changed = true;
        }
      }
    }

    if (changed) {
      console.log('\nChange detected, rebuilding...');
      await build();
    }
  };

  setInterval(checkForChanges, 1000);
}

// Main
if (isWatch) {
  watch().catch(console.error);
} else {
  build().catch(err => {
    console.error(err);
    process.exit(1);
  });
}
