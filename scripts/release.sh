#!/bin/bash -eu
cd $(realpath $(dirname $0))/..

function usage() {
  echo "Usage: $0 -[wctsh]"
  echo
  echo "  -w: Push app"
  echo "  -c: Push compiler service"
  echo "  -t: Push telemetry service"
  echo "  -s: Skip bumping version"
  echo "  -h: Display this message"
}

BUMP_VERSION=1
PUSH_ALL=1
PUSH_APP=0
PUSH_COMPILER_SERVICE=0
PUSH_TELEMETRY_SERVICE=0
while getopts "wctsh" option; do
   case $option in
      w) PUSH_ALL=0; PUSH_APP=1;;
      c) PUSH_ALL=0; PUSH_COMPILER_SERVICE=1;;
      t) PUSH_ALL=0; PUSH_TELEMETRY_SERVICE=1;;
      s) BUMP_VERSION=0;;
      h) usage; exit;;
      \?) usage; exit;;
   esac
done

if [[ $BUMP_VERSION -eq 1 ]]; then
  cargo workspaces publish --all --no-individual-tags --force='*'
fi

if [[ $PUSH_ALL -eq 1 || $PUSH_COMPILER_SERVICE  -eq 1 ]]; then
  scripts/build-compiler-service-docker-image.sh
  scripts/deploy-compiler-service.sh
fi

if [[ $PUSH_ALL -eq 1 || $PUSH_TELEMETRY_SERVICE  -eq 1 ]]; then
  scripts/build-telemetry-service-docker-image.sh
  scripts/deploy-telemetry-service.sh
fi

if [[ $PUSH_ALL -eq 1 || $PUSH_APP -eq 1 ]]; then
  scripts/push.sh
fi
