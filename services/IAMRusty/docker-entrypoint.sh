#!/bin/bash
set -e

echo "Starting IAM service initialization..."

echo "Running database migrations..."
/app/migration up

echo "Migrations completed successfully!"

echo "Starting IAM service..."
cd /app
exec /app/iam-service 