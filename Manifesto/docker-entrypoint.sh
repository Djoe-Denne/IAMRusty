#!/bin/bash
set -e

echo "Starting Manifesto service initialization..."

echo "Running database migrations..."
/app/migration up

echo "Migrations completed successfully!"

echo "Starting Manifesto service..."
cd /app
exec /app/manifesto-service
