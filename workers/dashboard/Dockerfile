FROM node:18-alpine

WORKDIR /app

# Copy the real dashboard HTML from the rust_api service
COPY services/rust_api/static/dashboard.html /app/dashboard.html

# Create Node server that serves the actual dashboard HTML
RUN echo 'const http = require("http"); \
const fs = require("fs"); \
const dashboardHtml = fs.readFileSync("/app/dashboard.html", "utf-8"); \
const server = http.createServer((req, res) => { \
  if (req.url === "/health") { \
    res.writeHead(200); \
    res.end("OK"); \
  } else { \
    res.writeHead(200, { "Content-Type": "text/html" }); \
    res.end(dashboardHtml); \
  } \
}); \
server.listen(3000, () => console.log("Dashboard listening on 3000"));' > app.js

USER nobody

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --quiet --tries=1 --spider http://localhost:3000/health || exit 1

EXPOSE 3000

CMD ["node", "app.js"]
