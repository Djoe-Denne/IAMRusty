#!/bin/bash
set -e

echo "Starting IAM service initialization..."

# Set database URL from environment or config
export DATABASE_URL="postgresql://postgres:postgres@postgres:5432/iam_dev"

echo "Running database migrations..."
/app/migration up

echo "Migrations completed successfully!"

echo "Starting IAM service..."
cd /usr/src/IAMRusty
exec /app/iam-service 