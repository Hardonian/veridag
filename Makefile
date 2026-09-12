# Convenience wrapper around just(1). Requires just on PATH.
.PHONY: setup check vectors formal demo devnet sim health cli site-dev site-build ts-test py-test
setup:      ; just setup
check:      ; just check
vectors:    ; just vectors
formal:     ; just formal
demo:       ; just demo
devnet:     ; just devnet
sim:        ; just sim
health:     ; just health
cli:        ; just cli
site-dev:   ; just site-dev
site-build: ; just site-build
ts-test:    ; just ts-test
py-test:    ; just py-test

