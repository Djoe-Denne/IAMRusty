 # Service Blueprint

This is a blueprint/template for creating new microservices that follow the same clean architecture pattern as IAMRusty and Telegraph services.

## Architecture Overview

The blueprint follows a **Clean Architecture** (also known as Hexagonal Architecture) pattern with clear separation of concerns across multiple layers:

```
service-blueprint/
├── domain/           # Core business logic (entities, services, ports)
├── application/      # Use cases and business orchestration
├── infra/           # Infrastructure implementations (repositories, adapters)
├── http/            # HTTP API layer (handlers, routing)
├── configuration/   # Service-specific configuration
├── setup/           # Application bootstrapping and dependency injection
├── migration/       # Database migrations
├── tests/           # Integration and end-to-end tests
├── config/          # Configuration files (TOML, environment-specific)
├── resources/       # Static resources (templates, etc.)
├── scripts/         # Build and deployment scripts
└── docs/            # Documentation
```

## Layer Responsibilities

### Domain Layer (`domain/`)
- **Entities**: Core business objects and value objects
- **Services**: Domain business logic and rules
- **Ports**: Interfaces (traits) for external dependencies
- **Error**: Domain-specific error types
- **No external dependencies** except for basic utilities

### Application Layer (`application/`)
- **Use Cases**: Application-specific business logic
- **Commands**: Command pattern implementations
- **DTOs**: Data Transfer Objects for API contracts
- **Depends only on**: Domain layer

### Infrastructure Layer (`infra/`)
- **Repository**: Database access implementations
- **Adapters**: External service integrations
- **Event**: Event handling and publishing
- **Implements**: Domain ports/interfaces

### HTTP Layer (`http/`)
- **Handlers**: HTTP request/response handling
- **Validation**: Input validation and sanitization
- **Error**: HTTP-specific error handling
- **Routes**: API endpoint definitions

### Configuration Layer (`configuration/`)
- **Service-specific configuration structures**
- **Environment-based configuration loading**
- **Integration with rustycog-config**

### Setup Layer (`setup/`)
- **Application bootstrapping**
- **Dependency injection setup**
- **Service initialization**

## Dependencies

The blueprint uses a shared set of dependencies managed through a workspace:

### Core Dependencies
- **Web Framework**: Axum for HTTP server
- **Database**: SeaORM for ORM, PostgreSQL for database
- **Async Runtime**: Tokio
- **Serialization**: Serde (JSON)
- **Logging**: Tracing
- **Error Handling**: thiserror, anyhow
- **Configuration**: config crate + rustycog-config
- **Events**: rustycog-events for event handling

### Testing Dependencies
- **Integration Testing**: axum-test, testcontainers
- **Mocking**: mockall
- **Test Utilities**: rstest, claims

## Creating a New Service

1. **Copy the blueprint directory**:
   ```bash
   # Unix/Linux/macOS
   cp -r service-blueprint my-new-service
   cd my-new-service
   
   # Windows (PowerShell)
   Copy-Item -Recurse service-blueprint my-new-service
   cd my-new-service
   ```

2. **Run the rename script**:
   ```bash
   # Unix/Linux/macOS
   chmod +x scripts/rename-service.sh
   ./scripts/rename-service.sh my-new-service "My new service description"
   
   # Windows (PowerShell) - Use Git Bash or WSL
   bash scripts/rename-service.sh my-new-service "My new service description"
   ```

3. **Customize the configuration**:
   - Update `config/default.toml` with service-specific settings
   - Modify `configuration/src/lib.rs` for custom config structures

4. **Define your domain**:
   - Add entities in `domain/src/entity/`
   - Define business logic in `domain/src/service/`
   - Create ports in `domain/src/port/`

5. **Implement use cases**:
   - Add use cases in `application/src/usecase/`
   - Create commands in `application/src/command/`
   - Define DTOs in `application/src/dto/`

6. **Add infrastructure**:
   - Implement repositories in `infra/src/repository/`
   - Add external service adapters in `infra/src/adapters/`

7. **Create HTTP endpoints**:
   - Add handlers in `http/src/handlers/`
   - Update routing in `http/src/lib.rs`

8. **Add database migrations**:
   - Create migration files in `migration/src/`

9. **Write tests**:
   - Add integration tests in `tests/`
   - Create test fixtures in `tests/fixtures/`

## Key Features

### Configuration Management
- Environment-based configuration (development, test, production)
- Configuration caching for performance
- Support for multiple configuration sources

### Event-Driven Architecture
- Integration with rustycog-events for event publishing/consuming
- SQS and Kafka support through rustycog infrastructure

### Database Management
- SeaORM integration with automatic migrations
- Repository pattern for data access
- Transaction support

### HTTP API
- RESTful API design
- JSON request/response handling
- Authentication and authorization support
- Comprehensive error handling

### Testing
- Unit tests for domain logic
- Integration tests with real databases
- HTTP endpoint testing
- Test fixtures and utilities

### Deployment
- Docker containerization
- Docker Compose for local development
- Environment-specific configuration
- Health checks and monitoring

## Naming Conventions

### File Naming
- Use snake_case for file names
- Use descriptive names that reflect the functionality

### Module Naming
- Domain entities: `user.rs`, `order.rs`
- Services: `user_service.rs`, `notification_service.rs`
- Repositories: `user_repository.rs`, `postgres_user_repository.rs`
- Handlers: `user_handler.rs`, `auth_handler.rs`

### Crate Naming
- Follow pattern: `{service-name}-{layer}`
- Example: `user-domain`, `user-application`, `user-infra`

## Integration with Shared Crates

The blueprint integrates with shared infrastructure crates:

- **rustycog-config**: Configuration management
- **rustycog-events**: Event handling
- **rustycog-http**: HTTP utilities and middleware
- **rustycog-db**: Database utilities
- **rustycog-testing**: Testing utilities

## Best Practices

### Domain Layer
- Keep domain logic pure and free of external dependencies
- Use value objects for data validation
- Define clear interfaces (ports) for external dependencies

### Application Layer
- Implement use cases as single-responsibility functions
- Use command pattern for complex operations
- Keep DTOs simple and focused

### Infrastructure Layer
- Implement domain ports faithfully
- Handle infrastructure-specific errors appropriately
- Use dependency injection for testability

### HTTP Layer
- Validate input at the boundary
- Transform domain errors to appropriate HTTP status codes
- Keep handlers thin and focused

### Testing
- Write unit tests for domain logic
- Use integration tests for complex workflows
- Mock external dependencies appropriately
- Use test fixtures for consistent test data

## Example Service Implementation

See the `examples/` directory for a complete example of a simple service implementation using this blueprint.

## Contributing

When updating the blueprint:
1. Ensure changes are backward compatible
2. Update this README with any new patterns
3. Add examples for new features
4. Update the rename script if needed

## License

This blueprint follows the same license as the parent project.