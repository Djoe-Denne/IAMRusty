#!/bin/bash
set -e

echo "Starting Lazaret service initialization..."

echo "Running database migrations..."
/app/migration up

echo "Migrations completed successfully!"

echo "Starting Lazaret service..."
cd /app
exec /app/lazaret-service
