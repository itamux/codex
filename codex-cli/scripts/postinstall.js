#!/usr/bin/env node
// Post-install script for @itamux/codex npm package

import path from "path";
import { fileURLToPath } from "url";
import fs from "fs/promises";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const packageRoot = path.dirname(__dirname);

async function main() {
  try {
    // Verify that assets directory exists
    const assetsDir = path.join(packageRoot, "assets");
    try {
      await fs.access(assetsDir);
      console.log("✅ Built-in prompts and styles are available");
    } catch (error) {
      console.warn("⚠️  Built-in assets directory not found. Some internal commands may not be available.");
    }

    // Verify that at least one binary exists
    const binDir = path.join(packageRoot, "bin");
    const binFiles = await fs.readdir(binDir);
    const binaryExists = binFiles.some(file => file.startsWith("codex-"));

    if (binaryExists) {
      console.log("✅ Codex binary is ready");
    } else {
      console.warn("⚠️  No Codex binary found in bin directory");
    }

    console.log("🚀 @itamux/codex installation complete!");
    console.log("   Run 'codex' to get started");
  } catch (error) {
    console.error("❌ Post-install check failed:", error.message);
    // Don't fail the installation for verification issues
    process.exit(0);
  }
}

main().catch((error) => {
  console.error("❌ Post-install script error:", error);
  process.exit(0);
});