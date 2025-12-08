# Manifesto Service - Setup Guide

## Prerequisites

- Rust (latest stable version)
- PostgreSQL 12+
- Access to the workspace root (for shared rustycog crates)

## Initial Setup

### 1. Database Setup

Create the database:

```bash
createdb manifesto_dev
createdb manifesto_test
```

Or using PostgreSQL CLI:

```sql
CREATE DATABASE manifesto_dev;
CREATE DATABASE manifesto_test;
```

### 2. Configuration

The service uses a layered configuration system:
- `config/default.toml` - Base configuration
- `config/development.toml` - Development overrides
- `config/test.toml` - Test environment
- Environment variables with `MANIFESTO_` prefix

Update `config/development.toml` with your database credentials if different from defaults.

### 3. Run Migrations

```bash
cd migration
cargo run -- up
```

To rollback migrations:

```bash
cargo run -- down
```

To check migration status:

```bash
cargo run -- status
```

## Database Schema

### Tables Created

The migrations create three tables:

#### 1. `projects`
- Core project information
- Ownership tracking (personal or organization)
- Status management (draft, active, archived, suspended)
- Visibility controls (private, internal, public)
- Timestamps and audit trail

#### 2. `project_components`
- Component attachments to projects
- Component type tracking
- Lifecycle status (pending, configured, active, disabled)
- Unique constraint: one component type per project

#### 3. `project_members`
- Project membership and access control
- Role-based permissions (owner, admin, write, read)
- Source tracking (direct, org_cascade, invitation, third_party_sync)
- Soft delete support with grace periods

### Indexes

The following indexes are created for optimal query performance:

**Projects:**
- `idx_projects_owner` - Composite index on (owner_type, owner_id)
- `idx_projects_status` - Index on status
- `idx_projects_created_by` - Index on created_by

**Project Components:**
- `idx_project_components_project` - Index on project_id
- `idx_project_components_status` - Composite index on (project_id, status)
- `project_components_unique` - Unique constraint on (project_id, component_type)

**Project Members:**
- `idx_project_members_project` - Index on project_id
- `idx_project_members_user` - Index on user_id
- `idx_project_members_active` - Composite index on (project_id, removed_at) for active members
- `project_members_unique` - Unique constraint on (project_id, user_id)

## Development Workflow

### Building the Project

```bash
cargo build
```

### Running the Service

```bash
cargo run
```

The service will start on `http://localhost:8080` by default.

### Running Tests

```bash
cargo test
```

## Configuration Options

### Business Logic Configuration

Key business configuration options in `config/default.toml`:

- `max_projects_per_user`: Maximum projects a user can own (default: 100)
- `max_projects_per_org`: Maximum projects per organization (default: 500)
- `max_members_per_project`: Maximum members per project (default: 100)
- `max_components_per_project`: Maximum components per project (default: 50)
- `member_removal_grace_period_days`: Grace period before final member deletion (default: 30 days)

### Feature Flags

Enable/disable features via configuration:

- `caching_enabled`: Enable response caching
- `audit_logging_enabled`: Enable detailed audit logs
- `event_publishing_enabled`: Enable event publishing to message queue
- `metrics_enabled`: Enable metrics collection

## Environment Variables

Override any configuration using environment variables:

```bash
export MANIFESTO_DATABASE_HOST=localhost
export MANIFESTO_DATABASE_PORT=5432
export MANIFESTO_DATABASE_NAME=manifesto_dev
export MANIFESTO_DATABASE_USERNAME=postgres
export MANIFESTO_DATABASE_PASSWORD=postgres
export MANIFESTO_SERVER_PORT=8080
export MANIFESTO_LOGGING_LEVEL=debug
```

## Next Steps

After initial setup, the following components need to be implemented:

1. **Domain Layer**: 
   - Project, ProjectComponent, ProjectMember entities
   - Value objects for status, visibility, role, etc.
   - Business rules and validations

2. **Application Layer**:
   - CRUD use cases for projects
   - Component management use cases
   - Member management use cases
   - DTOs and mappers

3. **Infrastructure Layer**:
   - Repository implementations
   - Database entity models (SeaORM)
   - Event publishers
   - External service integrations

4. **HTTP Layer**:
   - REST API endpoints
   - Request/response models
   - Error handling
   - Middleware

5. **Tests**:
   - Unit tests for domain logic
   - Integration tests for repositories
   - API tests for HTTP endpoints

## Troubleshooting

### Migration Errors

If migrations fail:

1. Check database connection:
   ```bash
   psql -h localhost -U postgres -d manifesto_dev
   ```

2. Verify configuration is loaded correctly
3. Check migration status:
   ```bash
   cd migration && cargo run -- status
   ```

### Configuration Issues

If the service can't load configuration:

1. Verify config files exist in `config/` directory
2. Check file permissions
3. Verify TOML syntax is valid
4. Check environment variables are properly prefixed with `MANIFESTO_`

## Architecture Notes

This service follows a hexagonal/clean architecture pattern:

- **Domain**: Pure business logic, framework-agnostic
- **Application**: Use cases orchestrating domain logic
- **Infrastructure**: Technical implementations (database, messaging, etc.)
- **HTTP**: Web API layer
- **Setup**: Application initialization and wiring

This separation allows for:
- Testability at each layer
- Independence from frameworks
- Flexibility to swap implementations
- Clear separation of concerns






