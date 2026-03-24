FROM node:18-alpine

WORKDIR /app

# Install curl for ECS health checks
RUN apk add --no-cache curl

# Install dependencies
COPY package*.json ./
RUN npm install --production

# Copy source code
COPY src/ ./src/

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD node -e "require('http').get('http://localhost:8080/health', (r) => { process.exit(r.statusCode === 200 ? 0 : 1); });"

# Expose port
EXPOSE 8080

# Run as non-root user for security
USER node

CMD ["node", "src/index.js"]
