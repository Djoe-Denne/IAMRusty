#!/bin/bash

# AIForAll Services Management Script

set -e

COMPOSE_FILE="docker-compose.yml"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE}  AIForAll Services Manager${NC}"
    echo -e "${BLUE}================================${NC}"
}

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

show_help() {
    print_header
    echo ""
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  start     Start all services"
    echo "  stop      Stop all services"
    echo "  restart   Restart all services"
    echo "  logs      Show logs for all services"
    echo "  status    Show status of all services"
    echo "  clean     Stop and remove all containers, networks, and volumes"
    echo "  build     Build all Docker images"
    echo "  health    Check health of all services"
    echo "  reset-db  Truncate all database tables"
    echo "  help      Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 start              # Start all services"
    echo "  $0 logs iam-service   # Show logs for IAM service only"
    echo "  $0 logs -f            # Follow all logs"
}

check_requirements() {
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed or not in PATH"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null; then
        print_error "Docker Compose is not installed or not in PATH"
        exit 1
    fi

    if [ ! -f "$COMPOSE_FILE" ]; then
        print_error "docker-compose.yml not found in current directory"
        exit 1
    fi
}

start_services() {
    print_status "Starting all services..."
    docker-compose up -d
    print_status "Services started successfully!"
    echo ""
    print_status "Service URLs:"
    echo "  - IAMRusty:   http://localhost:8080 (HTTP), https://localhost:8443 (HTTPS)"
    echo "  - Telegraph:  http://localhost:8081"
    echo "  - PostgreSQL: localhost:5432"
    echo "  - LocalStack: http://localhost:4566"
}

stop_services() {
    print_status "Stopping all services..."
    docker-compose down
    print_status "Services stopped successfully!"
}

restart_services() {
    print_status "Restarting all services..."
    docker-compose restart
    print_status "Services restarted successfully!"
}

show_logs() {
    if [ $# -eq 0 ]; then
        docker-compose logs
    else
        docker-compose logs "$@"
    fi
}

show_status() {
    print_status "Service status:"
    docker-compose ps
}

clean_all() {
    print_warning "This will remove all containers, networks, and volumes!"
    read -p "Are you sure? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_status "Cleaning up..."
        docker-compose down -v --remove-orphans
        docker system prune -f
        print_status "Cleanup completed!"
    else
        print_status "Cleanup cancelled."
    fi
}

build_images() {
    print_status "Building Docker images..."
    docker-compose build --no-cache
    print_status "Images built successfully!"
}

check_health() {
    print_status "Checking service health..."
    
    services=("postgres" "localstack" "iam-service" "telegraph-service")
    
    for service in "${services[@]}"; do
        status=$(docker-compose ps -q "$service" | xargs docker inspect --format='{{.State.Health.Status}}' 2>/dev/null || echo "not running")
        if [ "$status" = "healthy" ]; then
            echo -e "  ${service}: ${GREEN}healthy${NC}"
        elif [ "$status" = "unhealthy" ]; then
            echo -e "  ${service}: ${RED}unhealthy${NC}"
        elif [ "$status" = "starting" ]; then
            echo -e "  ${service}: ${YELLOW}starting${NC}"
        else
            echo -e "  ${service}: ${RED}not running${NC}"
        fi
    done
}

reset_database() {
    print_warning "This will truncate all database tables!"
    read -p "Are you sure? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_status "Resetting database..."
        docker-compose --profile tools run --rm truncate-db
        print_status "Database reset completed!"
    else
        print_status "Database reset cancelled."
    fi
}

main() {
    check_requirements
    
    case "${1:-help}" in
        start)
            start_services
            ;;
        stop)
            stop_services
            ;;
        restart)
            restart_services
            ;;
        logs)
            shift
            show_logs "$@"
            ;;
        status)
            show_status
            ;;
        clean)
            clean_all
            ;;
        build)
            build_images
            ;;
        health)
            check_health
            ;;
        reset-db)
            reset_database
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            print_error "Unknown command: $1"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

main "$@" 