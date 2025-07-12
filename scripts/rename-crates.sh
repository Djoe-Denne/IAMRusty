#!/bin/bash

# Script to rename crates to unique names and update references

echo "Renaming IAMRusty crates..."

# Update IAMRusty configuration crate
sed -i 's/name = "configuration"/name = "iam-configuration"/' IAMRusty/configuration/Cargo.toml

# Update IAMRusty infra crate
sed -i 's/name = "infra"/name = "iam-infra"/' IAMRusty/infra/Cargo.toml

# Update IAMRusty http crate
sed -i 's/name = "http"/name = "iam-http"/' IAMRusty/http/Cargo.toml

# Update IAMRusty setup crate
sed -i 's/name = "setup"/name = "iam-setup"/' IAMRusty/setup/Cargo.toml

# Update IAMRusty migration crate
sed -i 's/name = "migration"/name = "iam-migration"/' IAMRusty/migration/Cargo.toml

echo "Renaming Telegraph crates..."

# Update Telegraph domain crate
sed -i 's/name = "domain"/name = "telegraph-domain"/' Telegraph/domain/Cargo.toml

# Update Telegraph application crate
sed -i 's/name = "application"/name = "telegraph-application"/' Telegraph/application/Cargo.toml

# Update Telegraph configuration crate
sed -i 's/name = "configuration"/name = "telegraph-configuration"/' Telegraph/configuration/Cargo.toml

# Update Telegraph infra crate
sed -i 's/name = "infra"/name = "telegraph-infra"/' Telegraph/infra/Cargo.toml

# Update Telegraph http crate
sed -i 's/name = "http"/name = "telegraph-http"/' Telegraph/http/Cargo.toml

# Update Telegraph setup crate
sed -i 's/name = "setup"/name = "telegraph-setup"/' Telegraph/setup/Cargo.toml

echo "Updating references in IAMRusty crates..."

# Update references in IAMRusty domain
sed -i 's/domain = { path = "..\/domain" }/iam-domain = { path = "..\/domain" }/' IAMRusty/*/Cargo.toml
sed -i 's/application = { path = "..\/application" }/iam-application = { path = "..\/application" }/' IAMRusty/*/Cargo.toml
sed -i 's/infra = { path = "..\/infra" }/iam-infra = { path = "..\/infra" }/' IAMRusty/*/Cargo.toml
sed -i 's/configuration = { path = "..\/configuration" }/iam-configuration = { path = "..\/configuration" }/' IAMRusty/*/Cargo.toml

echo "Updating references in Telegraph crates..."

# Update references in Telegraph crates
sed -i 's/domain = { path = "..\/domain" }/telegraph-domain = { path = "..\/domain" }/' Telegraph/*/Cargo.toml
sed -i 's/application = { path = "..\/application" }/telegraph-application = { path = "..\/application" }/' Telegraph/*/Cargo.toml
sed -i 's/infra = { path = "..\/infra" }/telegraph-infra = { path = "..\/infra" }/' Telegraph/*/Cargo.toml
sed -i 's/configuration = { path = "..\/configuration" }/telegraph-configuration = { path = "..\/configuration" }/' Telegraph/*/Cargo.toml

echo "Updating main service references..."

# Update Telegraph main service
sed -i 's/domain = { path = "domain" }/telegraph-domain = { path = "domain" }/' Telegraph/Cargo.toml
sed -i 's/application = { path = "application" }/telegraph-application = { path = "application" }/' Telegraph/Cargo.toml
sed -i 's/infra = { path = "infra" }/telegraph-infra = { path = "infra" }/' Telegraph/Cargo.toml
sed -i 's/http = { path = "http" }/telegraph-http = { path = "http" }/' Telegraph/Cargo.toml
sed -i 's/configuration = { path = "configuration" }/telegraph-configuration = { path = "configuration" }/' Telegraph/Cargo.toml
sed -i 's/setup = { path = "setup" }/telegraph-setup = { path = "setup" }/' Telegraph/Cargo.toml

echo "Crate renaming complete!" 