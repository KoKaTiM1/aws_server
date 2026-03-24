const { Pool } = require('pg');

// PostgreSQL connection pool with SSL for RDS
const pool = new Pool({
  host: process.env.DB_HOST,
  port: parseInt(process.env.DB_PORT) || 5432,
  database: process.env.DB_NAME || 'eyedar',
  user: process.env.DB_USERNAME,
  password: process.env.DB_PASSWORD,
  max: 20,
  idleTimeoutMillis: 30000,
  connectionTimeoutMillis: 5000,
  ssl: {
    rejectUnauthorized: false, // RDS uses self-signed certificates
  },
});

// Test connection on startup
pool.on('connect', () => {
  console.log('✅ Database pool connected');
});

pool.on('error', (err) => {
  console.error('❌ Unexpected database error:', err);
  process.exit(-1);
});

module.exports = pool;
