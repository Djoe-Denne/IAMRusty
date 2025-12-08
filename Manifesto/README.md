# Manifesto Service

The Manifesto Service manages projects and their components. Projects are assemblies of elementary bricks (components), where each component's behavior is handled by dedicated external services.

## 📊 Implementation Status

**Current Status:** ~95% Complete - Production-Ready MVP

For detailed implementation status, architecture decisions, and progress tracking, see:
- **[IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md)** - Complete implementation status and roadmap

## Overview

Manifesto provides CRUD operations for project entities and manages:
- **Projects**: Core project entities with ownership, status, and visibility controls
- **Project Components**: Modular components that can be attached to projects
- **Project Members**: User membership with resource-based permission control

## Database Schema

### Projects Table

The `projects` table stores core project information:
- **Identification**: UUID-based primary key with auto-generation
- **Basic Info**: Name, description
- **Status Management**: Draft, active, archived, or suspended states
- **Ownership**: Support for both personal and organization-owned projects
- **Settings**: Visibility controls, external collaboration flags, data classification
- **Timestamps**: Creation, updates, and publishing tracking

### Project Components Table

The `project_components` table manages component attachments:
- **Component Types**: Flexible typing system for various component types (taskboard, custom-form, analytics, etc.)
- **Status Tracking**: Tracks component lifecycle (pending → configured → active → disabled)
- **Timestamps**: Full audit trail of component state changes
- **Unique Constraint**: Ensures one component of each type per project

### Project Members Table

The `project_members` table handles access control:
- **Roles**: Owner, admin, write, read permissions
- **Source Tracking**: Direct assignment, organization cascade, invitations, third-party sync
- **Removal Management**: Soft deletes with grace periods and removal reasons
- **Activity Tracking**: Last access timestamps for analytics

## Architecture

This service follows the blueprint architecture pattern:
- **Domain**: Core business logic and entities
- **Application**: Use cases and application services
- **Infrastructure**: Database, messaging, and external service integrations
- **HTTP**: REST API endpoints
- **Migration**: Database schema management

## Component Philosophy

Projects in Manifesto are compositions of components. Each component type (like taskboard, custom-form, analytics) is managed by its own dedicated service. Manifesto orchestrates these components but doesn't implement their behavior.

This allows for:
- **Modularity**: Components can be developed and deployed independently
- **Scalability**: Component services can scale based on their specific needs
- **Flexibility**: New component types can be added without changing Manifesto

## Getting Started

### Running Migrations

```bash
cd Manifesto/migration
cargo run -- up
```

### Configuration

Configuration follows the rustycog-config pattern. Set up your database connection in:
- `config/default.toml` for base settings
- `config/development.toml` for local development
- Environment variables with `MANIFESTO_` prefix

Example database configuration:
```toml
[database]
host = "localhost"
port = 5432
username = "postgres"
password = "postgres"
name = "manifesto_dev"
```

## Project Ownership Models

### Personal Projects
- `owner_type` = "personal"
- `owner_id` = user UUID
- User has full control over the project

### Organization Projects
- `owner_type` = "organization"
- `owner_id` = organization UUID
- Access controlled through organization membership and project-specific roles

## Member Sources

Members can be added to projects through multiple sources:
- **Direct**: Manually added by project admins
- **Organization Cascade**: Automatically inherit from organization membership
- **Invitation**: Added via invitation system
- **Third-Party Sync**: Synchronized from external systems

## Status Workflows

### Project Status Flow
```
draft → active → archived
       ↓         ↑
     suspended --┘
```

### Component Status Flow
```
pending → configured → active → disabled
```

## Key Features

✅ **Fully Implemented:**
- Complete project lifecycle management (create, update, publish, archive, delete)
- Modular component system with dynamic resource management
- Resource-based permission system with granular access control
- Clean architecture with proper layer separation
- 20 REST API endpoints with JWT authentication
- Database migrations (8 tables) with permission system
- Event infrastructure ready for cross-service integration

📖 **Documentation:**
- **[IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md)** - Complete implementation status, TODOs, and roadmap (START HERE)
- **[SETUP.md](SETUP.md)** - Database setup, configuration, and troubleshooting
- **[IMPLEMENTATION_TEMPLATES.md](IMPLEMENTATION_TEMPLATES.md)** - Code pattern reference examples
- **[openspecs.yaml](openspecs.yaml)** - OpenAPI specification
- **[/docs/project/Archi.md](/docs/project/Archi.md)** - Overall system architecture (components, impersonation, cascading)
- **[archive/](archive/)** - Historical progress documentation (superseded)

## License

Workspace license applies.


