#!/bin/sh

set -e

NAME=dino-park-fossil
DOCKER_REGISTRY=320464205386.dkr.ecr.us-west-2.amazonaws.com
REV=${REV:-latest}

compile_release() {
  cargo build --release
}

docker_build_local() {
  docker build -t ${DOCKER_REGISTRY}/${NAME}:${REV} -f Dockerfile.local .
}

docker_build() {
  docker build -t ${DOCKER_REGISTRY}/${NAME}:${REV} -f Dockerfile .
}

package_local() {
  compile_release
  docker_build_local
}

push_image() {
  docker push ${DOCKER_REGISTRY}/${NAME}:${REV}
}

deploy() {
  if [ -z ${DEPLOY_ENV} ]; then exit 1; fi
  helm template -f k8s/values.yaml -f k8s/values/${DEPLOY_ENV}.yaml \
    --set docker_registry=${DOCKER_REGISTRY},rev=${REV} k8s/ | kubectl apply -f -
}

if [ -z ${1} ]
then
  echo "usage: $0 [command]"
  echo
  echo "commands:"
  declare -F | sed  "s/declare -f /\t/g"
else 
  $1 
fi
