"use strict";

const { platform } = process;

if (platform !== "win32") {
  console.error(
    "npm run build (production installer) is supported only on Windows.\n" +
      "  • UI + backend dev:  npm run dev  (Linux, macOS, Windows)\n" +
      "  • Windows .exe/.msi from Linux/macOS:  GitHub Actions → Build Windows installers",
  );
  process.exit(1);
}
