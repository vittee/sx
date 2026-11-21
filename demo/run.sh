#!/bin/sh
# Records demo/sx.gif from demo/demo.tape against a throwaway sshd container. Needs Docker.
set -e
cd "$(dirname "$0")/.."
export MSYS_NO_PATHCONV=1
here=$(pwd -W 2>/dev/null || pwd)

mkdir -p demo/.keys
[ -f demo/.keys/id_ed25519 ] || ssh-keygen -q -t ed25519 -N '' -C demo -f demo/.keys/id_ed25519

docker network create sx-demo >/dev/null 2>&1 || true
docker build -q -f demo/server.Dockerfile -t sx-demo-server .
docker build -q -f demo/Dockerfile -t sx-demo-recorder .

docker rm -f server >/dev/null 2>&1 || true
docker run -d --rm --name server --hostname web1 --network sx-demo sx-demo-server >/dev/null
trap 'docker stop server >/dev/null 2>&1 || true' EXIT
sleep 2

docker run --rm --network sx-demo -v "$here/demo:/vhs" sx-demo-recorder "${1:-demo.tape}"
