"use strict";

const { execSync } = require("child_process");
const { platform } = process;

if (platform !== "win32") {
  console.error(
    "npm run build (production installer) is supported only on Windows.\n" +
      "  • UI + backend dev:  npm run dev  (Linux, macOS, Windows)\n" +
      "  • Windows .exe/.msi from Linux/macOS:  GitHub Actions → Build Windows installers",
  );
  process.exit(1);
}

/* Cargo cannot overwrite src-tauri/target/release/app.exe if it is running or locked (Cursor preview, AV). */
try {
  execSync("taskkill /IM app.exe /F /T", {
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });
  console.warn(
    "Stopped running app.exe so the release build can replace target/release/app.exe.",
  );
} catch {
  /* No process named app.exe — ignore */
}
