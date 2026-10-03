#!/bin/bash

# Script to rename crates to unique names and update references

echo "Renaming IAMRusty crates..."

# Update IAMRusty configuration crate
sed -i 's/name = "configuration"/name = "iam-configuration"/' services/IAMRusty/configuration/Cargo.toml

# Update IAMRusty infra crate
sed -i 's/name = "infra"/name = "iam-infra"/' services/IAMRusty/infra/Cargo.toml

# Update IAMRusty http crate
sed -i 's/name = "http"/name = "iam-http"/' services/IAMRusty/http/Cargo.toml

# Update IAMRusty setup crate
sed -i 's/name = "setup"/name = "iam-setup"/' services/IAMRusty/setup/Cargo.toml

# Update IAMRusty migration crate
sed -i 's/name = "migration"/name = "iam-migration"/' services/IAMRusty/migration/Cargo.toml

echo "Renaming Telegraph crates..."

# Update Telegraph domain crate
sed -i 's/name = "domain"/name = "telegraph-domain"/' services/Telegraph/domain/Cargo.toml

# Update Telegraph application crate
sed -i 's/name = "application"/name = "telegraph-application"/' services/Telegraph/application/Cargo.toml

# Update Telegraph configuration crate
sed -i 's/name = "configuration"/name = "telegraph-configuration"/' services/Telegraph/configuration/Cargo.toml

# Update Telegraph infra crate
sed -i 's/name = "infra"/name = "telegraph-infra"/' services/Telegraph/infra/Cargo.toml

# Update Telegraph http crate
sed -i 's/name = "http"/name = "telegraph-http"/' services/Telegraph/http/Cargo.toml

# Update Telegraph setup crate
sed -i 's/name = "setup"/name = "telegraph-setup"/' services/Telegraph/setup/Cargo.toml

echo "Updating references in IAMRusty crates..."

# Update references in IAMRusty domain
sed -i 's/domain = { path = "..\/domain" }/iam-domain = { path = "..\/domain" }/' services/IAMRusty/*/Cargo.toml
sed -i 's/application = { path = "..\/application" }/iam-application = { path = "..\/application" }/' services/IAMRusty/*/Cargo.toml
sed -i 's/infra = { path = "..\/infra" }/iam-infra = { path = "..\/infra" }/' services/IAMRusty/*/Cargo.toml
sed -i 's/configuration = { path = "..\/configuration" }/iam-configuration = { path = "..\/configuration" }/' services/IAMRusty/*/Cargo.toml

echo "Updating references in Telegraph crates..."

# Update references in Telegraph crates
sed -i 's/domain = { path = "..\/domain" }/telegraph-domain = { path = "..\/domain" }/' services/Telegraph/*/Cargo.toml
sed -i 's/application = { path = "..\/application" }/telegraph-application = { path = "..\/application" }/' services/Telegraph/*/Cargo.toml
sed -i 's/infra = { path = "..\/infra" }/telegraph-infra = { path = "..\/infra" }/' services/Telegraph/*/Cargo.toml
sed -i 's/configuration = { path = "..\/configuration" }/telegraph-configuration = { path = "..\/configuration" }/' services/Telegraph/*/Cargo.toml

echo "Updating main service references..."

# Update Telegraph main service
sed -i 's/domain = { path = "domain" }/telegraph-domain = { path = "domain" }/' services/Telegraph/Cargo.toml
sed -i 's/application = { path = "application" }/telegraph-application = { path = "application" }/' services/Telegraph/Cargo.toml
sed -i 's/infra = { path = "infra" }/telegraph-infra = { path = "infra" }/' services/Telegraph/Cargo.toml
sed -i 's/http = { path = "http" }/telegraph-http = { path = "http" }/' services/Telegraph/Cargo.toml
sed -i 's/configuration = { path = "configuration" }/telegraph-configuration = { path = "configuration" }/' services/Telegraph/Cargo.toml
sed -i 's/setup = { path = "setup" }/telegraph-setup = { path = "setup" }/' services/Telegraph/Cargo.toml

echo "Crate renaming complete!" 