#!/usr/bin/env node
// Script to sync built-in assets from Rust codebase to npm package

import path from "path";
import { fileURLToPath } from "url";
import fs from "fs/promises";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const packageRoot = path.dirname(__dirname);
const repoRoot = path.dirname(packageRoot);

async function copyDirectory(src, dest, options = {}) {
  const { filter = () => true } = options;

  try {
    await fs.mkdir(dest, { recursive: true });

    const entries = await fs.readdir(src, { withFileTypes: true });

    for (const entry of entries) {
      const srcPath = path.join(src, entry.name);
      const destPath = path.join(dest, entry.name);

      if (!filter(entry.name, entry.isDirectory())) {
        continue;
      }

      if (entry.isDirectory()) {
        await copyDirectory(srcPath, destPath, options);
      } else {
        await fs.copyFile(srcPath, destPath);
      }
    }
  } catch (error) {
    console.error(`Error copying ${src} to ${dest}:`, error.message);
    throw error;
  }
}

async function syncAssets() {
  console.log("🔄 Syncing built-in assets from Rust codebase to npm package...");

  // Source directories in the Rust codebase
  const rustAssetsDir = path.join(repoRoot, "codex-rs", "assets", "builtin-prompts");
  const rustStylesDir = path.join(repoRoot, "codex-rs", "tui", "styles");

  // Destination directories in npm package
  const npmAssetsDir = path.join(packageRoot, "assets");
  const npmPromptsDir = path.join(npmAssetsDir, "builtin-prompts");
  const npmStylesDir = path.join(npmAssetsDir, "styles");

  try {
    // Clean existing assets
    try {
      await fs.rm(npmAssetsDir, { recursive: true, force: true });
    } catch (error) {
      // Directory might not exist, that's okay
    }

    // Copy built-in prompts
    console.log("📄 Copying built-in prompts...");
    await copyDirectory(rustAssetsDir, npmPromptsDir, {
      filter: (name, isDir) => {
        // Only copy .md files and directories
        return isDir || name.endsWith('.md');
      }
    });

    // Copy built-in styles
    console.log("🎨 Copying built-in styles...");
    await copyDirectory(rustStylesDir, npmStylesDir, {
      filter: (name, isDir) => {
        // Only copy .yaml files and ignore .md files
        return isDir || (name.endsWith('.yaml') || name.endsWith('.yml'));
      }
    });

    // Create a manifest file for debugging
    const manifest = {
      syncDate: new Date().toISOString(),
      source: {
        prompts: rustAssetsDir,
        styles: rustStylesDir
      },
      destination: {
        prompts: npmPromptsDir,
        styles: npmStylesDir
      }
    };

    await fs.writeFile(
      path.join(npmAssetsDir, "sync-manifest.json"),
      JSON.stringify(manifest, null, 2)
    );

    console.log("✅ Asset sync completed successfully!");

    // List what was copied for verification
    const promptsCount = await countFiles(npmPromptsDir, '.md');
    const stylesCount = await countFiles(npmStylesDir, '.yaml');

    console.log(`   📄 ${promptsCount} prompt files`);
    console.log(`   🎨 ${stylesCount} style files`);

  } catch (error) {
    console.error("❌ Asset sync failed:", error.message);
    process.exit(1);
  }
}

async function countFiles(dir, ext) {
  try {
    const entries = await fs.readdir(dir, { recursive: true });
    return entries.filter(entry => entry.endsWith(ext)).length;
  } catch (error) {
    return 0;
  }
}

// Run if called directly
if (process.argv[1] === __filename) {
  syncAssets().catch(error => {
    console.error("❌ Script failed:", error);
    process.exit(1);
  });
}

export { syncAssets };