"use strict";

const fs = require("fs");
const path = require("path");

const root = path.join(__dirname, "..");
const webRoot = path.join(root, "web-root");
const srcHtml = path.join(root, "index.html");
const srcAssets = path.join(root, "assets");
const dstHtml = path.join(webRoot, "index.html");
const dstAssets = path.join(webRoot, "assets");

if (!fs.existsSync(srcHtml)) {
  console.error("sync-web-root: missing", srcHtml);
  process.exit(1);
}
if (!fs.existsSync(srcAssets) || !fs.statSync(srcAssets).isDirectory()) {
  console.error("sync-web-root: missing or not a directory:", srcAssets);
  process.exit(1);
}

fs.mkdirSync(webRoot, { recursive: true });
fs.copyFileSync(srcHtml, dstHtml);

if (fs.existsSync(dstAssets)) {
  const st = fs.statSync(dstAssets);
  if (st.isFile()) {
    fs.unlinkSync(dstAssets);
  } else if (st.isDirectory()) {
    fs.rmSync(dstAssets, { recursive: true, force: true });
  }
}

fs.cpSync(srcAssets, dstAssets, { recursive: true });
console.log("sync-web-root: wrote web-root/index.html and web-root/assets/");
