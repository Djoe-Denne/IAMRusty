{{/* Public invariants belong to the standard renderer, not a Python post-pass. */}}
{{- define "local-full.validate" -}}
{{- $object := .object -}}
{{- $root := .root -}}
{{- $identity := printf "%s/%s/%s" $object.kind (default "cluster" $object.metadata.namespace) $object.metadata.name -}}
{{- if hasKey .identities $identity -}}{{- fail "duplicate declared local-full resource" -}}{{- end -}}
{{- $_ := set .identities $identity true -}}
{{- $serialized := toJson $object -}}
{{- if or (contains "host.docker.internal" $serialized) (contains "oodhive-monolith" $serialized) -}}
{{- fail "host/hybrid dependency leaked into local-full render" -}}
{{- end -}}
{{- if .declaration.persistence -}}
{{- $contract := .declaration.persistence -}}
{{- $pod := $object.spec.template.spec -}}
{{- $volumes := dict -}}
{{- range $volume := $pod.volumes -}}
{{- if hasKey $volumes $volume.name -}}{{- fail "duplicate persistent volume" -}}{{- end -}}
{{- $_ := set $volumes $volume.name $volume -}}
{{- end -}}
{{- $expected := dict "name" $contract.volume "persistentVolumeClaim" (dict "claimName" $contract.claim) -}}
{{- if not (deepEqual (index $volumes $contract.volume) $expected) -}}
{{- fail (printf "local-full persistence patch missing: %s" $object.metadata.name) -}}
{{- end -}}
{{- $claimFound := false -}}
{{- range $candidate := $root.Values.resources -}}
{{- if $candidate -}}
{{- $claim := $candidate.object -}}
{{- if and (eq $claim.kind "PersistentVolumeClaim") (eq $claim.metadata.namespace $object.metadata.namespace) (eq $claim.metadata.name $contract.claim) -}}
{{- $claimFound = true -}}
{{- end -}}
{{- end -}}
{{- end -}}
{{- if not $claimFound -}}{{- fail "declared persistent claim missing" -}}{{- end -}}
{{- range $name := $contract.auxiliary -}}
{{- if not (hasKey $volumes $name) -}}{{- fail "persistent deployment lost a required auxiliary volume" -}}{{- end -}}
{{- end -}}
{{- range $container := concat (default (list) $pod.containers) (default (list) $pod.initContainers) -}}
{{- range $mount := $container.volumeMounts -}}
{{- if not (hasKey $volumes $mount.name) -}}{{- fail "persistent deployment lost a mounted volume" -}}{{- end -}}
{{- end -}}
{{- end -}}
{{- end -}}
{{- end -}}
