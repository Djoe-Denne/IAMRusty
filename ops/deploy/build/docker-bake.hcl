// Offline from the repository root:
// docker buildx bake -f ops/deploy/build/targets.json -f ops/deploy/build/docker-bake.hcl --print
// The inventory is a Bake JSON variable, consumed natively by the build graph.
variable "IMAGE_TARGETS" {
  // One shared producer for the complete application-image set, no monolith.
  default = join(",", inventory.groups.workspace)
}
variable "CARGO_PROFILE" { default = "dev-image" }
variable "SOURCE_REVISION" { default = "unattested" }
variable "SDK_REVISION" { default = "unattested" }
variable "ARTIFACTS_REF" { default = "local/build-artifacts:bake" }
variable "BUILD_TAG" { default = "" }

function "cargo_features" {
  params = [names]
  result = sort(distinct(flatten([for name in names : inventory.targets[name].features])))
}

function "cargo_build_args" {
  params = [names]
  result = join(" ", concat(
    flatten([for name in names : [for package, binary in inventory.targets[name].outputs : ["-p", package, "--bin", binary]]]),
    length(cargo_features(names)) > 0 ? ["--features", join(",", cargo_features(names))] : []
  ))
}

group "default" { targets = inventory.groups["local-full"] }
group "local-full" { targets = inventory.groups["local-full"] }
group "workspace" { targets = inventory.groups.workspace }
group "openbao" { targets = inventory.groups.openbao }
group "kind-local-images" { targets = inventory.groups.workspace }

target "artifacts" {
  context = "."
  dockerfile = "Dockerfile.build"
  platforms = ["linux/amd64"]
  tags = [ARTIFACTS_REF]
  output = ["type=docker"]
  attest = ["type=provenance,disabled=true"]
  args = {
    DELIVERY_PROFILE = "images"
    IMAGE_TARGETS = IMAGE_TARGETS
    CARGO_BUILD_ARGS = cargo_build_args(split(",", IMAGE_TARGETS))
    CARGO_PROFILE = CARGO_PROFILE
    SOURCE_REVISION = SOURCE_REVISION
    SDK_REVISION = SDK_REVISION
  }
}

target "images" {
  matrix = { item = [for name, value in inventory.targets : merge(value, {name = name})] }
  name = item.name
  context = "."
  dockerfile = item.dockerfile
  platforms = ["linux/amd64"]
  // Inventory references are local repository:tag names (no registry ports).
  tags = [BUILD_TAG != "" ? "${split(":", item.source)[0]}:${BUILD_TAG}" : item.source]
  contexts = { build-artifacts = "target:artifacts" }
  output = ["type=docker"]
  attest = ["type=provenance,disabled=true"]
  args = {
    BUILD_IMAGE = "build-artifacts"
    SOURCE_REVISION = SOURCE_REVISION
    SDK_REVISION = SDK_REVISION
  }
}
