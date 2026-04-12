FROM node:18-alpine

WORKDIR /app

# Create minimal React-like health app
RUN echo 'const http = require("http"); \
const server = http.createServer((req, res) => { \
  if (req.url === "/health") { \
    res.writeHead(200); \
    res.end("OK"); \
  } else { \
    res.writeHead(200, { "Content-Type": "text/html" }); \
    res.end("<h1>Dashboard</h1><p>Coming soon...</p>"); \
  } \
}); \
server.listen(3000, () => console.log("Dashboard listening on 3000"));' > app.js

USER nobody

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --quiet --tries=1 --spider http://localhost:3000/health || exit 1

EXPOSE 3000

CMD ["node", "app.js"]
