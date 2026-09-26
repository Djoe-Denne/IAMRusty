#!/bin/bash

# LocalStack initialization script
# This script runs when LocalStack starts up to create necessary AWS resources

echo "🚀 Initializing LocalStack resources..."

# Wait for LocalStack to be ready
awslocal sqs list-queues || echo "Waiting for SQS service..."

# Create the user-events queue
echo "📋 Creating user-events queue..."
awslocal sqs create-queue --queue-name user-events

echo "📋 Creating lazaret-kv-events queue..."
awslocal sqs create-queue --queue-name lazaret-kv-events

echo "📋 Creating sentinel-sync-events queue..."
awslocal sqs create-queue --queue-name sentinel-sync-events

# Verify queue was created
echo "✅ Verifying queue creation..."
awslocal sqs list-queues

echo "🎉 LocalStack initialization completed!" 