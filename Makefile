# Convenience wrapper around just(1). Requires just on PATH.
.PHONY: setup check audit vectors formal demo devnet sim health daemon docker-up docker-down bench cli site-dev site-build ts-test py-test release-gate
setup:        ; just setup
check:        ; just check
audit:        ; just audit
vectors:      ; just vectors
formal:       ; just formal
demo:         ; just demo
devnet:       ; just devnet
sim:          ; just sim
health:       ; just health
daemon:       ; just daemon
docker-up:    ; just docker-up
docker-down:  ; just docker-down
bench:        ; just bench
cli:          ; just cli
site-dev:     ; just site-dev
site-build:   ; just site-build
ts-test:      ; just ts-test
py-test:      ; just py-test
release-gate: ; just release-gate
