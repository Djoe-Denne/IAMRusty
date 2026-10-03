-- Hive Database Schema
-- Generated from SeaORM entities

-- Core Organizations
CREATE TABLE organizations (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    slug VARCHAR UNIQUE NOT NULL,
    description TEXT,
    avatar_url VARCHAR,
    owner_user_id UUID NOT NULL,
    settings JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Organization Roles
CREATE TABLE organization_roles (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL,
    name VARCHAR NOT NULL,
    description TEXT,
    is_system_default BOOLEAN NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE ON UPDATE CASCADE
);

-- Organization Members
CREATE TABLE organization_members (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL,
    user_id UUID NOT NULL,
    role_id UUID NOT NULL,
    status VARCHAR NOT NULL,
    invited_by_user_id UUID,
    invited_at TIMESTAMP WITH TIME ZONE,
    joined_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (role_id) REFERENCES organization_roles(id)
);

-- Organization Invitations
CREATE TABLE organization_invitations (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL,
    email VARCHAR NOT NULL,
    role_id UUID NOT NULL,
    invited_by_user_id UUID NOT NULL,
    token VARCHAR NOT NULL,
    status VARCHAR NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    accepted_at TIMESTAMP WITH TIME ZONE,
    message TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (role_id) REFERENCES organization_roles(id)
);

-- External Provider Types (GitHub, GitLab, etc.)
CREATE TABLE external_providers (
    id UUID PRIMARY KEY,
    provider_type VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    config_schema JSONB,
    is_active BOOLEAN NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Links between Organizations and External Providers
CREATE TABLE external_links (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL,
    provider_id UUID NOT NULL,
    provider_config JSONB NOT NULL,
    sync_enabled BOOLEAN NOT NULL,
    sync_settings JSONB NOT NULL,
    last_sync_at TIMESTAMP WITH TIME ZONE,
    last_sync_status VARCHAR,
    sync_error TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES external_providers(id)
);

-- Sync Jobs for External Data
CREATE TABLE sync_jobs (
    id UUID PRIMARY KEY,
    organization_external_link_id UUID NOT NULL,
    job_type VARCHAR NOT NULL,
    status VARCHAR NOT NULL,
    items_processed INTEGER NOT NULL,
    items_created INTEGER NOT NULL,
    items_updated INTEGER NOT NULL,
    items_failed INTEGER NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    details JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    FOREIGN KEY (organization_external_link_id) REFERENCES external_links(id) ON DELETE CASCADE ON UPDATE CASCADE
);

-- Permission System
CREATE TABLE permissions (
    id UUID PRIMARY KEY,
    level VARCHAR NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Resources that can be accessed
CREATE TABLE resources (
    id UUID PRIMARY KEY,
    resource_type VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Role-Permission-Resource Junction Table
CREATE TABLE role_permissions (
    id UUID PRIMARY KEY,
    organization_role_id UUID NOT NULL,
    permission_id UUID NOT NULL,
    resource_id UUID NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    FOREIGN KEY (organization_role_id) REFERENCES organization_roles(id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE ON UPDATE CASCADE
);

-- Useful indexes for performance
CREATE INDEX idx_organizations_slug ON organizations(slug);
CREATE INDEX idx_organization_members_org_user ON organization_members(organization_id, user_id);
CREATE INDEX idx_organization_invitations_email ON organization_invitations(email);
CREATE INDEX idx_external_links_org_provider ON external_links(organization_id, provider_id);
CREATE INDEX idx_sync_jobs_link_status ON sync_jobs(organization_external_link_id, status);
CREATE INDEX idx_role_permissions_role ON role_permissions(organization_role_id); 