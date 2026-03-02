{{/*
Expand the name of the chart.
*/}}
{{- define "raps.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "raps.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "raps.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels.
*/}}
{{- define "raps.labels" -}}
helm.sh/chart: {{ include "raps.chart" . }}
{{ include "raps.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels.
*/}}
{{- define "raps.selectorLabels" -}}
app.kubernetes.io/name: {{ include "raps.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Component-specific labels.
*/}}
{{- define "raps.componentLabels" -}}
{{ include "raps.labels" . }}
app.kubernetes.io/component: {{ .component }}
{{- end }}

{{/*
Component-specific selector labels.
*/}}
{{- define "raps.componentSelectorLabels" -}}
{{ include "raps.selectorLabels" . }}
app.kubernetes.io/component: {{ .component }}
{{- end }}

{{/*
Redis URL.
*/}}
{{- define "raps.redisUrl" -}}
{{- if .Values.messageBus.redis.external.enabled }}
{{- .Values.messageBus.redis.external.url }}
{{- else }}
{{- printf "redis://%s-redis-master:6379" .Release.Name }}
{{- end }}
{{- end }}

{{/*
Image for a component. Usage: include "raps.image" (dict "image" .Values.proxy.image "global" .Values.global)
*/}}
{{- define "raps.image" -}}
{{- $registry := .image.registry | default .global.image.registry -}}
{{- $repository := .image.repository -}}
{{- $tag := .image.tag | default .global.image.tag -}}
{{- if $registry -}}
{{- printf "%s/%s:%s" $registry $repository $tag -}}
{{- else -}}
{{- printf "%s:%s" $repository $tag -}}
{{- end -}}
{{- end }}

{{/*
Namespace for the raps system.
*/}}
{{- define "raps.namespace" -}}
{{- default .Release.Namespace .Values.global.namespace }}
{{- end }}
