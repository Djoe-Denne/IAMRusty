"use strict";
const http = require("http");
const fs = require("fs");
const path = require("path");
const catalog = process.env.CATALOG_JSON || path.join(__dirname, "component-catalog.json");
const port = Number(process.env.CATALOG_PORT || 9000);
const token = process.env.CATALOG_TOKEN;
if (!token) {
  process.stderr.write("CATALOG_TOKEN is required\n");
  process.exit(1);
}
const body = fs.readFileSync(catalog);
function authorized(req) {
  const header = req.headers.authorization || "";
  return header === `Bearer ${token}`;
}
http
  .createServer((req, res) => {
    const urlPath = (req.url || "").split("?")[0];
    if (req.method === "GET" && (urlPath === "/api/components" || urlPath === "/api/components/")) {
      if (!authorized(req)) {
        res.writeHead(401, { "Content-Length": 0, Connection: "close" });
        res.end();
        return;
      }
      res.writeHead(200, {
        "Content-Type": "application/json; charset=utf-8",
        "Content-Length": body.length,
        Connection: "close",
      });
      res.end(body);
      return;
    }
    res.writeHead(404, { "Content-Length": 0, Connection: "close" });
    res.end();
  })
  .listen(port, "0.0.0.0", () => {
    process.stdout.write(`component catalog stub listening on 0.0.0.0:${port}/api/components\n`);
  });
