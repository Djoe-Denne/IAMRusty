#!/bin/bash
set -e

echo "Starting Hive service initialization..."

echo "Running database migrations..."
/app/migration up

echo "Migrations completed successfully!"

echo "Starting Hive service..."
cd /app
exec /app/hive-service
