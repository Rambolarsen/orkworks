#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

escape_for_json() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\t'/\\t}"
  printf '%s' "$s"
}

doc_output=""
doc_status=0
status_already_reported=0

if [ "${ORKWORKS_DOC_CHECK_OUTPUT+x}" = "x" ]; then
  doc_output="${ORKWORKS_DOC_CHECK_OUTPUT}"
  raw_doc_status="${ORKWORKS_DOC_CHECK_EXIT_CODE:-0}"
  case "${raw_doc_status}" in
    ''|*[!0-9]*)
      doc_status=1
      status_already_reported=1
      if [ -n "${doc_output}" ]; then
        doc_output="[doc-check] Hook failed with invalid exit code ${raw_doc_status}."$'\n'"${doc_output}"
      else
        doc_output="[doc-check] Hook failed with invalid exit code ${raw_doc_status}."
      fi
      ;;
    *)
      doc_status="${raw_doc_status}"
      ;;
  esac
else
  set +e
  doc_output="$(bash "${SCRIPT_DIR}/doc-check.sh" 2>&1)"
  doc_status=$?
  set -e
fi

if ! [[ "${doc_status}" =~ ^[0-9]+$ ]]; then
  doc_status=1
fi

if [ "${doc_status}" -ne 0 ] && [ "${status_already_reported}" -eq 0 ]; then
  if [ -n "${doc_output}" ]; then
    doc_output="[doc-check] Hook failed with exit ${doc_status}."$'\n'"${doc_output}"
  else
    doc_output="[doc-check] Hook failed with exit ${doc_status}."
  fi
fi

if [ -z "${doc_output}" ]; then
  printf '{}\n'
  exit 0
fi

escaped_output="$(escape_for_json "${doc_output}")"
printf '{\n  "systemMessage": "%s"\n}\n' "${escaped_output}"
