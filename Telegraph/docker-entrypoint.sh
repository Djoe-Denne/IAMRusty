#!/bin/bash
set -e

echo "Starting Telegraph service initialization..."

echo "Running database migrations..."
/app/migration up

echo "Migrations completed successfully!"

echo "Starting Telegraph service..."
cd /app
exec /app/telegraph-service 